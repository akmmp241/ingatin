# ── Stage 1: dependency cache ─────────────────────────────────────────────────
FROM rust:1.94.1-slim-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 2: build ────────────────────────────────────────────────────────────
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cache dependencies layer separately — only rebuilt when Cargo.toml/lock changes
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin ingatin

# ── Stage 3: minimal runtime ──────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12 AS runtime
# distroless/cc includes libc + libgcc — enough for most Rust binaries.
# Switch to debian:bookworm-slim if you need a shell for debugging.

WORKDIR /app
COPY --from=builder /app/target/release/ingatin /app/ingatin

# Never run as root in production
USER nonroot:nonroot

EXPOSE 3000
ENTRYPOINT ["/app/ingatin"]