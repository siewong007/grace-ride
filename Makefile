# 恩典同行 Grace Ride — common developer, CI & operations tasks.
CARGO ?= ./dev-env.sh cargo
BACKEND_CARGO = $(patsubst ./%,../%,$(CARGO))

.DEFAULT_GOAL := help

.PHONY: help dev-api dev-web db-up db-down fmt lint lint-android test test-api test-web test-android test-ios test-e2e audit build image up deploy-aic backup restore restore-drill osrm-setup capacity

help: ## Show available Make targets
	@grep -E '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

dev-api: ## Run the API in dev mode with demo fixtures (:8080)
	cd backend && $(BACKEND_CARGO) run

dev-web: ## Run the Vite frontend dev server (:5173)
	cd frontend && npm run dev

db-up: ## Boot local PostgreSQL 18 dev & test databases via Docker Compose
	docker compose -f docker-compose.dev.yml up -d --wait

db-down: ## Stop local PostgreSQL 18 Docker Compose stack
	docker compose -f docker-compose.dev.yml down

fmt: ## Format Rust backend source code
	cd backend && $(BACKEND_CARGO) fmt

lint: ## Run Rust fmt/clippy (--locked) and frontend ESLint
	cd backend && $(BACKEND_CARGO) fmt --check && $(BACKEND_CARGO) clippy --all-targets --locked -- -D warnings
	cd frontend && npm run lint

lint-android: ## Run Android Lint check
	cd android && ./gradlew :app:lintDebug --no-daemon

test: ## Run backend Cargo tests + frontend typecheck & Vitest suites
	cd backend && $(BACKEND_CARGO) test --locked
	cd frontend && npm run typecheck && npm test

test-api: ## Run backend unit + integration tests (--locked)
	cd backend && $(BACKEND_CARGO) test --locked

test-web: ## Run frontend TypeScript typecheck + Vitest suites
	cd frontend && npm run typecheck && npm test

test-android: ## Run Android unit tests and compile instrumentation tests
	cd android && ./gradlew :app:testDebugUnitTest :app:compileDebugAndroidTestKotlin --no-daemon

test-ios: ## Run GraceRideKit Swift package unit tests and build unsigned iOS Simulator target
	DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer" swift test --package-path ios/GraceRideKit && \
	DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer" xcodebuild -project ios/GraceRide.xcodeproj -scheme GraceRide -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO build

test-e2e: ## Run Playwright browser end-to-end suites
	cd frontend && npm run test:e2e

audit: ## Scan backend and production frontend dependencies for advisories
	cd backend && $(BACKEND_CARGO) audit
	cd frontend && npm audit --omit=dev --audit-level=high

build: ## Build production frontend bundle + release Rust binary
	cd frontend && npm run build
	cd backend && $(BACKEND_CARGO) build --release --locked

image: ## Build production container image (grace-ride:latest)
	docker build -t grace-ride:latest .

up: ## Start production Docker Compose stack (requires .env.production)
	docker compose --env-file .env.production up -d --build

deploy-aic: ## Build linux/amd64 image and roll directly onto the AIC VPS (no GitHub Actions)
	./scripts/deploy_aic.sh

backup: ## Snapshot database + receipts into ./backups
	@set -a; \
	prod_env=0; \
	if [ -z "$${DATABASE_URL:-}" ] && [ -z "$${PG_CONTAINER:-}" ] && [ -z "$${DOCKER_DB_CONTAINER:-}" ]; then \
		if [ -f .env.production ]; then . ./.env.production; prod_env=1; \
		elif [ -f backend/.env ]; then . ./backend/.env; fi; \
	fi; \
	if [ "$$prod_env" = "1" ] && [ -z "$${REQUIRE_ENCRYPTION:-}" ]; then \
		export REQUIRE_ENCRYPTION=true; \
	fi; \
	set +a; \
	./scripts/backup.sh

restore: ## Restore DB + receipts (ARCHIVE=backups/grace-ride-YYYYMMDD-HHMMSS.tar.gz)
	@test -n "$(ARCHIVE)" || { echo "Usage: make restore ARCHIVE=backups/grace-ride-YYYYMMDD-HHMMSS.tar.gz" >&2; exit 1; }
	@set -a; \
	if [ -z "$${DATABASE_URL:-}" ] && [ -z "$${PG_CONTAINER:-}" ] && [ -z "$${DOCKER_DB_CONTAINER:-}" ]; then \
		if [ -f .env.production ]; then . ./.env.production; \
		elif [ -f backend/.env ]; then . ./backend/.env; fi; \
	fi; \
	set +a; \
	./scripts/restore.sh "$(ARCHIVE)"

restore-drill: ## Non-destructive restore & upload-reconciliation drill (ARCHIVE=...)
	@test -n "$(ARCHIVE)" || { echo "Usage: make restore-drill ARCHIVE=backups/grace-ride-YYYYMMDD-HHMMSS.tar.gz" >&2; exit 1; }
	./scripts/restore_drill.sh "$(ARCHIVE)"

osrm-setup: ## Download & build self-hosted Malaysia OSRM routing graph (OSRM_DATA_DIR=./osrm-data)
	./scripts/setup_osrm_malaysia.sh "$${OSRM_DATA_DIR:-./osrm-data}"

capacity: ## Run 1,000-member / 10-driver capacity baseline suite
	./scripts/capacity_baseline.sh
