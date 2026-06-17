# AdonisJS-Style MVC Restructure

## Goal
Restructure the project to follow AdonisJS-like MVC conventions: `app/` namespace for domain logic, `start/` for boot-time wiring, `config/` for configuration, proper middleware layer, service layer extraction, and standardized response envelope.

---

## Current Structure
```
src/
├── main.rs
├── cli.rs
├── config.rs
├── state.rs
├── error.rs
├── auth.rs
├── mail.rs
├── storage.rs
├── lib.rs
├── controllers/
├── models/
├── routes/
└── validators/
```

## Target Structure
```
src/
├── main.rs                          # Boot only
├── lib.rs                           # Module declarations
├── error.rs                         # AppError enum
├── state.rs                         # AppState
├── storage.rs                       # File upload utils
├── app/
│   ├── controllers/
│   │   ├── mod.rs
│   │   ├── auth_controller.rs
│   │   ├── user_controller.rs
│   │   └── otp_controller.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   └── email_verification_token.rs
│   ├── services/
│   │   ├── mod.rs
│   │   ├── auth_service.rs
│   │   ├── user_service.rs
│   │   └── otp_service.rs
│   ├── middleware/
│   │   ├── mod.rs
│   │   └── auth_middleware.rs
│   ├── validators/
│   │   ├── mod.rs
│   │   ├── auth_validator.rs
│   │   ├── user_validator.rs
│   │   └── otp_validator.rs
│   └── mails/
│       ├── mod.rs
│       └── mailer.rs
├── start/
│   ├── mod.rs
│   └── routes.rs
├── config/
│   ├── mod.rs
│   └── app_config.rs
└── database/
    └── migrations/                  # (already exists at root)
```

---

## Phase 1 — Create `app/` Namespace

- Create `src/app/` directory
- Move `src/controllers/` → `src/app/controllers/`
- Move `src/models/` → `src/app/models/`
- Move `src/validators/` → `src/app/validators/`
- Move `src/mail.rs` → `src/app/mails/mailer.rs`
- Update all imports across the project

---

## Phase 2 — Create `config/` Module

- Create `src/config/` directory
- Move `src/config.rs` → `src/config/app_config.rs`
- Create `src/config/mod.rs` re-exporting `AppConfig`

---

## Phase 3 — Create `start/` Module (Route Boot)

- Create `src/start/` directory
- Move `src/routes/` → `src/start/routes.rs`
- Create `src/start/mod.rs` with `pub mod routes`
- Update `main.rs` to import from `start::routes`

---

## Phase 4 — Extract Service Layer

Create `src/app/services/` with business logic extracted from controllers:

### `auth_service.rs`
- `register(state, dto) → Result<User, AppError>`
- `login(state, dto) → Result<TokenPair, AppError>`
- `refresh_token(state, dto) → Result<TokenPair, AppError>`
- `forgot_password(state, dto) → Result<(), AppError>`
- `reset_password(state, dto) → Result<(), AppError>`

### `user_service.rs`
- `list_users() → Result<Vec<User>, AppError>`
- `get_user(id) → Result<User, AppError>`
- `create_user(dto) → Result<User, AppError>`
- `update_user(id, dto) → Result<User, AppError>`
- `delete_user(id) → Result<bool, AppError>`
- `get_profile(user_id) → Result<User, AppError>`
- `upload_avatar(state, user_id, file) → Result<String, AppError>`

### `otp_service.rs`
- `send_otp(state, dto) → Result<(), AppError>`
- `verify_otp(state, dto) → Result<(), AppError>`

Controllers become thin HTTP adapters — extract params, call service, return response.

---

## Phase 5 — Add Middleware Layer

Create `src/app/middleware/`:

### `auth_middleware.rs`
- `JwtAuth` — tower `Layer`/`Service` that extracts Bearer token, verifies JWT, injects `Claims` into request extensions
- `OptionalJwtAuth` — same but doesn't error if no token
- Apply via `.layer(JwtAuth)` on protected route groups

### Update route groups
```
// Public routes
auth::routes()

// Protected routes (require valid JWT)
api::routes().layer(JwtAuth)

// Admin-only routes
admin::routes().layer(JwtAuth).layer(AdminOnly)
```

---

## Phase 6 — Standardize Response Envelope

Update `crates/api-core/src/response.rs` to match AdonisJS format:

```json
// Success
{ "data": { ... } }

// Created
{ "data": { ... } }

// Paginated
{ "data": [...], "meta": { "total": 100, "page": 1, "per_page": 20, "total_pages": 5 } }

// Error
{ "error": { "code": "E_VALIDATION", "message": "Invalid email" } }
```

Update `ApiResponse::ok()`, `ApiResponse::created()`, `ApiResponse::error()` to always wrap in `{ "data": ... }` or `{ "error": ... }`.

---

## Phase 7 — Update `lib.rs` and `main.rs`

### `lib.rs`
```rust
pub mod app;
pub mod config;
pub mod start;
pub mod error;
pub mod state;
pub mod storage;
```

### `main.rs`
- Import `start::routes::app_router`
- Import `config::AppConfig`
- Boot: load config → init pool → run migrations → build router → serve

---

## Phase 8 — Clean Up Old Files

- Delete `src/controllers/` (moved to `src/app/controllers/`)
- Delete `src/models/` (moved to `src/app/models/`)
- Delete `src/validators/` (moved to `src/app/validators/`)
- Delete `src/routes/` (moved to `src/start/`)
- Delete `src/mail.rs` (moved to `src/app/mails/`)
- Delete `src/config.rs` (moved to `src/config/`)
- Delete `src/cli.rs` (move into `main.rs` or `start/`)

---

## Phase 9 — Build & Verify

- `cargo check` — zero errors
- `cargo build` — clean compile
- Verify all imports resolve
- Run existing tests if any

---

## Migration Order

| Step | Phase | Description |
|------|-------|-------------|
| 1 | Phase 1 | Create `app/` namespace, move controllers + models + validators + mails |
| 2 | Phase 2 | Create `config/` module |
| 3 | Phase 3 | Create `start/` module, move routes |
| 4 | Phase 4 | Extract service layer from controllers |
| 5 | Phase 5 | Add middleware layer |
| 6 | Phase 6 | Standardize response envelope |
| 7 | Phase 7 | Update lib.rs + main.rs wiring |
| 8 | Phase 8 | Delete old files |
| 9 | Phase 9 | Build & verify |
