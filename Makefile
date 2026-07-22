.PHONY: setup dev test check frontend-check backend-check

setup:
	cd frontend && pnpm install --frozen-lockfile
	cd backend && cargo fetch

dev:
	@echo "Run 'pnpm dev' in frontend/ and 'cargo run -p icthub-server' in backend/."

test:
	cd frontend && pnpm test:run
	cd backend && cargo test --workspace

check: frontend-check backend-check

frontend-check:
	cd frontend && pnpm typecheck
	cd frontend && pnpm lint
	cd frontend && pnpm test:run
	cd frontend && pnpm build

backend-check:
	cd backend && cargo fmt --all --check
	cd backend && cargo clippy --workspace --all-targets --all-features -- -D warnings
	cd backend && cargo test --workspace
