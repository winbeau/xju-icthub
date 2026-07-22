#!/usr/bin/env bash
set -euo pipefail
umask 022

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

cd ../vendor/codex/codex-rs
cargo build --release --locked --bin codex --jobs "${ICTHUB_CODEX_BUILD_JOBS:-2}"
install -Dm0755 target/release/codex "$repo_dir/backend/tools/codex"

printf 'Production artifacts ready:\n'
printf '  frontend: %s/frontend/dist\n' "$repo_dir"
printf '  backend:  %s/backend/target/release/icthub-server\n' "$repo_dir"
printf '  worker:   %s/backend/target/release/icthub-import-worker\n' "$repo_dir"
printf '  Codex:    %s/backend/tools/codex\n' "$repo_dir"
