# ==============================================================================
# maxima-rs WebUI & CAS Engine Multi-Stage Dockerfile
#
# Production standalone container for offline air-gapped environments, with
# optional AWS Lambda Web Adapter support for containerized Lambda execution.
#
# All runtime configuration is driven strictly via CLI flags and configuration
# files (zero reliance on application environment variables).
# ==============================================================================

# ── Stage 1: Build & Optimization ─────────────────────────────────────────────
FROM rust:1.88-bookworm AS builder

WORKDIR /usr/src/maxima

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./
COPY sym-core/Cargo.toml sym-core/
COPY sym-macros/Cargo.toml sym-macros/
COPY maxima-rs/Cargo.toml maxima-rs/
COPY maxima-web/Cargo.toml maxima-web/
COPY tests/Cargo.toml tests/

# Copy workspace sources
COPY sym-core/src sym-core/src
COPY sym-macros/src sym-macros/src
COPY maxima-rs/src maxima-rs/src
COPY maxima-web/src maxima-web/src
COPY maxima-web/static maxima-web/static
COPY tests/ tests/

# Compile production release binary with thin LTO & stripped symbols
RUN cargo build --release --locked -p maxima-web

# ── Stage 2: Minimal Runtime ──────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies & certificates
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy AWS Lambda Web Adapter extension (used automatically when executed in AWS Lambda)
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:0.8.4 /lambda-adapter /opt/extensions/lambda-adapter

# Copy the compiled standalone executable (embedded assets include KaTeX, fonts, CSS, JS)
COPY --from=builder /usr/src/maxima/target/release/maxima-web /app/maxima-web

# Set up non-root execution user for security
RUN groupadd -g 1001 maxima && \
    useradd -u 1001 -g maxima -m -s /bin/bash maxima && \
    chown -R maxima:maxima /app

USER maxima
WORKDIR /app

# AWS Lambda Web Adapter readiness & port alignment (infrastructure-only)
ENV AWS_LWA_READINESS_CHECK_PATH=/api/health
ENV AWS_LWA_PORT=8080

# Expose default HTTP port (8080 for both standalone & Lambda Web Adapter)
EXPOSE 8080

# Explicit CLI entrypoint and parameters
ENTRYPOINT ["/app/maxima-web"]
CMD ["--bind", "0.0.0.0", "--port", "8080", "--timeout-ms", "8000"]
