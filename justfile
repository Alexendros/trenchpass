# Justfile · tareas comunes de TrenchPass.
# Instala: cargo install just
# Lista: just --list

set shell := ["bash", "-cu"]

default:
    @just --list

# ──────────────────────────────────────────── build & verify ──

check:
    cargo check --locked --all-targets

build:
    cargo build --locked

build-release:
    cargo build --locked --release

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

clippy:
    cargo clippy --all-targets --locked -- -D warnings

test:
    cargo test --locked --all-targets

deny:
    cargo deny check

audit:
    cargo audit

verify: fmt-check clippy test deny

# ──────────────────────────────────────────── local stack ──

stack-up:
    docker compose -f infra/dev/docker-compose.yml up -d

stack-down:
    docker compose -f infra/dev/docker-compose.yml down

stack-logs:
    docker compose -f infra/dev/docker-compose.yml logs -f

# ──────────────────────────────────────────── run ──

run:
    cargo run

run-release:
    cargo run --release

# ──────────────────────────────────────────── docker ──

docker-build:
    docker build -t ghcr.io/alexendros/trenchpass:dev .

docker-shell:
    docker run --rm -it --entrypoint /bin/sh ghcr.io/alexendros/trenchpass:dev

# ──────────────────────────────────────────── docs ──

third-party-notices:
    cargo about generate -c about.toml about.hbs > THIRD_PARTY_NOTICES.md

clean:
    cargo clean

# ──────────────────────────────────────────── release ──

release VERSION:
    @echo "→ release v{{VERSION}}"
    @grep -q "version = \"{{VERSION}}\"" Cargo.toml || (echo "Cargo.toml debe estar en {{VERSION}}" && exit 1)
    git tag -s "v{{VERSION}}" -m "release v{{VERSION}}"
    @echo "ahora: git push origin main v{{VERSION}}"
