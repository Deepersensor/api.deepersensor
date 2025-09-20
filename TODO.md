# DeeperSensor API & Frontend Roadmap (TODO)

Last Updated: 2025-09-20

## Legend
- Status: [ ] pending · [~] in progress · [x] completed · [-] cancelled
- Priority: (H) High · (M) Medium · (L) Low
- IDs match internal automation / tooling references

Overall Progress: 0 / 16 completed (0%)

---
## Phase 0 – Foundation & Workspace
| ID | Task | Priority | Status | Notes |
|----|------|----------|--------|-------|
| workspace | Convert to Cargo workspace (crates: api, core, model, auth) | H | [ ] | Prepare modular boundaries early |
| deps | Add foundational crates (axum, tokio, tracing, serde, sqlx, argon2, jsonwebtoken, config/dotenv, anyhow/thiserror) | H | [ ] | Keep minimal; add as needed |
| config | Unified config loader (env + .env + defaults) | H | [ ] | Support prod profile separation |
| logging | Structured logging + tracing + metrics (/metrics) | H | [ ] | Use tracing-subscriber + optionally opentelemetry |
| errors | Unified error type and HTTP mapper | M | [ ] | Map domain + validation + upstream errors |

## Phase 1 – Core AI & HTTP Surface
| ID | Task | Priority | Status | Notes |
|----|------|----------|--------|-------|
| model-provider | Trait: ModelProvider + Ollama client stub | H | [ ] | Graceful timeouts & retries |
| http-routes | Base routes: /health, /v1/models, /v1/chat (stream stub) | H | [ ] | Chat returns streaming SSE or chunked JSON |
| auth | Auth: signup/login, password hashing, JWT issuance & refresh | H | [ ] | Support short-lived access + refresh token |
| persistence | Postgres integration (sqlx) + migrations folder | H | [ ] | Migration tool: sqlx migrate / refinery |

## Phase 2 – Platform & Reliability
| ID | Task | Priority | Status | Notes |
|----|------|----------|--------|-------|
| rate-limiting | Redis-backed rate limiting + request ID middleware | M | [ ] | Per-IP + per-user buckets |
| docker | Multi-stage Dockerfile + docker-compose (api, postgres, redis, ollama, nginx) | H | [ ] | Enable reproducible local stack |
| nginx | Reverse proxy config (TLS termination, gzip, cache rules) | M | [ ] | Forward /api to backend & serve Next.js |
| security | CORS policy, security headers (STRICT-TRANSPORT-SECURITY, etc.), input validation hooks | M | [ ] | Add body size limits |

## Phase 3 – Frontend Integration
| ID | Task | Priority | Status | Notes |
|----|------|----------|--------|-------|
| nextjs | Scaffold Next.js app (web/) with auth pages + chat UI stub | M | [ ] | Use edge runtime where suitable |
| docs | Expand README with architecture & run instructions | M | [ ] | Include env var matrix |

## Phase 4 – Quality & Testing
| ID | Task | Priority | Status | Notes |
|----|------|----------|--------|-------|
| testing | Integration test harness (spawn test server, seed db) | L | [ ] | Include auth & chat happy paths |

---
## Task Details & Acceptance Criteria

### workspace
- Create root `Cargo.toml` workspace listing member crates.
- Crates: `crates/api` (HTTP), `crates/core` (domain types + errors), `crates/model` (model provider abstraction), `crates/auth` (auth logic).
- Move existing `api` crate code into `crates/api`.

### deps
- Add only essential dependencies initially.
- Ensure reproducible builds with `Cargo.lock` committed.

### config
- Support layered config: defaults -> file -> env override.
- Provide `AppConfig` struct (server.port, db.url, redis.url, jwt.secret, ollama.url, log.level).
- Fail-fast on required missing prod secrets.

### logging
- Initialize tracing with EnvFilter (`RUST_LOG`).
- JSON log option via env flag (e.g., `LOG_FORMAT=json`).
- Add request span: method, path, status, latency, request_id, user_id(optional).
- Expose `/metrics` (Prometheus) later; stub placeholder first.

### errors
- Domain error enum -> HTTP status mapping (401, 403, 404, 422, 429, 500).
- Return structured JSON: `{ "error": { "code": "..", "message": ".." } }`.

### model-provider
- Trait: `list_models()`, `chat_stream(request) -> Stream<Result<Chunk>>`.
- Ollama client: configurable base URL, proper timeout, error taxonomy.
- Mock provider for tests.

### http-routes
- `/health` -> 200 JSON status.
- `/v1/models` -> array of models from provider.
- `/v1/chat` -> POST { model, messages[] } streaming response (SSE or chunked JSON tokens).

### auth
- Password hashing (Argon2id, memory-hard params).
- Signup: store user (id UUID, email unique, password_hash, created_at).
- Login: verify + issue JWT (HS256) + refresh token (stored or stateless with rotation).
- Middleware: extract user claims, attach to request extensions.

### persistence
- Postgres schema: users, conversations, messages.
- Migrations folder with first schema migration.
- Connection pool with health check.

### rate-limiting
- Redis token bucket keyed by user or IP.
- Return 429 with retry-after header.

### docker
- Multi-stage Rust build (builder + slim runtime or distroless).
- Compose services: api, postgres, redis, ollama, nginx, (optional) jaeger.
- Healthchecks defined.

### nginx
- Upstreams: api:8080, web:3000.
- Enforce security headers, gzip text/*, brotli if available.
- Limit body size for uploads (e.g., 2m initial).

### nextjs
- Directory: `web/` (not yet created).
- Pages: `/login`, `/signup`, `/chat`.
- API client wrapper with JWT attach & refresh.

### docs
- Expand README: architecture diagram (ASCII), local run, env vars, build & deploy steps.

### security
- CORS allow specific origins (prod domain + localhost dev).
- Strict header policy (HSTS, X-Content-Type-Options, etc.).
- Input validation (email formats, length limits, model name whitelist).

### testing
- Integration test harness spawning ephemeral test db (apply migrations) and test server.
- Tests: signup/login flow, unauthorized access, model listing, chat streaming stub (mock provider).

---
## Open Design Decisions (To Resolve)
- Streaming format: SSE vs JSON lines vs WebSocket.
- JWT refresh strategy: stored refresh table vs rotating stateless tokens.
- Telemetry stack: OpenTelemetry + collector? (phase gating).
- Model abstraction: support future providers (OpenAI, local huggingface, etc.).
- Metrics crate: `prometheus` vs `opentelemetry-prometheus`.

---
## Changelog
- 2025-09-20: Initial TODO roadmap created.

---
## Update Instructions
1. Update statuses as work proceeds (search by ID).
2. Keep acceptance criteria immutable; append clarifications below rather than rewriting history.
3. Add new tasks under proper phase; increment total for progress metric.
4. Record changes in Changelog with date.
