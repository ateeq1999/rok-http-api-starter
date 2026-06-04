# rok-api-starter

Axum API starter built on the [rok-ecosystem](https://github.com/anomalyco/rok) — featuring auth, ORM, validation, email, and OTP verification.

## Prerequisites

- [Docker](https://docs.docker.com/engine/install/) with Compose
- An external `infra` network and services (PostgreSQL, Mailpit):

```yaml
# ~/docker/infra/docker-compose.yml
name: infra
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: rok_api_db
    ports:
      - "5432:5432"
    networks:
      - infra

  mailpit:
    image: axllent/mailpit
    ports:
      - "1025:1025"
      - "8025:8025"
    networks:
      - infra

networks:
  infra:
    external: false
```

Start infrastructure **before** the app:
```bash
docker compose -f ~/docker/infra/docker-compose.yml up -d
```

## Quick Start

```bash
# Clone and start the app
git clone <repo-url> rok-api-starter
cd rok-api-starter
docker compose up -d

# Migrations run automatically on startup
# Server listens on http://localhost:8080
# Check logs to verify:
docker logs rok-api-starter -f
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgresql://postgres:postgres@postgres:5432/rok_api_db` | PostgreSQL connection string |
| `AUTH_SECRET` | `change-me-in-production` | JWT signing secret |
| `TOKEN_TTL` | `3600` | Access token TTL (seconds) |
| `REFRESH_TTL` | `2592000` | Refresh token TTL (seconds, default 30d) |
| `SMTP_HOST` | `mailpit` | SMTP server hostname |
| `SMTP_PORT` | `1025` | SMTP server port |
| `SMTP_USERNAME` | *(empty)* | SMTP auth username |
| `SMTP_PASSWORD` | *(empty)* | SMTP auth password |
| `SMTP_FROM` | `noreply@axum-app.dev` | From address for outgoing emails |
| `APP_URL` | `http://localhost:8080` | Public-facing app URL (used in email links) |
| `GOOGLE_CLIENT_ID` | *(empty)* | Google OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | *(empty)* | Google OAuth client secret |
| `OTP_LENGTH` | `6` | Numeric OTP code length (4–8) |

## API Endpoints

### Authentication (`/api/v1/auth`)

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/auth/register` | Guest | Register a new user |
| `POST` | `/auth/login` | Guest | Login and receive tokens |
| `POST` | `/auth/logout` | Bearer | Blacklist current access token |
| `POST` | `/auth/forgot-password` | Guest | Request password reset email |
| `POST` | `/auth/reset-password` | Guest | Submit reset token + new password |

### User Management (`/api/v1`)

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/users` | Admin | List all users |
| `POST` | `/users` | Admin | Create a user |
| `GET` | `/users/{id}` | Admin | Show a user |
| `PUT` | `/users/{id}` | Admin | Update a user |
| `DELETE` | `/users/{id}` | Admin | Delete a user |
| `GET` | `/me` | Bearer | Get current user profile |

### OTP / Email Verification (`/api/v1`)

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/otp/send` | Guest | Send verification email |
| `POST` | `/otp/verify` | Guest | Submit verification code |

## Testing Email Locally

[Mailpit](https://github.com/axllent/mailpit) runs alongside PostgreSQL in the infra stack. It catches all outgoing SMTP traffic and provides a web UI.

```bash
# View captured emails
open http://localhost:8025
```

Email flows:
- **Registration**: sends a verification email with a `{{app_url}}/verify-email?code=...` link and a numeric OTP code
- **Forgot password**: sends a reset email with a `{{app_url}}/reset-password?token=...` link
- **OTP send**: sends a verification email with an N-digit numeric code and a verify link

## Database Migrations

Migrations run automatically on container startup. SQL files live in `database/migrations/` and are applied in filename order.

```bash
# Check migration status in logs
docker logs rok-api-starter | grep Migrated

# Connect directly to inspect
docker exec -it rok-api-starter-db-1 psql -U postgres -d rok_api_db
```

## Project Structure

```
├── Cargo.toml
├── docker-compose.yml
├── Dockerfile
├── .env
├── api.http                     # API request examples (VS Code REST Client)
├── database/
│   └── migrations/              # SQL migration files
├── templates/                   # HTML email templates
└── src/
    ├── main.rs                  # App entrypoint, router setup
    ├── config.rs                # Environment config
    ├── state.rs                 # AppState, HasPool/HasAuth impls
    ├── mail.rs                  # SMTP mailer with lettre
    ├── migrations.rs            # Migration runner
    ├── error.rs                 # AppError type
    ├── guards.rs                # Admin role guard
    ├── social.rs                # Social auth hooks
    ├── models/user.rs           # User model + UserProvider impl
    ├── controllers/
    │   ├── auth.rs              # register, login, logout, forgot/reset password
    │   ├── otp.rs               # send/verify OTP
    │   └── user.rs              # CRUD, /me
    ├── routes/
    │   ├── mod.rs               # Router nesting
    │   ├── auth.rs              # Auth route definitions
    │   └── api.rs               # API route definitions
    └── validators/
        ├── auth.rs              # Register/Login/Forgot/Reset request structs
        ├── otp.rs               # SendOtp/VerifyOtp request structs
        └── user.rs              # Create/Update user request structs
```

## API Client

The `api.http` file contains ready-to-use requests for [VS Code REST Client](https://marketplace.visualstudio.com/items?itemName=humao.rest-client).

1. Register a user → update `@token` and `@refreshToken` with real values from the login response
2. Test authenticated endpoints with the `Authorization: Bearer {{token}}` header
3. Update `@userId` for user management endpoints
