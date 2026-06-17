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

## Phase 4 — Social Login & Account Linking

- Wire `GOOGLE_CLIENT_ID`/`GOOGLE_CLIENT_SECRET` into actual flow using `oauth2` crate: `GET /auth/oauth/{provider}/redirect`, `GET /auth/oauth/{provider}/callback`. GitHub as second provider.
- `accounts` table (user_id, provider, provider_account_id, access/refresh tokens) — one user can have email/password + Google + GitHub.
- `POST /me/accounts/link/{provider}` and `DELETE /me/accounts/{provider}`.

**New migrations:** `accounts`.

---

## Phase 5 — Authorization (Bouncer-style RBAC)

- `roles` + `permissions` + `role_permissions` + `user_roles` tables — minimal RBAC.
- `Policy` trait (`fn allows(&self, user: &User, resource: &R) -> bool`) + Axum extractor (`Can<UpdatePost>`).
- Replace `AdminOnly` with generic `RequirePermission("users.write")` extractor, keeping `AdminOnly` as alias.

**New migrations:** `roles`, `permissions`, `role_permissions`, `user_roles`.

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
| 3 | `totp-rs`, `webauthn-rs` |
| 4 | `oauth2` |
| 5 | none new — plain Rust traits + Postgres tables |
| 9 | `apalis`, `utoipa`, `utoipa-swagger-ui` |
| 10 | `tracing-opentelemetry`, `opentelemetry-otlp` |
| 11 | `samael` (SAML) |

## Suggested directory additions

```
src/
├── events/                 # Phase 9: EventBus + event structs + listeners
├── jobs/                   # Phase 9: apalis job definitions
├── mail/                   # Phase 9: Mailable trait + templates
├── policies/               # Phase 5: Policy trait impls per resource
├── controllers/
│   ├── oauth.rs             # Phase 4
│   ├── two_factor.rs        # Phase 3
│   ├── passkey.rs           # Phase 3
│   ├── sessions.rs          # Phase 1/3
│   ├── api_keys.rs          # Phase 8
│   ├── organizations.rs     # Phase 6
│   └── admin.rs             # Phase 7
```

## Recommended starting point

Phases 1 → 2 → 3 → 4 track better-auth's own core-then-plugins order and are independently shippable. Phase 5 (RBAC) is worth pulling forward if any admin/permission work is on the near-term roadmap. Phase 6 (organizations) and Phase 11 (enterprise) are the two phases to skip entirely unless a specific requirement shows up.
