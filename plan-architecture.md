# Architecture Refactor Plan — crates/auth & crates/api-core

## Goal
Restructure auth crate for readability and simplicity. Max 150 lines per file, clear responsibilities, DRY code, fix known bugs.

## Changes Overview

### 1. Split `plugin.rs` (429 lines) → `plugin.rs` (65 lines) + `handlers/` (8 files)
- `plugin.rs`: AuthPlugin builder + route assembly only
- `handlers/auth.rs`: register, login, refresh, logout, forgot_password, reset_password
- `handlers/magic_link.rs`: request + verify
- `handlers/login_otp.rs`: send + verify
- `handlers/otp.rs`: send + verify (email verification)
- `handlers/two_factor.rs`: enable, verify, disable
- `handlers/sessions.rs`: list, revoke, revoke_all
- `handlers/oauth.rs`: redirect + callback (fix cookie bug)
- `handlers/rbac.rs`: role/permission management endpoints

### 2. Extract duplicated `generate_otp()` to `primitives.rs`
- Remove from `otp_service.rs` and `login_otp_service.rs`
- Single source of truth in primitives

### 3. Split `context.rs` → `config.rs` + `context.rs`
- `config.rs`: AuthConfig, OAuthProviderConfig (data structs)
- `context.rs`: AuthContext, MailSender, UserFinder, PermissionFinder (traits only)

### 4. Fix bugs
- `oauth_redirect` cookie bug: cookies never set on redirect response
- N+1 query in `rbac_service::list_roles()`: use single JOIN
- Remove `std::process::exit(1)` from `api-core/migrations.rs`
- Fix inconsistent error formats in rejections

### 5. Minor cleanup
- Fix `#[allow(dead_code)]` warnings in crud.rs
- Remove unused `OAuthState` struct from oauth_service.rs
- Remove unused `UsernameLoginRequest` from validators.rs

## File-by-file plan

### `crates/auth/src/plugin.rs` (NEW: ~65 lines)
- AuthPlugin struct + AuthPluginBuilder
- `public_routes()` and `protected_routes()` methods
- `token_response()` helper (shared by all handlers)
- Import handlers from `crate::handlers::*`

### `crates/auth/src/handlers/mod.rs` (NEW: ~20 lines)
```rust
pub mod auth;
pub mod magic_link;
pub mod login_otp;
pub mod otp;
pub mod two_factor;
pub mod sessions;
pub mod oauth;
pub mod rbac;
```

### `crates/auth/src/handlers/auth.rs` (NEW: ~80 lines)
- `register()`, `login()`, `refresh_handler()`, `logout()`
- `forgot_password()`, `reset_password()`

### `crates/auth/src/handlers/magic_link.rs` (NEW: ~45 lines)
- `magic_link_request()`, `magic_link_verify()`

### `crates/auth/src/handlers/login_otp.rs` (NEW: ~45 lines)
- `login_otp_send()`, `login_otp_verify()`

### `crates/auth/src/handlers/otp.rs` (NEW: ~45 lines)
- `otp_send()`, `otp_verify()`

### `crates/auth/src/handlers/two_factor.rs` (NEW: ~65 lines)
- `two_factor_enable()`, `two_factor_verify()`, `two_factor_disable()`

### `crates/auth/src/handlers/sessions.rs` (NEW: ~45 lines)
- `session_list()`, `session_revoke()`, `session_revoke_all()`

### `crates/auth/src/handlers/oauth.rs` (NEW: ~75 lines)
- `oauth_redirect()` (fix: set cookies on redirect response)
- `oauth_callback()`

### `crates/auth/src/handlers/rbac.rs` (NEW: ~80 lines)
- `list_roles()`, `create_role()`, `delete_role()`
- `grant_permission()`, `revoke_permission()`
- `list_permissions()`, `assign_role()`, `remove_role()`, `my_permissions()`

### `crates/auth/src/config.rs` (NEW: ~35 lines, extracted from context.rs)
- `OAuthProviderConfig` struct
- `AuthConfig` struct

### `crates/auth/src/context.rs` (NEW: ~35 lines, traits only)
- `trait MailSender`
- `trait UserFinder`
- `trait PermissionFinder`
- `trait AuthContext`
- `struct UserRecord`

### `crates/auth/src/primitives.rs` (EDIT: add generate_otp)
- Add `pub fn generate_otp(length: u32) -> String`

### `crates/auth/src/lib.rs` (EDIT: add handlers module)
- Add `pub mod handlers;`
- Update prelude

### `crates/auth/src/services/otp_service.rs` (EDIT: remove generate_otp)
- Use `primitives::generate_otp()`

### `crates/auth/src/services/login_otp_service.rs` (EDIT: remove generate_otp)
- Use `primitives::generate_otp()`

### `crates/auth/src/services/rbac_service.rs` (EDIT: fix N+1)
- `list_roles()` uses single JOIN query instead of N+1

### `crates/api-core/src/migrations.rs` (EDIT: remove process::exit)
- Return error instead of calling `std::process::exit(1)`

### `crates/api-core/src/crud.rs` (EDIT: fix warnings)
- Remove or handle `#[allow(dead_code)]` on FieldValue variants

## Implementation Order

1. Create `config.rs` and update `context.rs` (split)
2. Add `generate_otp()` to `primitives.rs`
3. Create `handlers/` directory and all handler files
4. Rewrite `plugin.rs` to use handlers
5. Update `lib.rs` and prelude
6. Fix `rbac_service.rs` N+1 query
7. Fix `otp_service.rs` and `login_otp_service.rs` to use primitives
8. Fix `api-core/migrations.rs` process::exit
9. Fix `crud.rs` dead_code warnings
10. `cargo check` + fix errors
