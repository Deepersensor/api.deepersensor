# DeeperSensor API • Copilot Instructions

## Architecture snapshot
- Workspace crates: `crates/api` (axum HTTP surface), `crates/core` (config + `ApiError`), `crates/model` (LLM abstraction), `crates/auth` (argon2 hashing + JWT). Versions are centralized via `[workspace.dependencies]` in the root `Cargo.toml`.
- Boot sequence (`crates/api/src/main.rs`): load `AppConfig`, enforce secrets, init tracing, build the router, lazily connect Postgres, run `migrations/*.sql`, then `axum::serve` on the configured host/port.
- `AppState` (`state.rs`) bundles the `ModelProvider`, a shared `DashMap` of per-IP token buckets, the cloned config, and a `sqlx::PgPool`; handlers obtain it through `State<AppState>`.

## Request & handler patterns
- `build_app` (`app.rs`) layers request IDs, tracing spans, request-size/concurrency limits, CORS (`cors.rs`), and strict security headers; extend this stack instead of rebuilding middleware.
- Handlers return `ApiResult<T>` and surface failures through `ds_core::error::ApiError`, yielding JSON `{ "error": { code, message } }`. Use `ApiError::Unprocessable` for validation issues and record errors with `tracing` before bubbling them.
- Always apply `rate_limit(&state, ip)` at the top of unauthenticated handlers to share the in-memory `TokenBucket` logic.

## Routing & features
- Route wiring lives in `routes.rs`; follow the existing `Router::new().route(...).route(...)` pattern. Keep streaming chat endpoints under `/v1/chat` and reuse the SSE helper shown in `chat_stream_sse`.
- `ChatMessage`, `ChatRequest`, and `ChatChunk` come from `ds_model`; any new provider must implement the `ModelProvider` trait and get constructed inside `build_app`.
- Auth endpoints rely on plain `sqlx::query` calls. Hash passwords with `ds_auth::hash_password`, verify via `verify_password`, and issue tokens through `generate_tokens` with TTLs retrieved from `AppConfig`.
- Guard rails such as `validate_chat` enforce message count/length caps—extend these helpers instead of duplicating inline checks.

## Persistence & auth glue
- `sqlx::PgPool::connect_lazy` needs `DATABASE_URL` in the environment before boot. Startup migrations run automatically; add new files as `00XX_description.sql` to keep ordering deterministic.
- Auth tables live in `migrations/0001_init.sql`; conversation history scaffolding is in `0002_conversations_messages.sql` and is currently unused by the HTTP layer.
- Redis is declared in config but not yet wired; rate limiting is in-memory for now—respect this until the roadmap task enables Redis-backed buckets.

## Configuration & environments
- `AppConfig::load` layers defaults, `.env`, and live env vars. Reference `env.sample` for the authoritative key list and mirror updates when adding new settings.
- Production mode enforces a strong `JWT_SECRET` and proceeds even if `migrations/` is missing (logging a warning). Tests should either vendor the migrations directory or skip boot-time migration execution.
- Request IDs default to `x-request-id`; update `observability::REQUEST_ID_HEADER` and `env.sample` together if this changes.

## Developer workflow
- Local API: `cargo run -p api` (requires `DATABASE_URL` to point at a reachable Postgres; migrations run on startup).
- Tests: `cargo test --workspace` covers all crates; keep the root workspace compiling before sending PRs.
- Full stack: `docker compose up api postgres redis ollama nginx` mirrors production wiring (the API container reads `.env`). Nginx forwards `/api/` to the service and enforces additional rate limiting.
- Set `RUST_LOG=info,api=debug` for verbose traces; switch to JSON logs with `LOG_FORMAT=json`.

## Security & observability
- Security headers (HSTS, referrer policy, CSP, permissions) are injected globally in `build_app`; avoid reapplying them per route to prevent duplicates.
- Tracing spans (`TraceLayer::new_for_http`) annotate method, path, status, and latency; include contextual fields when logging errors (e.g., `user_id`, `email`).
- `/health` returns plain text and `/metrics` is a stub—future Prometheus work should extend the latter without removing the former.

## Collaboration cues
- When introducing new flows, update `TODO.md` so roadmap status stays accurate.
- Reuse shared helpers across crates (`ds-core`, `ds-auth`, `ds-model`) before adding new cross-cutting code.
- Document new env vars or workflow changes in both `env.sample` and this file to keep AI agents aligned with human onboarding.
