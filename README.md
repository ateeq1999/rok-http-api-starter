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
│   ├── api-core/                     # shared generic crate (CRUD, DB, response)
│   │   └── src/
│   │       ├── response.rs           # ApiResponse + ErrorCode
│   │       ├── db.rs                 # OnceLock<PgPool>
│   │       ├── crud.rs               # CrudService trait + FieldValue
│   │       ├── migrations.rs         # run/rollback/fresh/refresh/status
│   │       ├── validator.rs          # validate() + ValidationRejection
│   │       ├── health.rs             # health check handler
│   │       └── lib.rs                # module exports + prelude
│   └── auth/                         # auth primitives + middleware + extractors
│       └── src/
│           ├── primitives.rs         # Claims, TokenPair, JWT, Argon2, SHA256
│           ├── middleware.rs          # JwtAuthLayer, AuthStrategy (bearer/cookie)
│           ├── extractors.rs         # AuthUser, AdminOnly
│           ├── validators.rs         # RegisterRequest, LoginRequest, ValidatedJson<T>
│           ├── session.rs            # Session model
│           └── lib.rs                # module exports + prelude
├── src/
│   ├── main.rs                       # binary entry point, CORS, security headers
│   ├── lib.rs                        # library root
│   ├── config/
│   │   ├── mod.rs
│   │   └── app_config.rs             # AppConfig from env (AUTH_STRATEGY, etc.)
│   ├── state.rs                      # AppState with FromRef
│   ├── error.rs                      # AppError enum + OrInternal trait
│   ├── storage.rs                    # avatar file I/O
│   ├── app/
│   │   ├── controllers/
│   │   │   ├── auth_controller.rs    # register, login, refresh, logout, forgot/reset
│   │   │   ├── otp_controller.rs     # send/verify OTP
│   │   │   └── user_controller.rs    # CRUD, /me, avatar upload
│   │   ├── services/
│   │   │   ├── auth_service.rs       # register, login (username/email), refresh (family revocation)
│   │   │   ├── magic_link_service.rs # magic link request + verify
│   │   │   ├── login_otp_service.rs  # login OTP send + verify
│   │   │   ├── otp_service.rs        # registration OTP generation + email
│   │   │   ├── session_service.rs    # session CRUD
│   │   │   └── user_service.rs       # user CRUD + avatar
│   │   ├── models/
│   │   │   ├── user.rs               # User struct + CrudService impl
│   │   │   └── email_verification_token.rs
│   │   ├── mails/
│   │   │   └── mailer.rs             # SMTP mailer via lettre
│   │   ├── middleware/                # (empty — middleware lives in crates/auth)
│   │   └── validators/
│   │       ├── mod.rs                # re-exports from auth crate
│   │       └── user.rs               # CreateUserRequest, UpdateUserRequest
│   └── start/
│       └── routes/
│           ├── mod.rs                # app_router() with JWT layer
│           ├── auth.rs               # auth routes with rate limiting
│           └── api.rs                # protected API routes
├── database/migrations/              # .up.sql / .down.sql pairs (12 migrations)
├── templates/                        # HTML email templates
├── api.http                          # VS Code REST Client examples
└── .env                              # environment overrides
```

## Architecture Notes

- **Three-crate workspace**: `api-core` (generic), `auth` (auth primitives), root (app)
- **AdonisJS MVC**: `controllers/`, `services/`, `models/`, `validators/`, `routes/`
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
