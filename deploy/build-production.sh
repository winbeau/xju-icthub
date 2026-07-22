#!/usr/bin/env bash
set -euo pipefail

repo_dir="${1:-/home/winbeau/xju-icthub}"

cd "$repo_dir"
git pull --ff-only
git submodule update --init --depth 1 vendor/codex

cd frontend
pnpm install --frozen-lockfile
pnpm build

cd ../backend
mkdir -p data uploads
cargo build --release --locked

printf 'Production artifacts ready:\n'
printf '  frontend: %s/frontend/dist\n' "$repo_dir"
printf '  backend:  %s/backend/target/release/icthub-server\n' "$repo_dir"
printf '  worker:   %s/backend/target/release/icthub-import-worker\n' "$repo_dir"
