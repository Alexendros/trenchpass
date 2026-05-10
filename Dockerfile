# syntax=docker/dockerfile:1.7

# ─── builder ──────────────────────────────────────────────────────────────────
FROM rust:1.95-bookworm AS builder

# Dependencias de sistema para tonic/grpc/openssl si alguna crate lo necesita.
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev protobuf-compiler ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache de dependencias: copia solo manifests para que cargo descargue/compile
# crates aunque el código fuente cambie.
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release || true
RUN rm -rf src target/release/trenchpass target/release/deps/trenchpass*

# Build real · `migrations/` necesario en compile-time (sqlx::migrate! valida el path).
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

# ─── runtime ──────────────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12 AS runtime

LABEL org.opencontainers.image.source="https://github.com/Alexendros/trenchpass"
LABEL org.opencontainers.image.description="TrenchPass · MCP gateway custodio único de credenciales"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"

COPY --from=builder /build/target/release/trenchpass /trenchpass

EXPOSE 8300
USER nonroot:nonroot
ENTRYPOINT ["/trenchpass"]
