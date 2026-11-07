# 恩典同行 Grace Ride — common tasks. Rust runs via the SSD toolchain wrapper.
CARGO := ./dev-env.sh cargo

.PHONY: dev-api dev-web test lint audit build image up backup

dev-api:            ## Run the API (dev mode, demo data)
	cd backend && ../$(CARGO) run
dev-web:            ## Run the Vite dev server on :5173
	cd frontend && npm run dev
test:               ## Backend unit + integration tests, frontend type-check
	cd backend && ../$(CARGO) test
	cd frontend && npx tsc -b
lint:               ## Formatting and clippy (what CI enforces)
	cd backend && ../$(CARGO) fmt --check && ../$(CARGO) clippy --all-targets -- -D warnings
audit:              ## Known-vulnerability scan for both stacks
	cd backend && ../$(CARGO) audit
	cd frontend && npm audit --audit-level=high
build:              ## Release binary + production web bundle
	cd frontend && npm run build
	cd backend && ../$(CARGO) build --release
image:              ## Production container image
	docker build -t grace-ride:latest .
up:                 ## Production stack (needs .env.production)
	docker compose --env-file .env.production up -d --build
backup:             ## Snapshot DB + receipts into ./backups
	scripts/backup.sh
