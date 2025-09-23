# syntax=docker/dockerfile:1.7
ARG RUST_VERSION=1.81
FROM rust:${RUST_VERSION}-slim AS builder
WORKDIR /app
# Create empty dirs to leverage caching of deps
COPY Cargo.toml Cargo.lock ../
COPY api/Cargo.toml api/Cargo.toml
COPY crates/core/Cargo.toml crates/core/Cargo.toml
COPY crates/model/Cargo.toml crates/model/Cargo.toml
COPY crates/auth/Cargo.toml crates/auth/Cargo.toml

# Dummy build to cache dependencies
RUN mkdir -p api/src crates/core/src crates/model/src crates/auth/src \
 && echo 'fn main(){}' > api/src/main.rs \
 && echo '' > crates/core/src/lib.rs \
 && echo '' > crates/model/src/lib.rs \
 && echo '' > crates/auth/src/lib.rs \
 && cargo build --release -p api || true

# Copy real sources
COPY . .
RUN cargo build --release -p api

FROM gcr.io/distroless/cc-debian12 AS runtime
ENV APP_ENV=production
WORKDIR /app
COPY --from=builder /app/target/release/api /app/api
USER 65534:65534
EXPOSE 8080
ENTRYPOINT ["/app/api"]
