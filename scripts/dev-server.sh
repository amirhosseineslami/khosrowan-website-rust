#!/usr/bin/env bash
# Launch the personal-site dev server for browser preview.
cd "$(dirname "$0")/.."
export SERVER_PORT="${SERVER_PORT:-8788}"
export DATABASE_URL="sqlite://data/personal-site.db?mode=rwc"
exec ./target/debug/personal-site
