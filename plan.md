# Pure Axum + SQLx Migration Plan

## Goal
Remove all `rok-*` crate dependencies, replacing with pure **axum** + **sqlx** (`0.8`). Implement JWT-based auth, SQLx migrations, and manual validation — no ORM, no framework magic.

---

## Phase 1 — Dependencies (`Cargo.toml`)

### Remove
- `rok-core`
- `rok-auth`
- `rok-orm`
- `rok-validate`

### Add
- `jsonwebtoken` — JWT encode/decode for access & refresh tokens
- `sha2` — SHA-256 hashing for tokens & OTP codes
- `uuid` — ID generation (replace rok-core's Cuid2)
- `validator` — derive-based request validation (replace rok-validate)
- `argon2` — password hashing
- `base64` — for token encoding
- `rand` — already present, keep for OTP generation

### Keep
- `axum`, `sqlx`, `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`, `tower-http`, `chrono`, `anyhow`, `async-trait`, `lettre`

---

## Phase 2 — Config (`config.rs`)

- Remove `AuthConfig` / `rok_auth::AuthConfig` import
- Add raw fields: `auth_secret: String`, `token_ttl: Duration`, `refresh_ttl: Duration`
- Remove `auth_config()` method
- Keep all SMTP, OAuth, OTP config fields

---

## Phase 3 — State (`state.rs`)

- Remove `rok_auth::axum::{HasAuth, HasPool}` trait impls
- Remove `rok_auth::Auth` field
- Replace with `auth_secret: String` + `token_ttl: Duration` + `refresh_ttl: Duration` in AppState
- Keep `pool: PgPool`, `config: AppConfig`, `mailer: Mailer`

---

## Phase 4 — App Startup (`main.rs`)

- Remove `rok_auth::Auth::new()` creation
- Remove `AuthLayer` and `OrmLayer` middleware
- Replace with manual JWT auth middleware (extract Bearer token, decode, inject into extensions)
- Replace `rok_orm::MigrationRunner` with raw SQL file runner or `sqlx::migrate!`

---

## Phase 5 — Error Handling (`error.rs`)

- Remove `rok_core::api::ApiResponse`
- Replace `ApiResponse::error()` / `ApiResponse::ok()` with native `(StatusCode, Json<Value>)` responses
- Keep `AppError` enum — implement `IntoResponse` returning `(StatusCode, Json<...>)` directly
- Remove `rok_auth::AuthError` From impl

---

## Phase 6 — Guards (`guards.rs`)

- Remove `rok_auth::axum::guard::RoleMarker`
- Implement custom axum **extractors** that check JWT claims for role
  - `AdminGuard` — checks `claims.roles` contains `"admin"`, returns 403 otherwise
  - `AuthGuard` — checks valid JWT exists, injects claims
- Use axum's `Extension<Claims>` or custom extractor pattern

---

## Phase 7 — Social (`social.rs`)

- Remove `rok_auth::social::SocialAuthHooks`
- Either remove entirely or keep as a placeholder struct for future OAuth callback handling

---

## Phase 8 — Migrations (`migrations.rs`)

- Remove `rok_orm::{FileSource, MigrationRunner}`
- Use `sqlx::migrate::Migrator` with `sqlx::migrate!("./database/migrations")`
- Or run raw SQL files manually via `sqlx::raw_sql`

---

## Phase 9 — Models

### `models/user.rs`

- Remove `rok_orm::Model` derive and `PgModel` / `SqlValue` usages
- Remove `UserProvider` trait impl
- Add manual `sqlx::FromRow` only
- Replace rok-orm query methods with raw sqlx queries:
  - `find_by_email` → `sqlx::query_as!` or `query_as`
  - `create_user` → `sqlx::query` with `INSERT ... RETURNING *`
  - `find_by_pk` → `sqlx::query_as("SELECT * FROM users WHERE id = $1")`
  - `all` → `sqlx::query_as("SELECT * FROM users")`
  - `update_by_pk` → `sqlx::query("UPDATE users SET ... WHERE id = $1")`
  - `delete_by_pk` → `sqlx::query("DELETE FROM users WHERE id = $1")`

### `models/email_verification_token.rs`

- Remove `rok_orm::Model` / `Table` derive
- Replace with raw sqlx queries:
  - `create` → `INSERT INTO email_verification_tokens ...`
  - `filter(...).first()` → `SELECT * FROM email_verification_tokens WHERE ... LIMIT 1`
  - `update_where` → `UPDATE email_verification_tokens SET ... WHERE ...`
  - `update_by_pk` → `UPDATE ... WHERE id = $1`

### New: `models/mod.rs` — helper module

- Add shared DB helper functions if needed

---

## Phase 10 — Controllers

### `controllers/auth.rs`

- Remove `rok_auth::axum::{GuestOnly, RequestContext}` extractors
- Remove `rok_auth::{login, register, password, AuthError, Claims}`
- Replace JWT operations with `jsonwebtoken` crate calls:
  - **register**: hash password (argon2), insert user, generate JWT pair, return tokens
  - **login**: find user by email, verify password hash, generate JWT pair
  - **logout**: no-op or optionally blacklist token
  - **forgot_password**: generate reset token, store in `password_resets` table, send email
  - **reset_password**: verify reset token, hash new password, update user
- Add helper functions: `create_jwt()`, `verify_jwt()`, `generate_token_pair()`

### `controllers/user.rs`

- Remove `rok_auth::axum::{RequestContext, RequireRole}`
- Replace all `User::all()`, `User::find_by_pk()`, `User::create_user()`, `User::update_by_pk()`, `User::delete_by_pk()` with raw sqlx queries
- Replace `RequireRole<Admin>` with custom `AdminGuard` extractor
- Replace `Claims` with custom `JwtClaims` struct or custom extractor

### `controllers/otp.rs`

- Remove `rok_auth::axum::{GuestOnly, RequestContext}`
- Remove `rok_auth::hash::sha256_hex`
- Replace with raw `sha2::Sha256` hashing
- Replace all model query methods with raw sqlx queries:
  - OTP send: hash code, invalidate old tokens, insert new, send email
  - OTP verify: find token by user_id + hash + not used + not expired, mark used, verify user email

---

## Phase 11 — Validators

### `validators/auth.rs`, `validators/user.rs`, `validators/otp.rs`

- Remove `rok_validate::{Validate, Valid}` imports and derive
- Replace with `validator::Validate` from the `validator` crate
- Replace axum `Valid<T>` extractor with a custom `ValidatedJson<T>` extractor that:
  1. Deserializes via `serde::Deserialize`
  2. Runs `validate()` method
  3. Returns 422 with error details on failure

---

## Phase 12 — Auth Utilities (New File)

Create `src/auth.rs` with:
- `Claims` struct (sub, exp, iat, roles, etc.)
- `generate_token_pair()` — creates access + refresh JWT
- `verify_token()` — validates JWT and returns claims
- `hash_password()` — argon2 hashing
- `verify_password()` — argon2 verification
- `sha256_hex()` — utility for OTP/token hashing
- Custom axum extractors:
  - `AuthUser` — extracts + verifies Bearer token from `Authorization` header, returns claims
  - `OptionalAuth` — same but doesn't error if no token
  - `AdminOnly` — wraps `AuthUser` and verifies admin role

---

## Phase 13 — Response Helpers (Optional)

Create `src/response.rs` with:
- `JsonResponse::ok(data)` → `(200, Json)`
- `JsonResponse::created(data)` → `(201, Json)`
- `JsonResponse::no_content()` → `(204, {})`
- `JsonResponse::error(code, msg, status)` → `(status, Json{{"error": code, "message": msg}})`

---

## Migration Order (Recommended)

| Step | Files | Description |
|------|-------|-------------|
| 1 | `Cargo.toml` | Update dependencies |
| 2 | `src/auth.rs` | New: JWT + password + extractors |
| 3 | `src/response.rs` | New: JSON response helpers |
| 4 | `src/config.rs` | Simplification |
| 5 | `src/state.rs` | Remove rok traits |
| 6 | `src/error.rs` | Pure axum IntoResponse |
| 7 | `src/migrations.rs` | Use sqlx migrations |
| 8 | `src/models/user.rs` | Raw sqlx queries |
| 9 | `src/models/email_verification_token.rs` | Raw sqlx queries |
| 10 | `src/guards.rs` | Custom extractors |
| 11 | `src/validators/*` | validator crate migration |
| 12 | `src/controllers/auth.rs` | Remove rok-auth |
| 13 | `src/controllers/user.rs` | Remove rok-orm |
| 14 | `src/controllers/otp.rs` | Remove rok-orm |
| 15 | `src/main.rs` | Final wiring |
| 16 | `src/social.rs` | Cleanup |
| 17 | Build & test | `cargo check`, `cargo build` |

---

## Files to Delete
- None explicitly — all files get rewritten in place except `social.rs` which may be removed.

## Files to Create
- `src/auth.rs` — JWT + password + extractors
- `src/response.rs` — JSON response helpers (optional, can inline)
