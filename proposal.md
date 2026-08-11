# Dependency Injection: making `rok-api-starter` NestJS-shaped

## Context

The project currently mixes three different, incompatible ways of wiring dependencies:

1. **A hidden global singleton.** `crates/api-core/src/db.rs` exposes `static POOL: OnceLock<PgPool>` via `db::pool()`. The generic `CrudService` trait (`crates/api-core/src/crud.rs`) requires implementors to provide `fn pool() -> &'static PgPool`, and `src/app/models/user.rs` satisfies it by calling the global. Nothing about this is injectable, mockable, or visible in any type signature.
2. **Free-function "services".** `src/app/services/user_service.rs` and RBAC handling are plain `pub async fn` modules that call `User::` static methods directly. There's no interface to swap an implementation behind, so unit-testing a controller without a live Postgres connection is impossible today.
3. **A hand-rolled trait-context pattern in `crates/auth`.** `AuthContext` (`crates/auth/src/context.rs`) is a single large trait exposing `pool()`, `config()`, `mailer() -> &dyn MailSender`, `user_finder() -> &dyn UserFinder`, `permission_finder() -> &dyn PermissionFinder`. Every auth service/handler is generic over `<C: AuthContext>`, and `src/state.rs`'s `AppState` hand-implements this trait plus axum's `FromRef` plus a `From<User>` conversion, including a `Box::leak` hack to fabricate a `'static AuthConfig` reference on every call — it's a God object serving as both the axum shared state and the DI context.

None of these give the ergonomics the user is after (constructor injection, modules, provider interfaces resolved by type), and (1)/(2) actively block testability. The user asked for a **NestJS-style DI system**, choosing the most ambitious of three options offered: a **custom proc-macro framework** (`#[injectable]`, `#[module]`) rather than adopting an existing crate (shaku/teloc) or hand-wiring `Arc`s without macros. This plan builds that framework as a new internal crate pair and migrates the root crate's data layer onto it, while making a deliberate, justified call to **coexist with** rather than rewrite `crates/auth`'s existing `AuthContext` pattern (rationale in §9 — it is itself a legitimate DI pattern, not one of the anti-patterns being fixed, and a full rewrite of 8 service + 8 handler modules is large, risky, and low-value for a starter kit).

## Design: the `di` framework

Two new workspace crates, mirroring the common `serde`/`serde_derive` split:

- **`crates/di`** — runtime: `Container`, `ContainerBuilder`, `Injectable`, `Injected<T>` (axum extractor), `DiError`. Re-exports the macros so callers only depend on `di`.
- **`crates/di-macros`** — proc-macro crate (`syn` 2, `quote`, `proc-macro2`) providing `#[injectable]` and `#[module]`.

**Rejected: `inventory`/`linkme`.** Those solve automatic, cross-crate discovery of annotated items via linker magic. This design uses explicit `providers = [...]` lists, mirroring Nest's explicit `@Module()` — there's no discovery problem to solve, and pulling in link-time-magic crates for a starter kit is over-engineering.

**Rejected: a global `OnceLock<Container>`.** That would just rebuild today's `OnceLock<PgPool>` anti-pattern one layer up. `Container` flows through `AppState` via axum's `FromRef`, like every other piece of state today.

**Keep `async-trait`** (already a workspace dependency) for any interface stored as `Arc<dyn Trait>` (e.g. `UserRepository`) — native `async fn` in traits still isn't `dyn`-compatible.

### Container

A single type-keyed map, keyed on the `TypeId` of the **stored `Arc<T>` itself** (not of `T`) — this is what's actually boxed as `Any`, so keying and downcasting agree by construction, and it works uniformly whether `T` is a concrete type or a trait object (`Arc<T>` is always `Sized` even when `T: ?Sized`):

```rust
// crates/di/src/container.rs
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct ContainerBuilder {
    providers: HashMap<TypeId, (Box<dyn Any + Send + Sync>, &'static str)>, // value, type_name for diagnostics
    registered_modules: HashSet<TypeId>, // diamond-import guard, see "Modules" below
}

#[derive(Clone)]
pub struct Container(Arc<HashMap<TypeId, (Box<dyn Any + Send + Sync>, &'static str)>>); // O(1) Clone

impl ContainerBuilder {
    pub fn new() -> Self { Self { providers: HashMap::new(), registered_modules: HashSet::new() } }

    /// Register a pre-built value under its own concrete type (config, PgPool, Mailer…).
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
        self.insert_arc(Arc::new(value))
    }

    /// Register an already-Arc'd value, possibly under a *trait* key (T = dyn Trait).
    pub fn insert_arc<T: ?Sized + Send + Sync + 'static>(&mut self, value: Arc<T>) -> &mut Self {
        self.providers.insert(TypeId::of::<Arc<T>>(), (Box::new(value), std::any::type_name::<T>()));
        self
    }

    /// Interface-binding sugar: builder.bind::<dyn UserRepository>().to_arc(arc_impl)
    pub fn bind<Trait: ?Sized + Send + Sync + 'static>(&mut self) -> Binder<'_, Trait> {
        Binder { builder: self, _t: std::marker::PhantomData }
    }

    pub fn get<T: ?Sized + 'static>(&self) -> Option<Arc<T>> {
        self.providers.get(&TypeId::of::<Arc<T>>())
            .and_then(|(b, _)| b.downcast_ref::<Arc<T>>())
            .cloned()
    }

    /// Returns false (and does nothing) if this module was already registered — see diamond imports.
    pub fn mark_registered<M: 'static>(&mut self) -> bool { self.registered_modules.insert(TypeId::of::<M>()) }

    pub fn build(self) -> Container { Container(Arc::new(self.providers)) }
}

pub struct Binder<'a, Trait: ?Sized> { builder: &'a mut ContainerBuilder, _t: std::marker::PhantomData<Trait> }
impl<'a, Trait: ?Sized + Send + Sync + 'static> Binder<'a, Trait> {
    pub fn to_arc(self, value: Arc<Trait>) { self.builder.insert_arc::<Trait>(value); }
}

impl Container {
    pub fn get<T: ?Sized + 'static>(&self) -> Option<Arc<T>> {
        self.0.get(&TypeId::of::<Arc<T>>()).and_then(|(b, _)| b.downcast_ref::<Arc<T>>()).cloned()
    }
}
```

Binding an interface must state the trait explicitly at the call site (`bind::<dyn UserRepository>()`, `get::<dyn UserRepository>()`) — inference can't recover it from context, which is expected and matches how Nest tokens work too.

### `#[injectable]`

A struct is eligible for **automatic** construction (usable bare inside `#[module(providers = [...])]`) only if *every* field is `#[inject]`. This is a deliberate simplification: teaching the macro to parse arbitrary constructor-argument literals inside `#[module(providers = [...])]` (e.g. `max_bytes = 5_242_880`) would require a small expression grammar for little benefit. Config values that need injecting become providers too (e.g. register `Arc<AppConfig>` in the container and `#[inject]` it like any other dependency — this mirrors Nest's `ConfigService` pattern). Structs that genuinely need a plain, non-container value are still macro-annotated (they get a generated `new()`) but are constructed by hand at the composition root and inserted via `builder.insert_arc(...)` — they're simply not listed in a module's `providers`.

```rust
#[injectable]
pub struct PgUserRepository {
    #[inject]
    pool: Arc<PgPool>,
}
```

expands to:

```rust
impl PgUserRepository {
    pub fn new(pool: Arc<PgPool>) -> Self { Self { pool } }
}

impl ::di::Injectable for PgUserRepository {
    fn construct(container: &::di::ContainerBuilder) -> Result<Arc<Self>, ::di::DiError> {
        let pool = container.get::<PgPool>()
            .ok_or_else(|| ::di::DiError::missing("PgUserRepository", "pool", std::any::type_name::<PgPool>()))?;
        Ok(Arc::new(Self::new(pool)))
    }
}
```

For a field typed `Arc<dyn UserRepository>`, the macro parses the `Arc<...>` wrapper via `syn` and substitutes the inner type (concrete or `dyn Trait`) into `container.get::<#inner>()` — one codegen path handles both cases uniformly.

**Macro validation (compile-time `compile_error!`, not runtime panics):**
- Only named-field structs are supported.
- Every `#[inject]` field's type must syntactically be `Arc<...>`.
- If a struct with non-`#[inject]` fields is referenced bare inside `#[module(providers = [...])]`, the generated call to `<T as Injectable>::construct(...)` fails to compile, because that type never implements `Injectable` — a compile error, not a startup or request-time failure.

### `#[module]`

```rust
#[module(
    providers = [PgUserRepository as dyn UserRepository, UserService],
)]
pub struct AppModule;
```

expands to roughly:

```rust
pub struct AppModule;
impl AppModule {
    pub fn register(builder: &mut ::di::ContainerBuilder) -> Result<(), ::di::DiError> {
        if !builder.mark_registered::<AppModule>() { return Ok(()); } // diamond-import guard

        let p0 = <PgUserRepository as ::di::Injectable>::construct(builder)?;
        builder.insert_arc::<PgUserRepository>(p0.clone());
        builder.bind::<dyn UserRepository>().to_arc(p0);

        let p1 = <UserService as ::di::Injectable>::construct(builder)?;
        builder.insert_arc::<UserService>(p1);

        Ok(())
    }
}
```

- **Ordering:** providers are constructed in **declared order**, no topological sort or cycle detection — out of budget for a proc-macro operating on one attribute invocation, and unnecessary for a project this size. This is documented as a requirement on `#[module]`. A genuine cycle is still caught automatically (whichever type constructs first fails to find the other), surfaced as `DiError::MissingDependency` with a hint in the message pointing at circular dependencies as a possible cause.
- **`imports`:** an `imports = [OtherModule]` entry expands to calling `OtherModule::register(builder)?` first. The `mark_registered::<M>()` guard means a module imported by two different parents is only ever constructed once (no duplicate/diamond construction).
- **`controllers`:** deliberately inert metadata in v1 (parsed — so a typo is a compile error — but generates nothing beyond a documentation-only const). Axum's handler model is free functions + extractors, not Nest's controller-class-per-route-group model; forcing handlers into DI-constructed controller structs would fight Axum's grain for no benefit, since constructor-injection ergonomics are already delivered per-handler via `Injected<T>`.

### Axum integration: `Injected<T>`

Built on top of axum's own `State<T>: FromRequestParts`, not a hand-rolled `Parts` walk:

```rust
// crates/di/src/extractor.rs
use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::request::Parts;
use std::sync::Arc;

pub struct Injected<T: ?Sized>(pub Arc<T>);

impl<S, T> FromRequestParts<S> for Injected<T>
where
    Container: FromRef<S>,
    S: Send + Sync,
    T: ?Sized + Send + Sync + 'static,
{
    type Rejection = DiRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let State(container) = State::<Container>::from_request_parts(parts, state)
            .await
            .expect("infallible: State<T> extraction cannot fail");
        container.get::<T>().map(Injected).ok_or_else(DiRejection::not_registered::<T>)
    }
}
```

Because construction is eager at startup (see below), `DiRejection` should never actually fire against a running server — the only way it fires is a handler-signature typo (`Injected<UserServic>`) that nothing else catches until that route is first hit. Mitigation: one integration test per route is enough to catch this class of bug in CI, same as any other Axum handler-signature mistake today.

`AppState` grows a `container: Container` field (plus a handful of fields cached from it for `auth` crate coexistence — see §9) and gets a `FromRef` impl for `Container`:

```rust
impl FromRef<AppState> for Container { fn from_ref(state: &AppState) -> Self { state.container.clone() } }
```

Controllers change from:

```rust
pub async fn index(_admin: AdminOnly) -> Result<ApiResponse, AppError> {
    let users = services::user_service::list().await?;
    ...
```

to:

```rust
pub async fn index(_admin: AdminOnly, Injected(users): Injected<UserService>) -> Result<ApiResponse, AppError> {
    let list = users.list().await?;
    ...
```

### Construction-time failures and fallible/async construction

`Injectable::construct` is **synchronous and infallible-to-call** (it only returns `Err` for a missing dependency, never for I/O). Anything expensive or fallible to build — `PgPool::connect`, `Mailer::new`'s SMTP transport setup — is built by hand in `main.rs` exactly as today, then handed to the builder as a pre-built leaf **before** any module registers:

```rust
let mut builder = ContainerBuilder::new();
builder.insert(pool);            // Arc<PgPool>
builder.insert(mailer);          // Arc<Mailer>
builder.insert(config.clone());  // Arc<AppConfig>
AppModule::register(&mut builder)?;  // constructs everything eagerly, in declared order
let container = builder.build();     // infallible — validation already happened above via `?`
```

Because construction is eager and sequential, construction *is* the validation: a missing/misordered dependency returns `Err` immediately from `register()`, and `main()` exits with a clear error before `axum::serve` ever runs. No lazy panics reach request handling.

```rust
#[derive(Debug, thiserror::Error)]
pub enum DiError {
    #[error("`{provider}` requires `{dep_type}` for field `{field}`, but nothing is registered for that type yet — check `#[module(providers=[...])]` declaration order, or that the providing module is imported. (If this is a circular dependency between two injectables, break the cycle.)")]
    MissingDependency { provider: &'static str, field: &'static str, dep_type: &'static str },
}
```

### Scope: singleton only

No request scope. Every existing shared dependency (`PgPool`, `AppConfig`, `Mailer`, and every new repository/service) is genuinely app-lifetime state; nothing in this codebase needs per-request DI-managed state beyond what ordinary Axum extractors (`Path`, `Json`, `AuthUser`) already provide from the request itself. `Injected<T>` is already just a synchronous hashmap lookup + `Arc::clone` per request. If per-request context is ever needed later (tenant id, request-scoped tracing span), that's an axum `Extension`/middleware concern, not a container-scope concern.

### Testing story

- **Unit tests** don't need the container at all: `#[injectable]` always emits a plain `fn new(...)`, so `UserService::new(Arc::new(MockUserRepository::default()))` is an ordinary constructor call. This is the primary, recommended path.
- **Integration tests** (router-level, `tests/api.rs`) build an alternate `ContainerBuilder`, bind a mock via `builder.bind::<dyn UserRepository>().to_arc(mock)` instead of running `AppModule::register`, and construct an `AppState` from it — same `Router<AppState>`, different provider graph. `tests/api.rs` is already stale today (it references a `rok_api_start::mail` module and a zero-arg `routes::app_router()` that no longer exist), so it needs rewriting regardless of this migration.

## Coexistence with `crates/auth`'s `AuthContext` (not a rewrite)

**Decision:** keep `AuthContext` and its `<C: AuthContext>`-generic services/handlers as-is. It's zero-cost static dispatch, it already works, and it's the largest, most security-sensitive surface in the repo (JWT, OTP, magic link, OAuth, 2FA, RBAC — 8 handler modules, 8 service modules). It is not one of the anti-patterns this plan targets — the free-function services and the global pool are. Rewriting all of `crates/auth` onto `Injected<T>` would be a large, risky diff disproportionate to the value for a starter kit, and is called out here as an explicit, deliberate scope decision rather than a silent omission — full migration of the `auth` crate's internals remains available as a later, optional phase if ever wanted.

The one real piece of friction: `AuthContext`'s accessors return **borrows** (`fn mailer(&self) -> &dyn MailSender`) tied to `&self`, while `Container::get::<T>()` returns an **owned** `Arc<T>` — you can't return a reference borrowed out of a temporary hashmap lookup. Fix: `AppState` caches a handful of fields **once at startup**, populated *from* the container, replacing today's per-call reconstruction and `Box::leak` hack:

```rust
#[derive(Clone)]
pub struct AppState {
    container: Container,                        // for NEW code, via Injected<T>
    pool: Arc<PgPool>,                            // cached from container, for AuthContext
    auth_config: Arc<auth::config::AuthConfig>,   // cached — removes the Box::leak hack
    mailer: Arc<dyn auth::context::MailSender>,
    user_finder: Arc<dyn auth::context::UserFinder>,
    permission_finder: Arc<dyn auth::context::PermissionFinder>,
}

impl auth::context::AuthContext for AppState {
    fn pool(&self) -> &PgPool { &self.pool }
    fn config(&self) -> &auth::config::AuthConfig { &self.auth_config }
    fn mailer(&self) -> &dyn auth::context::MailSender { self.mailer.as_ref() }
    fn user_finder(&self) -> &dyn auth::context::UserFinder { self.user_finder.as_ref() }
    fn permission_finder(&self) -> &dyn auth::context::PermissionFinder { self.permission_finder.as_ref() }
}
```

All five cached fields are populated by `container.get::<X>().unwrap()` once, immediately after `builder.build()` in `main.rs` — the container remains the single source of truth for construction; `AppState` is just a thin, cheaply-`Clone`-able view over it for the part of the codebase that needs borrow-shaped access. `UserFinder`/`PermissionFinder` become real container-registered providers (new `AppUserFinder`/`AppPermissionFinder` types in the root crate implementing those traits against `UserRepository`) instead of being implemented directly on `AppState` as today — net less hand-wiring than the current code, not more.

## Migration plan

1. **Build the framework in isolation.** `crates/di` + `crates/di-macros`, with the framework's own unit tests (container get/insert/bind, a toy `#[injectable]`/`#[module]` expansion, `Injected` against a bare test router). Nothing in the app changes yet.
2. **Kill the global pool.** Change `crates/api-core/src/crud.rs`'s `CrudService` methods from `Self::pool()`-based to parameterized (`&PgPool` argument). Delete `crates/api-core/src/db.rs`'s `OnceLock`. The only implementor today is `User`, so blast radius is small and mechanical, no proc-macro involvement yet.
3. **First vertical slice.** `src/app/repositories/user_repository.rs` — `UserRepository` trait + `#[injectable] PgUserRepository` wrapping `Arc<PgPool>`, delegating to the now-parameterized `CrudService` calls. First real use of the `di` crate against production code.
4. **Convert `user_service`.** `src/app/services/user_service.rs`'s free functions become `#[injectable] pub struct UserService { #[inject] users: Arc<dyn UserRepository> }` with `&self` async methods. `upload_avatar` needs storage config — register `Arc<AppConfig>` (or a slimmer storage-config type) as a provider and `#[inject]` it, same as any other dependency. Extract `src/storage.rs`'s filesystem functions behind a small `AvatarStorage` trait for the same reason.
5. **Convert `user_controller`.** Swap `services::user_service::xyz()` + `State<AppState>` for `Injected<UserService>` params.
6. **Composition root, done last (highest blast radius).** In `src/main.rs`/`src/state.rs`: build a `ContainerBuilder`, `insert` the pre-built `PgPool`/`Mailer`/`AppConfig`, define `#[module] AppModule`, call `AppModule::register`, `.build()`, then cache the five `AuthContext`-required fields onto the slimmed `AppState` as described above. Delete the `Box::leak` hack. `RbacModule`/`RbacService` follow the same conversion as `user_service` if time allows, but are not required for the DI story to be complete (the `rbac_controller` can keep calling `auth::services::rbac_service::*(&state)` unchanged, since `AppState` still implements `AuthContext`).
7. **`crates/auth` internals: untouched**, per the coexistence decision — only the root crate's `AppState` wiring changes.
8. **Rewrite `tests/api.rs`**, fixing its currently-stale imports as part of pointing it at the new `AppState`/`Container`/composition root, and add one test demonstrating a `Container` built with `MockUserRepository` bound via `builder.bind::<dyn UserRepository>().to_arc(mock)`.
9. **Document it.** A short README section: how to write a provider (`#[injectable]`), group providers into a `#[module]`, consume one in a controller (`Injected<T>`), and add a new module to `AppModule` — including a short note on why `crates/auth` intentionally still uses its own `AuthContext` pattern.

## Verification

- `cargo build --workspace` and `cargo test --workspace` after each step, not just at the end — the order above is chosen so the workspace compiles after every step.
- `crates/di`'s own unit tests cover the container/macro mechanics independent of the app.
- After step 6, exercise the existing `api.http` requests (or `cargo test --test api`) against a local Postgres to confirm user CRUD and RBAC endpoints behave identically to before the migration — this is a refactor of *how* dependencies are wired, not a behavior change, so response bodies/status codes should be unchanged.
- Add at least one new fake-backed unit test (e.g. `UserService::create` against `MockUserRepository`) as a concrete demonstration that the migration's testability goal was achieved.

### Critical files
- `crates/api-core/src/crud.rs`, `crates/api-core/src/db.rs` — the global-pool anti-pattern being removed.
- `src/app/services/user_service.rs`, `src/app/controllers/user_controller.rs` — the first vertical slice proving `#[injectable]` / `#[module]` / `Injected<T>` end to end.
- `src/state.rs`, `src/main.rs` — the composition root.
- `crates/auth/src/context.rs` — the shape the `AppState` coexistence bridge must satisfy.
