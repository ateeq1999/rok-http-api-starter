# DX Enhancement Plan — Less Syntax, JS-Like Experience

## Goal
Eliminate ~200 lines of boilerplate across the codebase. Make writing controllers and services feel more like AdonisJS — less syntax, more conventions.

---

## Phase 1 — `ValidatedJson<T>` Extractor

**Problem:** Every handler with a body does:
```rust
let body = validators::validate(body).map_err(|e| AppError::BadRequest(e.to_string()))?;
```
Repeated 7 times across controllers.

**Solution:** Custom axum extractor that combines deserialization + validation:
```rust
pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> Result<ApiResponse, AppError> { ... }
```

**Files:** Create `src/app/validators/extractor.rs`, update all controllers.

---

## Phase 2 — `thiserror` for AppError

**Problem:** `Display` impl is 12 lines of identical `write!(f, "{msg}")` across 6 variants.

**Solution:** Use `thiserror::Error` derive:
```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    NotFound(String),
    // ...
}
```

**Files:** `Cargo.toml` (add thiserror), `src/error.rs`.

---

## Phase 3 — `find_or_fail` on CrudService

**Problem:** 5 places do `find_by_id → check is_some → NotFound`.

**Solution:** Add to CrudService:
```rust
async fn find_or_fail(id: &str) -> Result<Self, sqlx::Error> {
    Self::find_by_id(id).await?.ok_or(sqlx::Error::RowNotFound)
}
```
The existing `From<sqlx::Error> for AppError` converts `RowNotFound` to `NotFound` automatically.

**Files:** `crates/api-core/src/crud.rs`, update services.

---

## Phase 4 — Eliminate `.map_err` with `From` impls

**Problem:** `.map_err(|e| AppError::Database(e.to_string()))` appears 25+ times.

**Solution:** Implement `From<argon2::password_hash::Error>`, `From<jsonwebtoken::errors::Error>` etc. for AppError, and add a helper trait:
```rust
pub trait IntoAppError<T> {
    fn or_internal(self) -> Result<T, AppError>;
}
impl<T> IntoAppError<T> for Result<T, sqlx::Error> {
    fn or_internal(self) -> Result<T, AppError> {
        self.map_err(|e| AppError::Database(e.to_string()))
    }
}
```

**Files:** `src/error.rs`, update services.

---

## Phase 5 — Shared TokenPair Type

**Problem:** Two `TokenPair` structs exist (api_core and auth_service) with manual mapping.

**Solution:** Use `api_core::auth::TokenPair` directly everywhere, derive Serialize.

**Files:** `crates/api-core/src/auth.rs`, `src/app/services/auth_service.rs`, `src/app/controllers/auth_controller.rs`.

---

## Phase 6 — `AppError` from common error types

**Problem:** `auth::hash_password` returns `argon2::Error` which needs `.map_err`.

**Solution:** Add blanket From impls:
```rust
impl From<argon2::password_hash::Error> for AppError { ... }
impl From<jsonwebtoken::errors::Error> for AppError { ... }
```

**Files:** `src/error.rs`.

---

## Phase 7 — Simplify ErrorCode (strum)

**Problem:** Two match blocks that must stay in sync.

**Solution:** Use `strum` derive or collapse into a single method returning `(StatusCode, &'static str)`.

**Files:** `crates/api-core/src/response.rs`.

---

## Migration Order

| Step | Phase | Description |
|------|-------|-------------|
| 1 | Phase 1 | ValidatedJson extractor |
| 2 | Phase 2 | thiserror for AppError |
| 3 | Phase 3 | find_or_fail on CrudService |
| 4 | Phase 4 | Error helper trait to eliminate .map_err |
| 5 | Phase 5 | Shared TokenPair type |
| 6 | Phase 6 | From impls for common errors |
| 7 | Phase 7 | Simplify ErrorCode |
| 8 | Build & verify | cargo check |

---

# Auth Parity & MVC DX Roadmap

**Goal:** bring `rok-api-starter` up to a "better-auth, but Rust" feature bar while keeping the AdonisJS-flavored MVC layout (`controllers/`, `models/`, `routes/`, `validators/`) and a Node-framework-like calling convention.

## Where the starter stands today

JWT + Argon2 auth, register/login/refresh/logout, forgot/reset password, email OTP verification, admin-gated user CRUD, avatar upload, a generic `CrudService` trait, and a migration CLI. Solid skeleton — the "email + password + JWT" slice of what a modern auth library ships.

## Gap analysis vs. better-auth's most-used surface

| Feature area | better-auth has it | rok-api-starter today | Gap |
|---|---|---|---|
| Email/password | ✅ core | ✅ | none |
| JWT/session + refresh rotation | ✅ core | ✅ basic | hardening only |
| Rate limiting | ✅ built-in, per-route | ❌ | **missing** |
| Cookie-based session option | ✅ default | ❌ bearer-only | **missing** |
| Magic link | ✅ plugin | ❌ | **missing** |
| Email OTP sign-in (not just verify) | ✅ plugin | ⚠️ OTP exists but verification-only | partial |
| Username sign-in | ✅ plugin | ❌ email-only | **missing** |
| Phone/SMS OTP | ✅ plugin | ❌ | **missing** |
| 2FA (TOTP + backup codes) | ✅ plugin | ❌ | **missing** |
| Passkey/WebAuthn | ✅ plugin | ❌ | **missing** |
| Social/OAuth sign-in | ✅ core | ⚠️ env vars exist, no routes/controller | **missing** |
| Account linking | ✅ | ❌ | **missing** |
| Session listing/revocation | ✅ | ❌ logout is single-session only | **missing** |
| Organizations / multi-tenancy | ✅ plugin | ❌ | **missing** |
| RBAC / access control | ✅ plugin | ⚠️ boolean `is_admin` only | partial |
| Admin plugin (ban, impersonate, force-logout) | ✅ plugin | ⚠️ basic CRUD only | partial |
| API keys (service-to-service auth) | ✅ plugin | ❌ | **missing** |
| Audit logging | ✅ | ❌ | **missing** |
| OpenTelemetry tracing | ✅ (1.6+) | ❌ | **missing** |
| SSO/SAML/SCIM (enterprise) | ✅ plugin | ❌ | **missing** (stretch) |

---

## Phase 1 — Session & Security Hardening (foundation, do first)

Everything after this phase assumes a hardened session layer.

- `tower-governor` for global + per-route rate limiting (`/auth/login`, `/auth/forgot-password`, `/otp/*` tighter).
- Cookie-based session mode alongside bearer JWT (`tower-cookies` + signed/encrypted cookie), selectable via `AUTH_STRATEGY=bearer|cookie`.
- Harden refresh token rotation: store refresh tokens hashed in DB with a `family_id` for reuse-after-rotation detection (replay-attack protection).
- `sessions` table (id, user_id, device/user_agent, ip, created_at, last_seen_at, revoked_at).
- CORS + security headers middleware (`tower-http` `CorsLayer`, `SetResponseHeaderLayer` for HSTS/X-Content-Type-Options).

**New migrations:** `sessions`, `refresh_token_families`.

---

## Phase 2 — Passwordless Sign-in

- `POST /auth/magic-link` (request) + `GET /auth/magic-link/verify` (consume), single-use signed token, short TTL.
- Promote OTP into full **email-OTP sign-in**: `POST /auth/otp/login/send` + `POST /auth/otp/login/verify`, distinct from registration-verification OTP.
- `username` field on `users` (nullable, unique) + `POST /auth/login` accepting username **or** email.
- Optional: phone OTP via pluggable SMS sender trait (Twilio/Africa's Talking — Tanzania deployment).

**New migrations:** `users.username`, `magic_link_tokens`.

---

## Phase 3 — Multi-Factor & Device Security

- TOTP 2FA: `totp-rs` crate, `POST /auth/2fa/enable`, `/auth/2fa/verify`, `/auth/2fa/disable`, plus single-use backup codes (hashed at rest).
- Passkey/WebAuthn: `webauthn-rs` crate, register + authenticate flows.
- Session listing + revocation: `GET /me/sessions`, `DELETE /me/sessions/{id}`, `DELETE /me/sessions` (revoke all but current).

**New migrations:** `two_factor_secrets`, `two_factor_backup_codes`, `passkeys`.

---

## Phase 4a — Plugin-like DX (reduce boilerplate) ✅ DONE

**Problem:** ~550 lines of controller+service code in `src/` with 18 `ApiResponse::ok(json!({}))`, 12 `.map_err` closures, and manual route wiring. User wants "configuration of plugins" feel.

### Response helpers (api-core)

Add to `ApiResponse`:
```rust
impl ApiResponse {
    pub fn message(msg: &str) -> Self { Self::ok(json!({ "message": msg })) }
    pub fn data(key: &str, val: impl Serialize) -> Self { Self::ok(json!({ key: val })) }
}
```
→ Replaces 12 of 18 `ApiResponse::ok(json!({...}))` calls.

### Error helpers (root error.rs)

Add method on `AppError`:
```rust
impl AppError {
    pub fn internal(msg: impl ToString) -> Self { Self::Internal(msg.to_string()) }
}
```
Add `OrBadRequest` trait (like existing `OrInternal`):
```rust
pub trait OrBadRequest<T> {
    fn or_bad_request(self) -> Result<T, AppError>;
}
```
→ Replaces 12 `.map_err(|e| AppError::Internal(e.to_string()))` closures.

### AuthPlugin builder (crates/auth)

Create `AuthPlugin` that auto-configures all auth routes:
```rust
let auth = AuthPlugin::builder()
    .google(GoogleOAuth::from_env())   // enables /auth/oauth/google/*
    .github(GithubOAuth::from_env())   // enables /auth/oauth/github/*
    .magic_link()                      // enables /auth/magic-link/*
    .login_otp()                       // enables /auth/otp/*
    .totp_2fa()                        // enables /auth/2fa/*
    .sessions()                        // enables /me/sessions
    .build(pool, mailer);

// In main.rs:
let app = Router::new()
    .merge(auth.public_routes())   // register, login, oauth redirect/callback, magic link, OTP
    .merge(auth.protected_routes().layer(jwt_layer))  // logout, refresh, 2fa, sessions
    .merge(api::routes().layer(jwt_layer));  // user CRUD, profile, OTP verify
```

The auth crate owns handlers + services internally. Root `src/` loses ~400 lines.

### Dependency inversion

Auth crate defines `AuthContext` trait:
```rust
pub trait AuthContext: Clone + Send + Sync + 'static {
    fn pool(&self) -> &PgPool;
    fn config(&self) -> &AuthConfig;
    fn mailer(&self) -> &dyn MailSender;
}
```
Root implements `AuthContext for AppState`. No circular deps.

### Files

- `crates/auth/src/plugin.rs` — AuthPlugin builder + handlers + routes
- `crates/auth/src/context.rs` — AuthContext trait
- `crates/api-core/src/response.rs` — ApiResponse::message, ApiResponse::data
- `src/error.rs` — AppError::internal, OrBadRequest
- `src/start/routes/mod.rs` — use AuthPlugin instead of manual nesting
- `src/start/routes/auth.rs` — DELETE (absorbed into plugin)
- `src/start/routes/api.rs` — keep user/OTP routes, sessions moves to plugin
- `src/app/controllers/auth_controller.rs` — DELETE (absorbed into plugin)
- `src/app/controllers/two_factor_controller.rs` — DELETE (absorbed into plugin)
- `src/app/controllers/session_controller.rs` — DELETE (absorbed into plugin)
- `src/app/services/auth_service.rs` — DELETE (absorbed into plugin)
- `src/app/services/two_factor_service.rs` — DELETE (absorbed into plugin)
- `src/app/services/session_service.rs` — DELETE (absorbed into plugin)
- `src/app/services/magic_link_service.rs` — DELETE (absorbed into plugin)
- `src/app/services/login_otp_service.rs` — DELETE (absorbed into plugin)
- Root `src/` keeps only: user_controller, user_service, otp_controller, profile stuff

**Net result:** ~400 lines removed from `src/`, auth becomes a self-contained plugin.

---

## Phase 4b — Social Login & Account Linking ✅ DONE

Wire `GOOGLE_CLIENT_ID`/`GOOGLE_CLIENT_SECRET` and `GITHUB_CLIENT_ID`/`GITHUB_CLIENT_SECRET` into actual OAuth flows using the `oauth2` crate. One user can have email/password + Google + GitHub.

### Database

**New migration `000017_accounts.sql`:**
```sql
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_account_id TEXT NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TIMESTAMPTZ,
    provider_user_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_accounts_provider_user ON accounts(provider, provider_account_id);
CREATE INDEX idx_accounts_user_id ON accounts(user_id);
```

One user can have multiple accounts (google + github + email/password). `provider_account_id` is the provider's unique ID (e.g. GitHub's numeric user ID, Google's sub claim).

### Config

Add to `AppConfig` + `.env`:
- `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_REDIRECT_URI`

### Dependencies

Add `oauth2 = "5"` to root `Cargo.toml`.

### Service: `oauth_service.rs`

```
start_authorization(provider, redirect_base) -> (url, state, pkce_verifier)
    - Builds OAuth2 authorize URL with PKCE + CSRF state
    - Stores state + pkce_verifier in DB (oauth_states table) or returns them to be cookie-set
    - Returns (auth_url, state, pkce_verifier)

handle_callback(provider, code, state, pkce_verifier) -> TokenPair
    - Exchanges code + pkce_verifier for tokens
    - Fetches user info from provider's userinfo endpoint
    - Finds or creates user by email (if email exists, link account; if not, create user)
    - Creates/updates account record
    - Returns JWT tokens (same as login)
```

### Controller: `oauth_controller.rs`

| Endpoint | Method | Description |
|---|---|---|
| `GET /auth/oauth/{provider}/redirect` | Public | Generates auth URL, redirects to provider |
| `GET /auth/oauth/{provider}/callback` | Public | Handles provider callback, returns tokens |

### Account linking (authenticated)

| Endpoint | Method | Description |
|---|---|---|
| `POST /me/accounts/link/{provider}` | Protected | Initiate linking for logged-in user |
| `DELETE /me/accounts/{provider}` | Protected | Unlink provider from account |
| `GET /me/accounts` | Protected | List linked providers |

### CSRF + PKCE state storage

Use cookies for state + pkce_verifier (simpler than DB table for single-server):
- `oauth_state` cookie (HttpOnly, SameSite=Lax, 5 min TTL)
- `oauth_pkce` cookie (HttpOnly, SameSite=Lax, 5 min TTL)

### Flow

1. `GET /auth/oauth/google/redirect` → generates state + PKCE, sets cookies, redirects to Google
2. Google redirects to `GET /auth/oauth/google/callback?code=...&state=...`
3. Controller validates state from cookie, exchanges code for tokens
4. Fetches user info (email, name, avatar)
5. Finds user by email or creates new user
6. Upserts account record
7. Returns JWT tokens (cookie or bearer based on AUTH_STRATEGY)

### Files

- `database/migrations/000017_accounts.up.sql` + `.down.sql`
- `src/app/services/oauth_service.rs`
- `src/app/controllers/oauth_controller.rs`
- `src/start/routes/auth.rs` (add OAuth routes)
- `src/config/app_config.rs` (add github env vars)
- `.env` (add GITHUB vars)
- `Cargo.toml` (add oauth2)
- `README.md` + `api.http` (update docs)

---

## Phase 5 — Authorization (RBAC) ✅

- `roles` + `permissions` + `role_permissions` + `user_roles` tables — minimal RBAC.
- Permission checks via `user.claims.has_permission("users.write")` in handlers (explicit, no magic).
- `AdminOnly` kept for backward compatibility but deprecated in favor of permission checks.
- RBAC management endpoints for admin role/permission CRUD.

### Database

**New migration `000018_rbac.sql`:**
```sql
CREATE TABLE roles (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, description TEXT, ...);
CREATE TABLE permissions (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, description TEXT, ...);
CREATE TABLE role_permissions (role_id, permission_id) -- composite PK
CREATE TABLE user_roles (user_id, role_id, assigned_at) -- composite PK
```

Seed: admin role gets all permissions, user role gets `users.read`. Auto-migrate existing `users.roles` string.

### Permission checking

Add `permissions` field to JWT Claims, query at login:
```rust
impl Claims {
    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.split(',').any(|p| p == perm)
    }
}

// In handlers:
if !user.claims.has_permission("users.delete") {
    return Err(AppError::forbidden("insufficient permissions"));
}
```

### Endpoints

| Endpoint | Method | Description |
|---|---|---|
| `GET /roles` | Admin | List all roles with permissions |
| `POST /roles` | Admin | Create role |
| `DELETE /roles/{id}` | Admin | Delete role |
| `POST /roles/{id}/permissions` | Admin | Grant permission to role |
| `DELETE /roles/{id}/permissions/{perm_id}` | Admin | Revoke permission from role |
| `POST /users/{id}/roles` | Admin | Assign role to user |
| `DELETE /users/{id}/roles/{role_id}` | Admin | Remove role from user |
| `GET /me/permissions` | Auth | List current user's permissions |

---

## Phase 6 — Multi-Tenancy / Organizations (skip unless needed)

Only if a concrete multi-tenant use case shows up — most expensive phase for least immediate payoff.

- `organizations`, `organization_members` (with role per org), `organization_invitations`.
- Scope `CrudService` resources to current organization via request-context extractor.

**New migrations:** `organizations`, `organization_members`, `organization_invitations`.

---

## Phase 7 — Admin & User Lifecycle

- Soft deletes on `users` (`deleted_at`) with `WithSoftDeletes` mixin trait on `CrudService`.
- Ban/unban (`users.banned_at`, `users.ban_reason`) enforced at auth-middleware layer.
- Admin impersonation: `POST /admin/users/{id}/impersonate` with scoped impersonation token.
- Force-logout: `DELETE /admin/users/{id}/sessions`.
- Audit log table (`actor_id`, `action`, `target_type`, `target_id`, `metadata jsonb`, `created_at`).

**New migrations:** `users.deleted_at`, `users.banned_at/ban_reason`, `audit_logs`.

---

## Phase 8 — API Keys & Service-to-Service Auth

- `api_keys` table (hashed key, owner user_id, scopes, expires_at, last_used_at) + Axum extractor (`Authorization: ApiKey ...` or `X-Api-Key`).
- `POST /me/api-keys`, `GET /me/api-keys`, `DELETE /me/api-keys/{id}`.
- Apply Phase 1 rate limiter per-API-key, not just per-IP.

**New migrations:** `api_keys`.

---

## Phase 9 — AdonisJS-Style Developer Experience

None of this is auth — all ergonomics.

- **Generators**: `cargo run -- make:controller Name`, `make:model Name`, `make:migration name`, `make:validator Name`.
- **Events & listeners**: `EventBus` (tokio broadcast channel) — `UserRegistered`, `PasswordReset`, `OrgInvited` fire events that mail/audit/webhook listeners subscribe to. Biggest thing that makes Rust controllers *feel* like Adonis controllers.
- **Mailable abstraction**: `Mailable` trait (`fn template() -> &'static str`, `fn data(&self) -> impl Serialize`) — `Mail::send(WelcomeEmail::new(user)).await?`.
- **Background jobs/queue**: `apalis` backed by Postgres/Redis, move OTP-send/email-send/cleanup off request path.
- **OpenAPI docs**: `utoipa` + `utoipa-swagger-ui` annotations, browsable `/docs`.

---

## Phase 10 — Observability & Production Hardening

- `tracing` + `tracing-opentelemetry` + OTLP exporter.
- Structured JSON logging in production.
- `/api/v1/health` checks DB pool + SMTP, separate `/ready` for orchestrators.
- Centralize Phase 7 audit log into queryable admin endpoint (`GET /admin/audit-logs`).

---

## Phase 11 — Enterprise Extensions (stretch, only if B2B customer needs it)

- SSO via SAML 2.0 / OIDC (`samael`).
- SCIM provisioning for directory sync.
- Billing/subscription hooks (Stripe webhooks → role/plan updates).

Treat as optional scope — matching better-auth feature-for-feature isn't worth it unless there's an actual enterprise buyer.

---

## Suggested new crates by phase

| Phase | Crate(s) |
|---|---|
| 1 | `tower-governor`, `tower-cookies`, `tower-http` (cors/headers) |
| 3 | `totp-rs` |
| 4a | none — refactoring into auth plugin |
| 4b | `oauth2` |
| 5 | none new — plain Rust traits + Postgres tables |
| 9 | `apalis`, `utoipa`, `utoipa-swagger-ui` |
| 10 | `tracing-opentelemetry`, `opentelemetry-otlp` |
| 11 | `samael` (SAML) |

## Directory evolution (after Phase 4a)

**Before (root `src/`):**
```
src/app/controllers/auth_controller.rs      (146 lines)
src/app/controllers/two_factor_controller.rs (32 lines)
src/app/controllers/session_controller.rs    (34 lines)
src/app/controllers/otp_controller.rs        (28 lines)
src/app/services/auth_service.rs            (246 lines)
src/app/services/two_factor_service.rs      (192 lines)
src/app/services/session_service.rs          (60 lines)
src/app/services/magic_link_service.rs       (90 lines)
src/app/services/login_otp_service.rs        (105 lines)
src/start/routes/auth.rs                     (51 lines)
                                          ─────────────
                                          ~984 lines
```

**After (root `src/`):**
```
src/app/controllers/user_controller.rs      (92 lines)
src/app/controllers/otp_controller.rs       (28 lines)
src/app/services/user_service.rs            (100 lines)
src/start/routes/mod.rs                     (15 lines — uses AuthPlugin)
src/start/routes/api.rs                     (26 lines — user CRUD + OTP)
                                          ─────────────
                                          ~261 lines

crates/auth/src/plugin.rs                  (~600 lines — all auth handlers + services + routes)
crates/auth/src/context.rs                 (10 lines — AuthContext trait)
```

**Net: ~723 lines removed from root `src/`, auth becomes self-contained plugin.**

## Recommended starting point

1. **Phase 4a first** (plugin DX) — reduces boilerplate, makes Phase 4b cleaner
2. **Phase 4b** (social login) — OAuth flows inside the plugin
3. **Phase 5** (RBAC) ✅ — permissions in JWT, explicit handler checks, RBAC management endpoints
4. Skip Phase 6 (organizations) unless a concrete multi-tenant requirement appears
