# =============================================================================
# Makefile for Hydra Gateway
# REST API and WebSocket gateway for the hydra-amm engine
# =============================================================================

# Detect current branch
CURRENT_BRANCH := $(shell git rev-parse --abbrev-ref HEAD)

# Project name
PROJECT_NAME := hydra-gateway

# Docker image name
DOCKER_IMAGE := hydra-gateway

# =============================================================================
# Default target
# =============================================================================
.PHONY: all
all: fmt lint test build

# =============================================================================
# Build
# =============================================================================

.PHONY: build
build:
	@echo "Building debug version..."
	cargo build

.PHONY: release
release:
	@echo "Building release version..."
	cargo build --release

.PHONY: clean
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# =============================================================================
# Test & Quality
# =============================================================================

.PHONY: test
test:
	@echo "Running all tests..."
	RUST_LOG=warn cargo test

.PHONY: test-lib
test-lib:
	@echo "Running library tests..."
	RUST_LOG=warn cargo test --lib

.PHONY: test-doc
test-doc:
	@echo "Running documentation tests..."
	cargo test --doc

.PHONY: fmt
fmt:
	@echo "Formatting code..."
	cargo +stable fmt --all

.PHONY: fmt-check
fmt-check:
	@echo "Checking code formatting..."
	cargo +stable fmt --all --check

.PHONY: lint
lint:
	@echo "Running clippy lints..."
	cargo clippy --all-targets -- -D warnings

.PHONY: lint-fix
lint-fix:
	@echo "Auto-fixing lint issues..."
	cargo clippy --fix --all-targets --allow-dirty --allow-staged -- -D warnings

.PHONY: fix
fix:
	@echo "Applying cargo fix suggestions..."
	cargo fix --allow-staged --allow-dirty

.PHONY: check
check: fmt-check lint test
	@echo "All checks passed!"

.PHONY: pre-push
pre-push: fix fmt lint-fix test doc
	@echo "All pre-push checks passed!"

# =============================================================================
# Documentation
# =============================================================================

.PHONY: doc
doc:
	@echo "Generating documentation..."
	cargo doc --no-deps --document-private-items

.PHONY: doc-open
doc-open:
	@echo "Opening documentation in browser..."
	cargo doc --no-deps --open

.PHONY: doc-check
doc-check:
	@echo "Checking documentation builds without warnings..."
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# =============================================================================
# Docker
# =============================================================================

.PHONY: docker-build
docker-build:
	@echo "Building Docker image..."
	docker build -f Docker/Dockerfile -t $(DOCKER_IMAGE) .

.PHONY: docker-up
docker-up:
	@echo "Starting services..."
	docker compose -f Docker/docker-compose.yml up -d

.PHONY: docker-down
docker-down:
	@echo "Stopping services..."
	docker compose -f Docker/docker-compose.yml down

.PHONY: docker-logs
docker-logs:
	@echo "Tailing service logs..."
	docker compose -f Docker/docker-compose.yml logs -f

.PHONY: docker-clean
docker-clean:
	@echo "Stopping services and removing volumes..."
	docker compose -f Docker/docker-compose.yml down -v

# =============================================================================
# Coverage
# =============================================================================

.PHONY: coverage
coverage:
	@echo "Generating code coverage report (XML)..."
	@command -v cargo-tarpaulin > /dev/null || cargo install cargo-tarpaulin
	@mkdir -p coverage
	RUST_LOG=warn cargo tarpaulin --verbose --timeout 120 --out xml --output-dir coverage

.PHONY: coverage-html
coverage-html:
	@echo "Generating HTML coverage report..."
	@command -v cargo-tarpaulin > /dev/null || cargo install cargo-tarpaulin
	@mkdir -p coverage
	RUST_LOG=warn cargo tarpaulin --timeout 120 --out html --output-dir coverage

.PHONY: open-coverage
open-coverage:
	@echo "Opening coverage report..."
	open coverage/tarpaulin-report.html

# =============================================================================
# Git & Helpers
# =============================================================================

.PHONY: git-log
git-log:
	@if [ "$(CURRENT_BRANCH)" = "HEAD" ]; then \
		echo "You are in a detached HEAD state. Please check out a branch."; \
		exit 1; \
	fi; \
	echo "Showing git log for branch $(CURRENT_BRANCH) against main:"; \
	git log main..$(CURRENT_BRANCH) --pretty=full

.PHONY: check-spanish
check-spanish:
	@echo "Checking for Spanish words in code..."
	@rg -n --pcre2 -e '^\s*(//|///|//!|#|/\*|\*).*?[áéíóúÁÉÍÓÚñÑ¿¡]' \
		--glob '!target/*' \
		--glob '!**/*.png' \
		. && (echo "Spanish comments found"; exit 1) || echo "No Spanish comments found"

.PHONY: tree
tree:
	@echo "Project structure:"
	@tree -I 'target|.git|node_modules|coverage|dist' -L 3

.PHONY: loc
loc:
	@echo "Lines of code:"
	@tokei --exclude target --exclude .git

.PHONY: deps
deps:
	@echo "Dependency tree:"
	cargo tree --depth 1

.PHONY: outdated
outdated:
	@echo "Checking for outdated dependencies..."
	@command -v cargo-outdated > /dev/null || cargo install cargo-outdated
	cargo outdated

.PHONY: audit
audit:
	@echo "Security audit..."
	@command -v cargo-audit > /dev/null || cargo install cargo-audit
	cargo audit

# =============================================================================
# Release
# =============================================================================

.PHONY: version
version:
	@echo "Current version:"
	@grep '^version' Cargo.toml | head -1

.PHONY: tag
tag:
	@echo "Creating git tag..."
	@version=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'); \
	git tag -a "v$$version" -m "Release v$$version"; \
	echo "Created tag v$$version"

# =============================================================================
# Help
# =============================================================================

.PHONY: help
help:
	@echo ""
	@echo "============================================================"
	@echo "  Hydra Gateway - Development Commands"
	@echo "============================================================"
	@echo ""
	@echo "Build:"
	@echo "  make build           Build debug version"
	@echo "  make release         Build in release mode"
	@echo "  make clean           Clean build artifacts"
	@echo ""
	@echo "Test & Quality:"
	@echo "  make test            Run all tests"
	@echo "  make test-lib        Run library tests only"
	@echo "  make test-doc        Run documentation tests"
	@echo "  make fmt             Format code"
	@echo "  make fmt-check       Check formatting without applying"
	@echo "  make lint            Run clippy (strict)"
	@echo "  make lint-fix        Auto-fix lint issues"
	@echo "  make fix             Auto-fix compiler suggestions"
	@echo "  make check           Run fmt-check + lint + test"
	@echo "  make pre-push        Run all pre-push checks"
	@echo ""
	@echo "Documentation:"
	@echo "  make doc             Generate documentation"
	@echo "  make doc-open        Generate and open in browser"
	@echo "  make doc-check       Check docs build without warnings"
	@echo ""
	@echo "Docker:"
	@echo "  make docker-build    Build Docker image"
	@echo "  make docker-up       Start all services (postgres + gateway)"
	@echo "  make docker-down     Stop all services"
	@echo "  make docker-logs     Tail service logs"
	@echo "  make docker-clean    Stop services and remove volumes"
	@echo ""
	@echo "Coverage:"
	@echo "  make coverage        Generate XML coverage report"
	@echo "  make coverage-html   Generate HTML coverage report"
	@echo "  make open-coverage   Open HTML coverage report"
	@echo ""
	@echo "Git & Helpers:"
	@echo "  make git-log         Show commits on branch vs main"
	@echo "  make check-spanish   Check for Spanish in comments"
	@echo "  make tree            Show project structure"
	@echo "  make loc             Count lines of code"
	@echo "  make deps            Show dependency tree (depth 1)"
	@echo "  make outdated        Check for outdated dependencies"
	@echo "  make audit           Run security audit"
	@echo ""
	@echo "Release:"
	@echo "  make version         Show current version"
	@echo "  make tag             Create git tag from Cargo.toml version"
	@echo ""
