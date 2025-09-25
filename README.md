# DeeperSensor API

A Rust (Axum) HTTP API that fronts local/remote AI model providers (initially Ollama), with basic auth (signup/login), rate limiting, and Postgres-backed persistence groundwork.

- Language/Runtime: Rust 1.82+, Tokio, Axum
- Crates: `api` (HTTP), `core` (config + errors), `model` (provider abstraction + Ollama client), `auth` (Argon2id + JWT)
- Infra (docker-compose): `api`, `postgres`, `redis` (future), `ollama`, `nginx`

## Architecture

```
[Client]
   |  HTTP (JSON/SSE)
   v
[Nginx]  <-- optional TLS termination + rate limiting
   |
   v
[API (Axum)] -- SQLX --> [Postgres]
   |  \
   |   \-- HTTP --> [Ollama]
   |
   \-- (future) Redis for rate limiting / sessions
```

Key behaviors
- JSON logs with request spans and request ID propagation (`x-request-id`).
- CORS configurable via env; security headers enforced globally.
- Migrations auto-run on boot if `migrations/` is present.
- Model listing proxies to Ollama; chat streaming is currently a stub that echoes.

## Endpoints

- `GET /health` → `200 ok`
- `GET /metrics` → placeholder metrics text
- `GET /v1/models` → `["model:tag", ...]` (proxied from Ollama `/api/tags`)
- `POST /v1/chat` → `[ { model, content, done }, ... ]` (non-streamed aggregate)
- `POST /v1/chat/stream` (SSE) → `event: chunk` data=`{ model, content, done }`
- `POST /v1/auth/signup` → `{ id, email }`
- `POST /v1/auth/login` → `{ access_token }` (JWT HS256)

Examples

```
# List models (requires Ollama up at OLLAMA_BASE_URL)
curl -s http://localhost:8080/v1/models

# Chat (echo stub)
curl -s -X POST http://localhost:8080/v1/chat \
  -H 'content-type: application/json' \
  -d '{"model":"llama3","messages":[{"role":"user","content":"hello"}]}'

# Signup
curl -s -X POST http://localhost:8080/v1/auth/signup \
  -H 'content-type: application/json' \
  -d '{"email":"user@example.com","password":"correct horse battery staple"}'

# Login
curl -s -X POST http://localhost:8080/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"user@example.com","password":"correct horse battery staple"}'
```

## Configuration

All configuration is via environment variables (with `.env` supported for local dev). See `env.sample` for full list; common keys:

- App: `APP_ENV` (local|production), `APP_HOST`, `APP_PORT`, `APP_PUBLIC_URL`
- Logging: `RUST_LOG`, `LOG_FORMAT` (text|json), `REQUEST_ID_HEADER`
- Auth: `JWT_SECRET` (>=32 chars in prod), `JWT_ISSUER`, `JWT_ACCESS_TTL_SECS`, `JWT_REFRESH_TTL_SECS`
- CORS: `ALLOWED_ORIGINS`, `CORS_ALLOW_*`
- Rate limit: `RATE_LIMIT_ENABLED`, `RATE_LIMIT_REQUESTS_PER_MINUTE`, `RATE_LIMIT_BURST`
- Model provider: `OLLAMA_BASE_URL`, `OLLAMA_DEFAULT_TIMEOUT_MS`
- Postgres: `DATABASE_URL`
- HTTP server: `SERVER_*`, `MAX_REQUEST_SIZE_BYTES`, `TRUSTED_PROXY_IPS`, `FORCE_HTTPS`

Notes
- In `production`, startup fails if `JWT_SECRET` is unset/weak (<32 chars).
- Default `DATABASE_URL` points to local Postgres (`postgres://postgres:postgres@localhost:5432/deepersensor`). Override for Docker networks (`...@postgres:5432/...`).

## Quickstart (local dev)

Prerequisites
- Rust 1.82+ (rustup), Docker (optional), Postgres 16 (local or Docker)
- Optional: Ollama if you want `/v1/models` to return real data

1) Create `.env`

```
cp env.sample .env
# change at least:
#   JWT_SECRET=<generate a secure random 64 bytes/characters>
#   DATABASE_URL=postgres://postgres:postgres@localhost:5432/deepersensor
```

2) Start Postgres (choose one)

- Using Docker
```
docker run -d --name pg -p 5432:5432 \
  -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=deepersensor \
  postgres:16-alpine
```
- Or install locally and create the `deepersensor` database.

3) (Optional) Start Ollama
```
docker run -d --name ollama -p 11434:11434 ollama/ollama:latest
# then set OLLAMA_BASE_URL=http://localhost:11434
```

4) Run the API
```
cargo run -p api
```

- Server binds to `APP_HOST:APP_PORT` (default `0.0.0.0:8080`).
- Migrations in `migrations/` are applied automatically at startup.
- Logs are JSON by default; set `RUST_LOG=info,api=debug` for more detail.

## One-command local stack (docker-compose)

This brings up API, Postgres, Redis, Ollama, and Nginx reverse proxy on `:80`.

Important: Because compose sets `APP_ENV=production`, you MUST provide a strong `JWT_SECRET` in `.env`.

```
cp env.sample .env
# edit .env: set JWT_SECRET to >= 32 chars
# if running inside compose network, set DATABASE_URL=postgres://postgres:postgres@postgres:5432/deepersensor

docker compose up -d --build
# Nginx: http://localhost/ -> proxies /api/* to API on :8080
# Health: http://localhost/api/health (via Nginx)
```

- Images: `crates/api/Dockerfile` (distroless runtime), `nginx/Dockerfile`
- Healthchecks: API `GET /health`; Postgres and Ollama expose their ports internally
- Logs: `docker compose logs -f api` (JSON)

## Production deployment

Strategy A: Docker Compose (single host)
- Provision a VM with Docker.
- Create `.env` with production values. Ensure:
  - `APP_ENV=production`
  - Strong `JWT_SECRET` (>=32 chars); do NOT commit secrets.
  - `DATABASE_URL` points to your managed Postgres or self-hosted Postgres service (not `localhost` inside Nginx container).
- Build and start:
```
docker compose -f docker-compose.yml build
docker compose -f docker-compose.yml up -d
```
- TLS: Terminate TLS in front of Nginx (e.g., Caddy/Traefik) or replace the Nginx image with a TLS-enabled config.

Strategy B: Images + Orchestrator (Kubernetes, Nomad)
- Build/push images:
```
docker build -t your-registry/deepersensor-api:$(git rev-parse --short HEAD) -f crates/api/Dockerfile .
docker build -t your-registry/deepersensor-nginx:$(git rev-parse --short HEAD) -f nginx/Dockerfile .
docker push your-registry/deepersensor-*
```
- Deploy with your manifests (Service, Deployment, ConfigMap/Secret for `.env`).
- Expose only Nginx (or your ingress) publicly; keep Postgres/Redis private.

Migrations
- The API runs SQLx migrations at startup using `migrations/`.
- For safer rollouts, run migrations as a separate init job before starting the API (same image, `api` binary will apply and exit if you wrap it accordingly).

Health/Observability
- Liveness/readiness: `GET /health`
- Logs: structured JSON; include `x-request-id` in responses; propagate via Nginx.
- `/metrics` is a placeholder; wire to Prometheus in a later phase.

## Database schema

`migrations/` contains initial tables:
- `0001_init.sql`: `users(id, email unique, password_hash, created_at)`
- `0002_conversations_messages.sql`: `conversations(id, user_id, title, created_at)` and `messages(id, conversation_id, role, content, created_at)`

## Security notes

- Set a unique, random `JWT_SECRET` in production (>=32 chars). The app enforces this.
- CORS: restrict `ALLOWED_ORIGINS` to your domains in prod.
- Behind a proxy: ensure `X-Forwarded-Proto` and trusted proxies (`TRUSTED_PROXY_IPS`) are accurate.
- Nginx adds basic security headers; API additionally enforces HSTS, X-Content-Type-Options, X-Frame-Options, CSP, Referrer-Policy, and Permissions-Policy.

## Development tips

- Workspace build: `cargo build -p api`
- Run with extra logs: `RUST_LOG=info,api=debug cargo run -p api`
- Change request/response size limits with `MAX_REQUEST_SIZE_BYTES`.
- Rate limit: per-IP token bucket in-process (configurable); future: Redis-backed.

## Troubleshooting

- API exits immediately in compose: likely weak/missing `JWT_SECRET` with `APP_ENV=production`.
- `/v1/models` fails: ensure Ollama is reachable at `OLLAMA_BASE_URL` (compose service `ollama:11434` or local `localhost:11434`).
- DB connection refused: verify `DATABASE_URL`; with compose use hostname `postgres` not `localhost`.
- CORS blocked in browser: set `ALLOWED_ORIGINS` appropriately and restart.

## License

MIT
