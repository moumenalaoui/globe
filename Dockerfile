# Multi-stage build for Railway.
#
# A Dockerfile is required rather than optional here: Railway's buildpack
# detects the language from root-level files, and this repo's root holds only
# `backend/` and `frontend/` — no root Cargo.toml or package.json, so there is
# nothing to detect. It also could not work even with a root directory override,
# because `frontend/dist` is gitignored and must therefore be built inside the
# image, which a Rust buildpack will not do.
#
# Final layout, which the STATIC_DIR / SEED_DIR env vars must match exactly:
#   /app/blackout        the server binary
#   /app/dist            the built SPA          -> STATIC_DIR=/app/dist
#   /app/data/seed       committed seed files   -> SEED_DIR=/app/data/seed
# The SQLite database lives on the mounted volume, NOT in the image:
#   /data/mena_ai.db                            -> DATABASE_PATH=/data/mena_ai.db


# ---------------------------------------------------------------------------
# Stage 1 — build the SPA.
#
# Vite 5 needs Node 18+; 22 is the current LTS. `npm ci` (not `install`) so the
# committed package-lock.json is authoritative.
# ---------------------------------------------------------------------------
FROM node:22-bookworm-slim AS frontend

WORKDIR /build

# Dependencies first: this layer is cached until the lockfile actually changes,
# which is what keeps iteration on source cheap.
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
# Output is ~16 MB, ~15 MB of it the Cesium assets that vite-plugin-cesium
# copies in. No build-time env is required: VITE_CESIUM_ION_TOKEN is read in
# Globe.jsx but the viewer runs with `baseLayer: false` and no terrain
# provider, so no Ion asset is ever requested and an unset token is harmless.
RUN npm run build


# ---------------------------------------------------------------------------
# Stage 2 — build the server.
#
# Rust is pinned because backend/Cargo.toml declares `edition = "2024"`, which
# needs Rust >= 1.85 — an older default toolchain fails to compile at all. Bump
# this tag freely; do not drop below 1.85.
# ---------------------------------------------------------------------------
FROM rust:1.90-slim-bookworm AS backend

# Two native dependencies, both confirmed from Cargo.lock rather than assumed:
#   - libsqlite3-sys is used via rusqlite's `bundled` feature, so SQLite is
#     compiled from C source and needs a C toolchain.
#   - reqwest 0.12 with default features resolves to native-tls -> openssl-sys,
#     so OpenSSL headers are needed here (and libssl3 at runtime, below).
#     Switching reqwest to rustls-tls would remove both, but that is a
#     dependency change, not a packaging one.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Dependencies are compiled against a stub binary first, so the 185-crate
# release build lands in its own layer keyed only on Cargo.toml/Cargo.lock.
# Without this split, editing one line of Rust would rebuild every dependency
# from scratch on every deploy.
#
# --locked: Cargo.lock is committed, so the dependency set is reproducible and
# a drifted lockfile fails loudly instead of resolving something newer.
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY backend/src ./src
COPY backend/data ./data
# `touch` so cargo does not mistake the real main.rs for the already-compiled
# stub — mtime is what invalidates its fingerprint.
RUN touch src/main.rs && cargo build --release --locked


# ---------------------------------------------------------------------------
# Stage 3 — runtime.
#
# Neither toolchain ships in the final image; only the artefacts do.
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# ca-certificates is not optional: every fetcher talks HTTPS to OONI, IODA,
# Tor Metrics, Cloudflare, OWID and Pulse, and without a trust store all of
# them fail at runtime while the process itself looks healthy. libssl3 is the
# runtime half of the openssl-sys link above — the binary will not start
# without it.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend  /build/target/release/backend  /app/blackout
COPY --from=backend  /build/data/seed               /app/data/seed
COPY --from=frontend /build/dist                    /app/dist

# Run unprivileged. The volume is mounted at /data, so that is the only path
# this user needs to write to — chown it at build time so first boot can create
# the database there.
RUN useradd --system --uid 10001 --create-home blackout \
    && mkdir -p /data \
    && chown -R blackout:blackout /data /app
USER blackout

# Defaults baked in so the image is correct even if a dashboard variable is
# forgotten. Every one of these is still overridable at runtime.
#   SEED_DIR / STATIC_DIR are absolute here on purpose: their code defaults are
#   relative to the process working directory ("data/seed", "../frontend/dist"),
#   which is a laptop-shaped assumption. A wrong SEED_DIR is fatal — the seed
#   load propagates through init_schema into main, so the container crash-loops
#   rather than starting degraded.
ENV SEED_DIR=/app/data/seed \
    STATIC_DIR=/app/dist \
    DATABASE_PATH=/data/mena_ai.db

# Documentation only — Railway injects PORT and the app reads it, falling back
# to 3001 solely for local runs. Do not set PORT yourself on Railway.
EXPOSE 3001

CMD ["/app/blackout"]
