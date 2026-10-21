# ---------------------------------------------------------------------------
# Build stage
# ---------------------------------------------------------------------------
FROM rust:1-slim AS builder

RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache the dependency tree.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
 && echo 'fn main() {}' > src/main.rs \
 && echo '' > src/lib.rs \
 && cargo build --release \
 && rm -rf src

# Copy the real sources and build.
COPY . .
RUN touch src/main.rs src/lib.rs \
 && cargo build --release \
 && strip target/release/personal-site

# ---------------------------------------------------------------------------
# Runtime stage
# ---------------------------------------------------------------------------
FROM debian:stable-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --create-home --uid 10001 site

WORKDIR /app

COPY --from=builder /build/target/release/personal-site /usr/local/bin/personal-site

RUN mkdir -p /app/data /app/media && chown -R site:site /app

USER site

ENV APP_ENV=production \
    HOST=0.0.0.0 \
    PORT=3000 \
    DATABASE_URL="sqlite:///app/data/personal-site.db?mode=rwc" \
    MEDIA_DIR=/app/media

EXPOSE 3000
VOLUME ["/app/data", "/app/media"]

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -fsS http://127.0.0.1:3000/robots.txt || exit 1

ENTRYPOINT ["personal-site"]
