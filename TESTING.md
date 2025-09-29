# Testing Guide - DeeperSensor API

This document describes how to run tests for the DeeperSensor API.

## Overview

The test suite includes:
- **Unit tests**: Small, focused tests for individual functions (validation, utilities)
- **Integration tests**: End-to-end tests of HTTP endpoints with a real database
- **Build verification**: Clippy lints and cargo checks via CI/CD

## Prerequisites

### For Unit Tests
No special setup required. Unit tests run in isolation:

```bash
cargo test --lib
```

### For Integration Tests

Integration tests require a PostgreSQL database. You have two options:

#### Option 1: Use Docker Compose (Recommended)

Start a test database with Docker:

```bash
# Start PostgreSQL (detached)
docker compose up -d postgres

# Wait for PostgreSQL to be ready
sleep 3

# Set environment variable for tests
export TEST_DATABASE_URL="postgresql://deepersensor:devpassword@localhost:5432/deepersensor"

# Run integration tests
cargo test --test integration_tests -- --test-threads=1
```

#### Option 2: Use Existing PostgreSQL

If you have PostgreSQL running locally:

```bash
# Create a test database
createdb deepersensor_test

# Export the connection URL
export TEST_DATABASE_URL="postgresql://username:password@localhost/deepersensor_test"

# Run integration tests
cargo test --test integration_tests -- --test-threads=1
```

> **Note**: Integration tests use `--test-threads=1` to avoid database conflicts between parallel tests.

## Running All Tests

### Quick Test (Unit Tests Only)

```bash
cargo test --workspace --lib
```

This runs all unit tests across all workspace crates without needing external dependencies.

### Full Test Suite (Unit + Integration)

```bash
# Start services
docker compose up -d postgres

# Set test database URL
export TEST_DATABASE_URL="postgresql://deepersensor:devpassword@localhost:5432/deepersensor"

# Run all tests
cargo test --workspace -- --test-threads=1
```

## Test Organization

### Unit Tests

Located within each module using `#[cfg(test)]`:

- **`crates/api/src/validation.rs`**: Email, password, model name, message content validators
- **`crates/auth/src/lib.rs`**: Password hashing, JWT generation/verification
- **`crates/core/src/error.rs`**: Error handling and serialization

Run unit tests for a specific crate:

```bash
cargo test -p api --lib
cargo test -p ds-auth --lib
cargo test -p ds-core --lib
```

### Integration Tests

Located in `crates/api/tests/`:

- **`integration_tests.rs`**: HTTP endpoint tests (signup, login, health, metrics)

Run integration tests:

```bash
export TEST_DATABASE_URL="postgresql://deepersensor:devpassword@localhost:5432/deepersensor"
cargo test --test integration_tests -- --test-threads=1
```

Test coverage includes:
- ✅ Health endpoint returns 200 OK with dependency status
- ✅ Readiness endpoint returns 200 OK
- ✅ Signup with valid credentials succeeds
- ✅ Signup with duplicate email returns 422 UNPROCESSABLE_ENTITY
- ✅ Signup with weak password returns 422 UNPROCESSABLE_ENTITY
- ✅ Login with correct credentials succeeds
- ✅ Login with wrong password returns 401 UNAUTHORIZED
- ✅ Metrics endpoint returns Prometheus format

## Continuous Integration

GitHub Actions automatically runs tests on every push and pull request:

### CI Workflow (`.github/workflows/ci.yml`)

```yaml
# Automated testing includes:
- cargo fmt -- --check           # Code formatting
- cargo clippy -- -D warnings    # Linting
- cargo test --workspace         # All tests
- cargo audit                    # Security vulnerabilities
```

View CI results: https://github.com/your-org/api.deepersensor/actions

## Test Database Cleanup

Integration tests automatically clean up test data using:

```rust
async fn cleanup_test_db(pool: &sqlx::PgPool) -> Result<()> {
    sqlx::query("TRUNCATE TABLE users CASCADE")
        .execute(pool)
        .await?;
    Ok(())
}
```

Each test calls `cleanup_test_db()` at the end to ensure isolation.

## Writing New Tests

### Adding a Unit Test

Add to the appropriate module's `#[cfg(test)]` section:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_function() {
        let result = your_function("input");
        assert_eq!(result, expected_value);
    }
}
```

### Adding an Integration Test

Add to `crates/api/tests/integration_tests.rs`:

```rust
#[tokio::test]
async fn test_new_endpoint() -> Result<()> {
    let (_cfg, state, router) = setup_test_app().await?;
    
    let response = router
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .uri("/your/endpoint")
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    cleanup_test_db(&state.db).await?;
    Ok(())
}
```

## Test Coverage Goals

Current coverage focus areas:
- ✅ Input validation (email, password, model names, messages)
- ✅ Authentication flow (signup, login)
- ✅ Health monitoring endpoints
- ⚠️ JWT-protected endpoints (TODO: add after applying middleware)
- ⚠️ Rate limiting (TODO: add integration tests)
- ⚠️ Chat streaming (TODO: requires Ollama mock)

## Debugging Failed Tests

### View Test Output

```bash
# Show all test output (not just failures)
cargo test -- --nocapture

# Run a specific test
cargo test test_signup_success -- --nocapture

# Show test backtraces
RUST_BACKTRACE=1 cargo test
```

### Database Issues

If integration tests fail with database errors:

```bash
# Check PostgreSQL is running
docker compose ps postgres

# View PostgreSQL logs
docker compose logs postgres

# Reset the database
docker compose down -v
docker compose up -d postgres
```

### SQL Migration Errors

If migrations fail in integration tests:

```bash
# Manually run migrations
sqlx migrate run --database-url "postgresql://deepersensor:devpassword@localhost:5432/deepersensor"

# Check migration status
sqlx migrate info --database-url "postgresql://deepersensor:devpassword@localhost:5432/deepersensor"
```

## Performance Testing

For performance and load testing, see `DEPLOYMENT.md` section on "Performance Testing".

Integration tests focus on correctness, not performance.

## Security Testing

Run security audit as part of testing:

```bash
cargo audit
```

This checks for known vulnerabilities in dependencies (automated in CI).

## Environment Variables for Tests

Tests respect these environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `TEST_DATABASE_URL` | (required) | PostgreSQL connection string for integration tests |
| `RUST_LOG` | `info` | Log level for test output |
| `RUST_BACKTRACE` | `0` | Set to `1` or `full` for detailed error traces |

Example test run with full logging:

```bash
RUST_LOG=debug \
RUST_BACKTRACE=1 \
TEST_DATABASE_URL="postgresql://deepersensor:devpassword@localhost:5432/deepersensor" \
cargo test --test integration_tests -- --nocapture --test-threads=1
```

## Next Steps

After the core API is stable:

1. **Add JWT-protected route tests**: Apply `auth_middleware::require_auth` to chat endpoints, test with valid/invalid tokens
2. **Add rate limiting tests**: Verify rate limits trigger correctly
3. **Add chat streaming tests**: Mock Ollama responses or use test instance
4. **Add load tests**: Use `criterion` for benchmarking critical paths
5. **Add property-based tests**: Use `proptest` for validation fuzzing

## Resources

- [Axum Testing Guide](https://docs.rs/axum/latest/axum/test_helpers/index.html)
- [SQLx Testing](https://github.com/launchbadge/sqlx#testing)
- [Tokio Test Documentation](https://docs.rs/tokio/latest/tokio/attr.test.html)
