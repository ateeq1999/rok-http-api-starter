# rok-api-starter

Axum + SQLx API starter with AdonisJS-style MVC layout. Auth (JWT + Argon2), cookie or bearer sessions, rate limiting, refresh token rotation with family-based revocation, CRUD service, email (OTP, password reset), avatar upload, PostgreSQL migrations, and CLI.

```
cargo run                    # start server
cargo run -- db migrate      # run migrations
cargo run -- db fresh        # drop + re-run all
cargo run -- db status       # check migration state
```

## Prerequisites

- [Docker](https://docs.docker.com/engine/install/) with Compose
- Rust 1.75+

Start infrastructure (PostgreSQL + Mailpit):

```yaml
# docker-compose.infra.yml
name: infra
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: axum_app
    ports:
      - "5432:5432"
  mailpit:
    image: axllent/mailpit
    ports:
      - "1025:1025"
      - "8025:8025"
```

```bash
docker compose -f docker-compose.infra.yml up -d
```

## Quick Start

```bash
# Create .env with DATABASE_URL
echo 'DATABASE_URL="postgres://postgres:postgres@localhost:5432/axum_app"' > .env

# Create the database (first time only)
docker exec -it infra-postgres-1 createdb -U postgres axum_app

# Run migrations
cargo run -- db migrate

# Start the server
cargo run
# → http://localhost:8080
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/axum_app` | PostgreSQL connection string |
| `AUTH_SECRET` | `change-me-in-production` | JWT signing secret |
| `AUTH_STRATEGY` | `bearer` | Auth strategy: `bearer` (Authorization header) or `cookie` (HTTP-only cookies) |
| `TOKEN_TTL` | `3600` | Access token TTL (seconds) |
| `REFRESH_TTL` | `2592000` | Refresh token TTL (seconds) |
| `SMTP_HOST` | `localhost` | SMTP server hostname |
| `SMTP_PORT` | `1025` | SMTP server port |
| `SMTP_FROM` | `noreply@axum-app.dev` | From address for emails |
| `APP_URL` | `http://localhost:8080` | Public URL (used in email links) |
| `GOOGLE_CLIENT_ID` | *(empty)* | Google OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | *(empty)* | Google OAuth client secret |
| `OTP_LENGTH` | `6` | Numeric OTP code length (4–8) |
| `STORAGE_DIR` | `./storage` | Directory for file uploads |

## CLI Commands

| Command | Description |
|---|---|
| `cargo run` | Start the HTTP server |
| `cargo run -- server` | Start the HTTP server (migrations run automatically) |
| `cargo run -- db migrate` | Apply pending migrations |
| `cargo run -- db rollback` | Roll back the last migration |
| `cargo run -- db fresh` | Drop all tables then re-run all migrations |
| `cargo run -- db refresh` | Roll back all then re-apply all |
| `cargo run -- db status` | List applied/pending migrations |

Migrations use `.up.sql` / `.down.sql` pairs in `database/migrations/`.

## API Endpoints

### Health

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/health` | — | Returns `{ "status": "ok" }` |

### Authentication (`/api/v1/auth`)

| Method | Path | Auth | Rate Limit | Description |
|--------|------|------|------------|-------------|
| `POST` | `/auth/register` | — | 5 burst, 1/sec | Register (password: 8+ chars, upper + lower + digit + special) |
| `POST` | `/auth/login` | — | 5 burst, 1/sec | Login with email **or** username, returns access + refresh tokens |
| `POST` | `/auth/refresh` | — | — | Rotate a refresh token (family-based reuse detection) |
| `POST` | `/auth/logout` | Bearer | — | Logout current session |
| `POST` | `/auth/forgot-password` | — | 10 burst, 2/sec | Request password reset email |
| `POST` | `/auth/reset-password` | — | — | Submit reset token + new password |
| `POST` | `/auth/magic-link` | — | 10 burst, 2/sec | Request magic link email (15 min expiry) |
| `GET` | `/auth/magic-link/verify?token=...` | — | — | Verify magic link, returns tokens (sets cookies if cookie mode) |
| `POST` | `/auth/otp/login/send` | — | 5 burst, 1/sec | Send login OTP code via email (10 min expiry) |
| `POST` | `/auth/otp/login/verify` | — | 5 burst, 1/sec | Verify login OTP code, returns tokens |

### Two-Factor Authentication (`/api/v1/auth`)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/auth/2fa/enable` | Bearer | Initiate 2FA — returns secret, otpauth URL, and backup codes |
| `POST` | `/auth/2fa/verify` | Bearer | Verify TOTP code to activate 2FA |
| `POST` | `/auth/2fa/disable` | Bearer | Disable 2FA (requires password + TOTP code) |

### Sessions (`/api/v1`)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/me/sessions` | Bearer | List active sessions |
| `DELETE` | `/me/sessions/{id}` | Bearer | Revoke a specific session |
| `DELETE` | `/me/sessions` | Bearer | Revoke all sessions |

### Profile (`/api/v1`)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/me` | Bearer | Get current user |
| `POST` | `/me/avatar` | Bearer | Upload profile picture (multipart, max 5 MB) |

### User Management — Admin (`/api/v1`)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/users` | Admin | List all users |
| `POST` | `/users` | Admin | Create a user |
| `GET` | `/users/{id}` | Admin | Show user |
| `PUT` | `/users/{id}` | Admin | Update user |
| `DELETE` | `/users/{id}` | Admin | Delete user |

### RBAC — Role & Permission Management (`/api/v1`)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/roles` | Admin | List all roles with permissions |
| `POST` | `/roles` | Admin | Create role |
| `DELETE` | `/roles/{id}` | Admin | Delete role (system roles protected) |
| `POST` | `/roles/{id}/permissions` | Admin | Grant permission to role |
| `DELETE` | `/roles/{id}/permissions/{perm_id}` | Admin | Revoke permission from role |
| `GET` | `/permissions` | Admin | List all permissions |
| `POST` | `/users/{id}/roles` | Admin | Assign role to user |
| `DELETE` | `/users/{id}/roles/{role_id}` | Admin | Remove role from user |
| `GET` | `/me/permissions` | Bearer | List current user's permissions |

### Social Login (`/api/v1/auth`)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/auth/oauth/{provider}/redirect` | — | Redirect to OAuth provider (Google, GitHub) |
| `GET` | `/auth/oauth/{provider}/callback` | — | Handle OAuth callback, returns tokens |

## Permission Checking

JWT tokens include a comma-separated `permissions` field. Check permissions in handlers:

```rust
if !user.claims.has_permission("users.write") {
    return Err(AppError::forbidden("insufficient permissions"));
}
```

Available permissions: `users.read`, `users.write`, `users.delete`, `roles.read`, `roles.write`, `roles.delete`, `permissions.read`, `permissions.write`.

### OTP / Email Verification (`/api/v1`)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/otp/send` | — | Send verification OTP email |
| `POST` | `/otp/verify` | — | Submit verification code |

## Auth Strategy

Set `AUTH_STRATEGY` to switch between bearer tokens and cookie-based sessions:

- **`bearer`** (default): Tokens returned in JSON response body. Client sends `Authorization: Bearer <token>`.
- **`cookie`**: Tokens set as HTTP-only cookies (`access_token`, `refresh_token`). Middleware reads from cookies automatically. Use for browser-based apps.

## Security Features

- **Refresh token rotation** with family-based revocation — token reuse triggers family-wide revocation and all user sessions are terminated
- **Rate limiting** on auth endpoints via `tower-governor` (IP-based)
- **Security headers**: `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Strict-Transport-Security`
- **CORS** enabled (permissive in dev — tighten for production)
- **Password policy**: 8+ chars, uppercase, lowercase, digit, special character

## Password Policy

Registration and password reset require:
- Minimum 8 characters, maximum 128
- At least one uppercase letter
- At least one lowercase letter
- At least one digit
- At least one special character

## Testing Email

[Mailpit](https://github.com/axllent/mailpit) catches all outgoing SMTP. Open http://localhost:8025 to view sent emails.

Email flows:
- **Registration**: verification email with OTP code
- **Forgot password**: reset link
- **OTP send**: verification code

## Running Tests

```bash
# Create a test database
docker exec -it infra-postgres-1 createdb -U postgres axum_app_test

# Run migrations on test DB
DATABASE_URL="postgres://postgres:postgres@localhost:5432/axum_app_test" cargo run -- db migrate

# Run tests
DATABASE_URL="postgres://postgres:postgres@localhost:5432/axum_app_test" cargo test
```

## Project Structure

```
├── Cargo.toml                        # workspace root
├── crates/
│   ├── api-core/                     # shared generic crate (CRUD, response)
│   │   └── src/
│   │       ├── response.rs           # ApiResponse + ErrorCode + message()/data() helpers
│   │       ├── crud.rs               # CrudService trait (&PgPool-parameterized) + FieldValue
│   │       ├── migrations.rs         # run/rollback/fresh/refresh/status
│   │       ├── validator.rs          # validate() + ValidationRejection
│   │       ├── health.rs             # health check handler
│   │       └── lib.rs                # module exports + prelude
│   ├── di/                           # DI container runtime: Container, Injectable, Injected<T>
│   │   └── src/
│   │       ├── container.rs          # ContainerBuilder / Container (type-keyed provider map)
│   │       ├── error.rs              # DiError
│   │       ├── extractor.rs          # Injected<T>: axum FromRequestParts
│   │       └── lib.rs                # re-exports + Injectable / Module traits
│   ├── di-macros/                    # #[injectable] and #[module] proc-macros
│   │   └── src/lib.rs
│   └── auth/                         # auth plugin (self-contained, own AuthContext DI — see below)
│       └── src/
│           ├── plugin.rs             # AuthPlugin builder + handlers + routes
│           ├── context.rs            # AuthContext, MailSender, UserFinder traits
│           ├── error.rs              # AuthError enum
│           ├── primitives.rs         # Claims, TokenPair, JWT, Argon2, SHA256
│           ├── middleware.rs          # JwtAuthLayer, AuthStrategy (bearer/cookie)
│           ├── extractors.rs         # AuthUser, AdminOnly
│           ├── validators.rs         # RegisterRequest, LoginRequest, ValidatedJson<T>
│           ├── session.rs            # Session model
│           ├── services/             # auth services (generic over AuthContext)
│           │   ├── auth_service.rs   # register, login, refresh, forgot/reset
│           │   ├── magic_link_service.rs
│           │   ├── login_otp_service.rs
│           │   ├── otp_service.rs
│           │   ├── session_service.rs
│           │   └── two_factor_service.rs
│           └── lib.rs                # module exports + prelude
├── src/
│   ├── main.rs                       # binary entry point, CORS, security headers
│   ├── lib.rs                        # library root
│   ├── config/
│   │   ├── mod.rs
│   │   └── app_config.rs             # AppConfig from env (AUTH_STRATEGY, etc.) — a DI provider
│   ├── state.rs                      # AppModule, AppState, bootstrap() — the composition root
│   ├── error.rs                      # AppError enum + OrInternal trait
│   ├── storage.rs                    # avatar file I/O (used by LocalAvatarStorage)
│   ├── app/
│   │   ├── controllers/
│   │   │   └── user_controller.rs    # CRUD, /me, avatar upload — uses Injected<UserService>
│   │   ├── services/
│   │   │   └── user_service.rs       # #[injectable] UserService (+ mock-backed unit tests)
│   │   ├── repositories/             # DI provider interfaces + Postgres implementations
│   │   │   ├── user_repository.rs    # UserRepository trait + PgUserRepository
│   │   │   ├── avatar_storage.rs     # AvatarStorage trait + LocalAvatarStorage
│   │   │   ├── user_finder.rs        # AppUserFinder: bridges into auth::context::UserFinder
│   │   │   └── permission_finder.rs  # AppPermissionFinder: bridges into auth::context::PermissionFinder
│   │   ├── models/
│   │   │   ├── user.rs               # User struct + CrudService impl
│   │   │   └── email_verification_token.rs
│   │   └── mails/
│   │       └── mailer.rs             # SMTP mailer via lettre; implements auth::context::MailSender
│   └── start/
│       └── routes/
│           ├── mod.rs                # app_router() using AuthPlugin
│           └── api.rs                # protected API routes (user CRUD)
├── database/migrations/              # .up.sql / .down.sql pairs (16 migrations)
├── templates/                        # HTML email templates
├── api.http                          # VS Code REST Client examples
└── .env                              # environment overrides
```

## Dependency Injection

Providers are wired through a small NestJS-flavored DI framework (`crates/di` + `crates/di-macros`) instead of free functions calling a global connection pool.

**Writing a provider** — annotate a struct with `#[injectable]`; every `#[inject]`-marked field is resolved from the container by its type (works for both `Arc<Concrete>` and `Arc<dyn Trait>`):

```rust
#[injectable]
pub struct PgUserRepository {
    #[inject]
    pool: Arc<PgPool>,
}
```

A struct where every field is `#[inject]` also gets an `Injectable` impl, making it eligible for the bare-name form inside `#[module(providers = [...])]`. A struct with a plain (non-`#[inject]`) field still gets a generated `new()`, but must be built by hand at the composition root instead.

**Grouping providers into a module** — list providers in dependency order (leaves first); `Impl as dyn Trait` binds an implementation to an interface:

```rust
#[module(
    providers = [
        PgUserRepository as dyn UserRepository,
        LocalAvatarStorage as dyn AvatarStorage,
        UserService,
    ],
)]
pub struct AppModule;
```

**Consuming a provider in a controller** — the `Injected<T>` extractor pulls an `Arc<T>` straight from the container, no `State<AppState>` plumbing needed:

```rust
pub async fn index(_admin: AdminOnly, Injected(users): Injected<UserService>) -> Result<ApiResponse, AppError> {
    let list = users.list().await?;
    Ok(ApiResponse::ok(serde_json::json!({ "users": list })))
}
```

**Adding a new module**: define it with `#[module(...)]`, then add it to `AppModule`'s `providers` (bare, since modules aren't nested via `providers` — use `imports = [OtherModule]` instead) and register it in `src/state.rs::bootstrap()`.

**Testing**: because services depend on interfaces (`UserRepository`, `AvatarStorage`), unit tests construct them directly with hand-written fakes via the plain `new()` the macro generates — no container, no database. See `src/app/services/user_service.rs`'s `#[cfg(test)]` module for a worked example.

Construction is eager and synchronous: `AppModule::register` runs once at startup inside `state::bootstrap()`, right after the pool/mailer are connected — a missing or misordered provider fails the whole binary immediately with a clear `DiError`, never a lazy panic mid-request. Anything fallible or async to build (`PgPool::connect`, `Mailer::new`'s SMTP transport) is constructed *before* the container and inserted as a pre-built leaf; `Injectable::construct` itself is always synchronous.

**Why `crates/auth` doesn't use this container.** Its services/handlers are generic over an `AuthContext` trait (`fn mailer(&self) -> &dyn MailSender`, etc.) — a different, equally legitimate DI pattern (static dispatch via generics) that predates this framework and covers the largest, most security-sensitive surface in the repo (JWT, OTP, magic link, OAuth, 2FA, RBAC). Rewriting it wasn't worth the risk for a starter kit, so `AppState` instead bridges the two: a handful of fields (`pool`, `mailer`, `user_finder`, `permission_finder`) are resolved from the container *once* at bootstrap and cached, satisfying `AuthContext`'s borrow-shaped accessors without a `Box::leak` or per-call reconstruction. See the `AppUserFinder`/`AppPermissionFinder` providers in `src/app/repositories/` for the bridge implementations.

## Architecture Notes

- **Five-crate workspace**: `api-core` (generic), `di` + `di-macros` (DI framework), `auth` (self-contained plugin), root (app)
- **Auth plugin pattern**: `AuthPlugin::builder().magic_link().login_otp().totp_2fa().sessions().build()` — configure what you need, nothing else
- **Dependency injection**: root-crate services/repositories are `#[injectable]`, wired by `#[module]`s into one `Container` at startup (see "Dependency Injection" above); `auth` crate keeps its own trait-based `AuthContext` pattern, bridged into the container's `AppState`
- **AdonisJS MVC**: `controllers/`, `services/`, `repositories/`, `models/`, `routes/`
- **No ORM** — raw SQLx queries with `CrudService` trait for generic CRUD
- **`ValidatedJson<T>`** extractor: auto-validates request bodies (no boilerplate)
- **`OrInternal`** trait: eliminates `.map_err(|e| AppError::Database(e.to_string()))`
- **Rate limiting**: per-route via `tower-governor` on sensitive auth endpoints
- **Session management**: `sessions` table tracks active sessions, supports per-device revocation

## API Client

The `api.http` file contains ready-to-use requests for [VS Code REST Client](https://marketplace.visualstudio.com/items?itemName=humao.rest-client).

1. Register a user → copy tokens into `@token` / `@refreshToken`
2. Test authenticated endpoints with `Authorization: Bearer {{token}}`
3. Update `@userId` for admin user management
