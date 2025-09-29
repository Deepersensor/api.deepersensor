# DeeperSensor API - Architecture Documentation

> **System Architecture and Design Decisions**  
> Version: 0.1.0  
> Last Updated: September 2025

## Table of Contents

1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [Component Design](#component-design)
4. [Data Flow](#data-flow)
5. [Security Model](#security-model)
6. [Scaling Strategy](#scaling-strategy)
7. [Technology Stack](#technology-stack)
8. [Design Decisions](#design-decisions)

---

## Overview

DeeperSensor API is a production-grade Rust backend service that provides a unified HTTP API for interacting with local and remote AI model providers (initially Ollama). The system is designed for:

- **High Performance:** Async I/O with Tokio runtime
- **Type Safety:** Leveraging Rust's compile-time guarantees
- **Observability:** Structured logging, distributed tracing, metrics
- **Security:** JWT authentication, rate limiting, defense-in-depth
- **Scalability:** Stateless design, horizontal scaling support

### Key Features

- ✅ **Authentication:** User signup/login with Argon2id password hashing and JWT tokens
- ✅ **Model Abstraction:** Provider-agnostic interface for LLM interaction
- ✅ **Streaming Support:** Server-Sent Events (SSE) for real-time chat responses
- ✅ **Rate Limiting:** Per-IP and per-user token bucket implementation
- ✅ **Persistence:** PostgreSQL for users, conversations, and message history
- ✅ **Caching:** Redis for rate limiting and future session management
- ✅ **Reverse Proxy:** Nginx with security headers, compression, and request routing

---

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          Internet / Clients                          │
└──────────────────────────┬──────────────────────────────────────────┘
                           │ HTTPS (TLS)
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Reverse Proxy (Nginx)                            │
│  • TLS Termination                                                   │
│  • Security Headers (CSP, HSTS, X-Frame-Options)                     │
│  • Rate Limiting (Nginx layer)                                       │
│  • Request ID Generation                                             │
│  • Compression (gzip, brotli)                                        │
│  • Load Balancing (multi-instance)                                   │
└──────────────────────────┬──────────────────────────────────────────┘
                           │ HTTP (internal)
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Application Layer (Axum)                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Middleware Stack                          │   │
│  │  • Request ID Propagation                                    │   │
│  │  • Tracing Spans                                             │   │
│  │  • CORS                                                      │   │
│  │  • Security Headers                                          │   │
│  │  • Request Size Limits                                       │   │
│  │  • Concurrency Limits                                        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  Auth Routes │  │ Chat Routes  │  │ Model Routes │              │
│  │              │  │              │  │              │              │
│  │ • Signup     │  │ • Chat       │  │ • List       │              │
│  │ • Login      │  │ • Stream     │  │ • Info       │              │
│  │ • Refresh    │  │              │  │              │              │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘              │
│         │                 │                 │                        │
│         └─────────────────┼─────────────────┘                        │
│                           │                                           │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                      Business Logic                             │ │
│  │  • Rate Limiting (ds_core)                                      │ │
│  │  • JWT Verification (ds_auth)                                   │ │
│  │  • Request Validation                                           │ │
│  │  • Model Provider Abstraction (ds_model)                        │ │
│  └────────────────────────────────────────────────────────────────┘ │
└──────┬────────────────┬────────────────┬────────────────────────────┘
       │                │                │
       │                │                │
       ▼                ▼                ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  PostgreSQL │  │    Redis    │  │   Ollama    │
│             │  │             │  │             │
│ • Users     │  │ • Rate      │  │ • Models    │
│ • Sessions  │  │   Limits    │  │ • Chat      │
│ • Messages  │  │ • Cache     │  │   Inference │
│ • Audit Log │  │             │  │             │
└─────────────┘  └─────────────┘  └─────────────┘
```

### Component Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                    DeeperSensor Workspace                     │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐ │
│  │  ds-api        │  │  ds-core       │  │  ds-model      │ │
│  │  (crate)       │  │  (crate)       │  │  (crate)       │ │
│  │                │  │                │  │                │ │
│  │ • HTTP Server  │  │ • Config       │  │ • Trait        │ │
│  │ • Routes       │  │ • Error Types  │  │ • Ollama Impl  │ │
│  │ • Middleware   │  │ • Rate Limit   │  │ • Streaming    │ │
│  │ • State        │  │   Logic        │  │                │ │
│  └────────┬───────┘  └────────┬───────┘  └────────┬───────┘ │
│           │                   │                   │          │
│           └───────────────────┼───────────────────┘          │
│                               │                              │
│                               ▼                              │
│                    ┌────────────────────┐                    │
│                    │  ds-auth (crate)   │                    │
│                    │                    │                    │
│                    │ • Password Hashing │                    │
│                    │ • JWT Generation   │                    │
│                    │ • Token Validation │                    │
│                    └────────────────────┘                    │
└──────────────────────────────────────────────────────────────┘
```

---

## Component Design

### Workspace Crates

#### 1. **ds-api** (`crates/api`)

**Responsibility:** HTTP surface layer

- **Entry Point:** `main.rs` - loads config, initializes tracing, builds router, starts server
- **App Router:** `app.rs` - constructs the Axum app with middleware layers
- **Routes:** `routes.rs` - endpoint definitions for auth, chat, models, health
- **State:** `state.rs` - shared application state (DB pool, config, model provider, rate limiters)
- **Middleware:** CORS, security headers, request ID, tracing spans, limits
- **Observability:** `observability.rs` - tracing initialization and formatting

**Dependencies:** `axum`, `tower`, `tower-http`, `tokio`, `tracing`

#### 2. **ds-core** (`crates/core`)

**Responsibility:** Core domain types and cross-cutting concerns

- **Config:** `config.rs` - unified configuration loader (env + .env files)
- **Errors:** `error.rs` - `ApiError` enum with HTTP status mapping
- **Rate Limiting:** Token bucket algorithm (in-memory with DashMap)

**Dependencies:** `config`, `dotenvy`, `thiserror`, `dashmap`

#### 3. **ds-model** (`crates/model`)

**Responsibility:** LLM provider abstraction

- **Trait:** `ModelProvider` - defines `list_models()`, `chat()`, `chat_stream()`
- **Ollama:** `OllamaClient` - HTTP client for Ollama API
- **Types:** `ChatRequest`, `ChatMessage`, `ChatChunk`, `ModelInfo`

**Dependencies:** `reqwest`, `async-trait`, `serde`, `futures-util`

#### 4. **ds-auth** (`crates/auth`)

**Responsibility:** Authentication and authorization

- **Password Hashing:** Argon2id with configurable parameters
- **JWT:** HS256 signing, access/refresh token generation
- **Token Verification:** Claim extraction and validation

**Dependencies:** `argon2`, `jsonwebtoken`, `uuid`, `chrono`

### Shared Dependencies

Centralized in workspace `Cargo.toml`:

```toml
[workspace.dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }
axum = { version = "0.7", features = ["macros", "json"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres"] }
# ... etc
```

---

## Data Flow

### Request Lifecycle

```
1. Client Request
   │
   ├─▶ [Nginx]
   │    ├─ TLS Termination
   │    ├─ Generate Request ID (if missing)
   │    ├─ Rate Limit Check (Nginx layer)
   │    ├─ Security Headers
   │    └─ Forward to API
   │
   ├─▶ [Axum Middleware Stack]
   │    ├─ Request ID Propagation
   │    ├─ Tracing Span Creation
   │    ├─ CORS Preflight Handling
   │    ├─ Request Size Validation
   │    └─ Concurrency Limits
   │
   ├─▶ [Route Handler]
   │    ├─ Extract State<AppState>
   │    ├─ Rate Limit Check (application layer)
   │    ├─ JWT Verification (if protected)
   │    ├─ Request Validation
   │    └─ Business Logic
   │
   ├─▶ [External Services]
   │    ├─ Database Query (sqlx)
   │    ├─ Redis Access (future)
   │    └─ Ollama API Call
   │
   └─▶ [Response]
        ├─ Serialize to JSON / SSE
        ├─ Add Response Headers
        ├─ Log Completion (tracing)
        └─ Return to Client
```

### Authentication Flow

```
┌──────────┐                                          ┌──────────┐
│  Client  │                                          │   API    │
└────┬─────┘                                          └────┬─────┘
     │                                                      │
     │  POST /v1/auth/signup                               │
     │  { email, password }                                │
     ├─────────────────────────────────────────────────────▶
     │                                                      │
     │                                        [Validate Input]
     │                                        [Hash Password (Argon2)]
     │                                        [Insert User (DB)]
     │                                                      │
     │  201 Created                                        │
     │  { id, email }                                      │
     ◀─────────────────────────────────────────────────────┤
     │                                                      │
     │  POST /v1/auth/login                                │
     │  { email, password }                                │
     ├─────────────────────────────────────────────────────▶
     │                                                      │
     │                                        [Lookup User (DB)]
     │                                        [Verify Password]
     │                                        [Generate JWT Access Token]
     │                                        [Generate Refresh Token]
     │                                                      │
     │  200 OK                                             │
     │  { access_token, refresh_token }                    │
     ◀─────────────────────────────────────────────────────┤
     │                                                      │
     │  POST /v1/chat                                      │
     │  Authorization: Bearer <access_token>               │
     ├─────────────────────────────────────────────────────▶
     │                                                      │
     │                                        [Verify JWT]
     │                                        [Extract Claims]
     │                                        [Authorize Request]
     │                                        [Process Chat]
     │                                                      │
     │  200 OK                                             │
     │  { ... chat response ... }                          │
     ◀─────────────────────────────────────────────────────┤
```

---

## Security Model

### Defense in Depth

**Layer 1: Network (Nginx)**
- TLS 1.3 (or 1.2 minimum)
- Strong cipher suites
- Rate limiting (per IP)
- Request size limits (2MB default)
- Security headers (HSTS, CSP, X-Frame-Options, etc.)

**Layer 2: Application (Axum)**
- CORS policy enforcement
- JWT verification middleware
- Request validation (email format, length limits)
- Rate limiting (per user + per IP)
- Input sanitization
- SQL injection protection (parameterized queries via sqlx)

**Layer 3: Authentication (ds-auth)**
- Argon2id password hashing (memory-hard, GPU-resistant)
- JWT with HS256 (future: RS256 for distributed systems)
- Short-lived access tokens (15 minutes default)
- Refresh token rotation

**Layer 4: Database**
- Least privilege principle (app-specific DB user)
- Connection pooling with limits
- No raw SQL construction
- Prepared statements only

**Layer 5: Container (Docker)**
- Non-root user (UID 65534)
- Read-only filesystem
- Dropped capabilities (`CAP_DROP: ALL`)
- No new privileges (`no-new-privileges:true`)

### Secrets Management

- **Development:** `.env` file (excluded from Git)
- **Production:** Environment variables from secret management systems
  - AWS Secrets Manager
  - HashiCorp Vault
  - Kubernetes Secrets
  - Docker Swarm Secrets

### Audit Logging

All security-relevant events are logged with structured fields:

```json
{
  "timestamp": "2025-09-29T12:34:56Z",
  "level": "WARN",
  "target": "api::routes::auth",
  "message": "Failed login attempt",
  "email": "user@example.com",
  "ip": "192.168.1.100",
  "request_id": "abc123"
}
```

---

## Scaling Strategy

### Horizontal Scaling

The API is **stateless** (except for in-memory rate limiters, which will migrate to Redis):

```
┌─────────────────────────────────────┐
│       Load Balancer (Nginx/ALB)     │
└──────────┬──────────┬───────────────┘
           │          │
    ┌──────▼───┐  ┌──▼───────┐  ┌───────────┐
    │  API-1   │  │  API-2   │  │   API-3   │
    └──────┬───┘  └──┬───────┘  └─────┬─────┘
           │         │                │
           └─────────┼────────────────┘
                     │
          ┌──────────▼──────────┐
          │   Shared Database   │
          │   (Postgres)        │
          └─────────────────────┘
```

**Scaling Considerations:**

1. **Database Connections:** Each instance maintains its own connection pool (configurable limit)
2. **Rate Limiting:** Move to Redis-backed token buckets for shared state
3. **Session Affinity:** Not required (stateless JWT)
4. **Shared Filesystem:** Not required (all state in DB)

### Vertical Scaling

Resource limits (docker-compose.prod.yml):

```yaml
api:
  deploy:
    resources:
      limits:
        cpus: '2.0'
        memory: 2G
      reservations:
        cpus: '0.5'
        memory: 512M
```

**Tuning Parameters:**
- Database connection pool size
- HTTP server concurrency limits
- Request size limits
- Rate limit buckets

### Database Scaling

**Read Replicas:** Use `sqlx` with read/write split (future enhancement)

```rust
struct AppState {
    write_pool: PgPool,
    read_pool: PgPool,
}
```

**Connection Pooling:** Already implemented via `sqlx::PgPool`

---

## Technology Stack

### Backend

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Language | Rust | 1.82+ | Systems programming, performance, safety |
| Runtime | Tokio | 1.x | Async I/O, multi-threaded executor |
| HTTP Framework | Axum | 0.7 | Web server, routing, middleware |
| Database | PostgreSQL | 16 | Relational data persistence |
| Cache | Redis | 7 | Rate limiting, sessions (future) |
| ORM | SQLx | 0.7 | Compile-time SQL verification |
| Serialization | Serde | 1.x | JSON encoding/decoding |
| Logging | Tracing | 0.1 | Structured logging, distributed tracing |
| Auth | Argon2, JWT | Latest | Password hashing, token-based auth |

### Infrastructure

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Container | Docker | 24.0+ | Application packaging |
| Orchestration | Docker Compose / K8s | Service management |
| Reverse Proxy | Nginx | 1.27 | TLS, rate limiting, load balancing |
| CI/CD | GitHub Actions | Automated testing, builds, deployments |
| Monitoring | Prometheus + Grafana | Metrics, dashboards |
| Logging | Loki (optional) | Log aggregation |

---

## Design Decisions

### Why Rust?

- **Performance:** Near-C performance with zero-cost abstractions
- **Safety:** Memory safety without garbage collection
- **Concurrency:** Fearless concurrency with ownership system
- **Tooling:** Cargo, rustfmt, clippy, excellent ecosystem

### Why Axum over Actix-Web?

- **Ecosystem Alignment:** Built on top of Tokio and Tower (industry standard)
- **Type Safety:** Leverages Rust's type system for compile-time correctness
- **Extractors:** Ergonomic request handling
- **Middleware:** Tower middleware ecosystem

### Why SQLx over Diesel?

- **Async Support:** Native async/await (Diesel is sync)
- **Compile-Time Verification:** SQL queries checked at compile time
- **Flexibility:** Raw SQL with type safety, less ORM magic

### Why JWT over Sessions?

- **Stateless:** No server-side session storage (easier to scale)
- **Distributed:** Works across multiple API instances
- **Standard:** Industry-standard token format (RFC 7519)
- **Tradeoff:** Cannot revoke tokens before expiry (mitigated with short TTL + refresh tokens)

### Why Server-Sent Events (SSE) over WebSockets?

- **Simplicity:** HTTP-based, easier to implement and debug
- **Proxying:** Works through standard HTTP proxies/load balancers
- **Reconnection:** Browser handles auto-reconnect
- **Tradeoff:** Unidirectional (server→client only)

### Future Enhancements

1. **Redis Integration:** Move rate limiting to Redis for shared state
2. **OpenTelemetry:** Distributed tracing across services
3. **Read Replicas:** Database scaling with read/write split
4. **GraphQL API:** Alternative to REST for complex queries
5. **gRPC:** Internal service-to-service communication
6. **Message Queue:** Async job processing (Kafka, RabbitMQ)

---

**Document Version:** 1.0.0  
**Last Review:** September 2025  
**Next Review:** December 2025
