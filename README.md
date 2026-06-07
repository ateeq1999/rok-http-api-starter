# rok-api-starter

Axum + SQLx API starter. Features auth (JWT, Argon2), CRUD service, email (OTP, password reset), avatar upload, PostgreSQL migrations, and CLI commands.

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
| `cargo run -- server --run-migrations` | Run migrations then start server |
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

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/auth/register` | — | Register (password: 8+ chars, upper + lower + digit + special) |
| `POST` | `/auth/login` | — | Login, returns access + refresh tokens |
| `POST` | `/auth/refresh` | — | Rotate a refresh token |
| `POST` | `/auth/logout` | Bearer | Logout current session |
| `POST` | `/auth/forgot-password` | — | Request password reset email |
| `POST` | `/auth/reset-password` | — | Submit reset token + new password |

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
├── Cargo.toml                    # workspace root
├── crates/api-core/              # shared generic crate
│   └── src/
│       ├── response.rs           # ApiResponse + ErrorCode
│       ├── db.rs                 # OnceLock<PgPool>
│       ├── crud.rs               # CrudService trait + FieldValue
│       ├── auth.rs               # JWT, argon2, sha2, uuid utils
│       ├── migrations.rs         # run/rollback/fresh/refresh/status
│       ├── validator.rs          # validate() + ValidationRejection
│       ├── health.rs             # health check handler
│       └── prelude.rs            # re-exports
├── src/
│   ├── main.rs                   # binary entry point
│   ├── lib.rs                    # library root (exposes modules)
│   ├── config.rs                 # AppConfig from env
│   ├── state.rs                  # AppState with FromRef
│   ├── auth.rs                   # AuthUser / AdminOnly extractors
│   ├── error.rs                  # AppError enum
│   ├── db.rs                     # re-exports api_core::db
│   ├── response.rs               # re-exports api_core::response
│   ├── storage.rs                # avatar file I/O
│   ├── mail.rs                   # SMTP mailer via lettre
│   ├── models/
│   │   ├── user.rs               # User struct + CrudService impl
│   │   └── email_verification_token.rs
│   ├── controllers/
│   │   ├── auth.rs               # register, login, refresh, logout, forgot/reset
│   │   ├── otp.rs                # send/verify OTP
│   │   └── user.rs               # CRUD, /me, avatar upload
│   ├── routes/
│   │   ├── mod.rs                # Router + ServeDir for uploads
│   │   ├── auth.rs
│   │   └── api.rs
│   └── validators/
│       ├── auth.rs               # RegisterRequest, etc. + password validator
│       ├── otp.rs
│       └── user.rs
├── database/migrations/          # .up.sql / .down.sql pairs
├── templates/                    # HTML email templates
├── tests/api.rs                  # integration tests
├── api.http                      # VS Code REST Client examples
└── .env                          # environment overrides
```

## API Client

The `api.http` file contains ready-to-use requests for [VS Code REST Client](https://marketplace.visualstudio.com/items?itemName=humao.rest-client).

1. Register a user → copy tokens into `@token` / `@refreshToken`
2. Test authenticated endpoints with `Authorization: Bearer {{token}}`
3. Update `@userId` for admin user management
