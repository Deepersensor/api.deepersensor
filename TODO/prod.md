# Production Readiness Checklist

Status legend: [ ] not started, [~] in progress, [x] done

## 1. Security / Auth
- [ ] Generate & provision 64-byte JWT secret via secret manager (no plaintext in repo)
- [ ] (Optional later) Migrate to asymmetric JWT (EdDSA/RS256)
- [ ] Add security headers middleware (HSTS, CSP, X-Content-Type-Options, Frame-Options, Referrer-Policy, Permissions-Policy)
- [ ] Explicit Argon2id parameters (memory, time, parallelism) and rehash on weaker hash
- [ ] Implement failed login throttle / temporary lockout
- [ ] Add audit log events (signup success/fail, login success/fail)
- [ ] Enforce strict email validation & normalize (lowercase)
- [ ] Add password strength validation (zxcvbn or heuristic)
- [ ] Refresh token flow with storage + revocation
- [ ] Token revocation / logout endpoint
- [ ] Reuse detection for refresh tokens

## 2. Configuration & Secrets
- [ ] Move prod config to secret manager (Vault/SSM/Secrets Manager)
- [ ] Fail startup if required secrets missing (no silent defaults in prod)
- [ ] Document rotation procedure for JWT secret & DB creds
- [ ] Pin base images by digest in Dockerfiles

## 3. Database
- [ ] Create dedicated least-privilege DB user (no superuser)
- [ ] Apply migrations automatically with rollback strategy
- [ ] Add conversation/message query indexes (conversation_id, created_at)
- [ ] Implement data retention or archival policy (messages)
- [ ] Automated backups + restore drill runbook
- [ ] Add connection pool metrics + saturation alerts

## 4. Rate Limiting & Abuse Prevention
- [ ] Endpoint-specific limits (auth endpoints tighter)
- [ ] Global IP burst breaker / anomaly detection
- [ ] Add per-user limits post-auth
- [ ] Instrument rate limit hits / rejections for monitoring

## 5. Observability
- [ ] Structured logging fields (request_id, user_id, ip)
- [ ] Add metrics: auth_success_total, auth_fail_total, db_pool_in_use, rate_limit_dropped_total, external_model_latency_histogram
- [ ] Optional: tracing exporter (OTLP) integration
- [ ] Alert rules (5xx %, latency p95, auth failures spike, pool saturation)

## 6. Networking / Infra
- [ ] HTTPS termination + HSTS (1y, includeSubDomains, preload)
- [ ] Confirm proxy sets X-Forwarded-Proto and trusted proxy list matches infra
- [ ] Private network for DB & Redis (no public exposure)
- [ ] Redis AUTH / move to managed service
- [ ] Firewall egress restrictions (allow only needed destinations)

## 7. Application Hardening
- [ ] Remove or guard any `unwrap()` on external inputs
- [ ] Enforce Content-Type=application/json for JSON routes
- [ ] Reject bodies > MAX_REQUEST_SIZE_BYTES early
- [ ] SSE / streaming: cap concurrent streams per client
- [ ] Graceful shutdown verified under load
- [ ] Add panic hook to log & increment metric

## 8. Dependency & Supply Chain
- [ ] Add `cargo audit` in CI
- [ ] Add `cargo deny` (licenses, advisories, bans)
- [ ] Generate SBOM (cargo sbom or syft) per release
- [ ] Image vulnerability scan (Trivy/Grype) gating deploy
- [ ] Track base image CVEs & rebuild cadence

## 9. Performance & Load
- [ ] Load test baseline (auth + chat endpoints) p50/p95 latency
- [ ] Tune Pg pool size & timeout based on load test
- [ ] Benchmark Argon2 parameters vs. latency budget
- [ ] Evaluate horizontal scaling strategy (stateless + sticky where needed)

## 10. Operational Runbooks
- [ ] Incident response playbook (where logs, metrics, traces)
- [ ] Deployment / rollback procedure
- [ ] Secret rotation steps
- [ ] Backup restore test steps
- [ ] On-call escalation matrix

## 11. Release & Versioning
- [ ] Embed Git SHA at build time
- [ ] Tag releases and generate changelog
- [ ] Immutable image tags (git SHA) in deploy manifests

## 12. Future Enhancements
- [ ] Asymmetric JWT & key rotation
- [ ] Distributed cache / rate limit cluster coordination
- [ ] Regional failover strategy
- [ ] DDoS mitigation / WAF integration

---
Initial high-priority focus items for next sprint:
1. Security headers middleware
2. Argon2 parameter enforcement + rehash detection
3. Audit logging for auth flows
4. Login/signup throttling (per-IP + exponential backoff)
5. JWT secret generation & secret manager integration
