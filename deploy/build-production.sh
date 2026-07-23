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
codex_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
original_lock="$(mktemp)"
cp Cargo.lock "$original_lock"
restore_codex_lock() {
    cp "$original_lock" Cargo.lock
    rm -f "$original_lock"
}
trap restore_codex_lock EXIT

# The upstream release commit keeps workspace packages as 0.0.0 in Cargo.lock while
# Cargo.toml carries the release version. Normalize only those workspace version lines,
# reject every other lock-file change, then build with --locked.
cargo update --workspace
unexpected_lock_diff="$(
    git diff --unified=0 -- Cargo.lock \
        | grep -E '^[+-]' \
        | awk -v version="$codex_version" '
            /^(---|\+\+\+)/ { next }
            $0 == "-version = \"0.0.0\"" { next }
            $0 == "+version = \"" version "\"" { next }
            { print }
        ' \
        || true
)"
if [[ -n "$unexpected_lock_diff" ]]; then
    printf 'Unexpected Codex Cargo.lock changes:\n%s\n' "$unexpected_lock_diff" >&2
    exit 1
fi
printf 'Normalized Codex Cargo.lock SHA-256: '
sha256sum Cargo.lock | cut -d' ' -f1
# The production host has 8 GiB of RAM and no swap. Building Codex with two
# release/LTO rustc processes can exceed that limit and be killed by the OOM
# killer, so keep the safe default serial while still allowing larger builders
# to opt in to more jobs explicitly.
cargo build --release --locked --bin codex --jobs "${ICTHUB_CODEX_BUILD_JOBS:-1}"
install -Dm0755 target/release/codex "$repo_dir/backend/tools/codex"
restore_codex_lock
trap - EXIT

printf 'Production artifacts ready:\n'
printf '  frontend: %s/frontend/dist\n' "$repo_dir"
printf '  backend:  %s/backend/target/release/icthub-server\n' "$repo_dir"
printf '  worker:   %s/backend/target/release/icthub-import-worker\n' "$repo_dir"
printf '  Codex:    %s/backend/tools/codex\n' "$repo_dir"
