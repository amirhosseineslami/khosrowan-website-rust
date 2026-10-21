# personal-site — common tasks

.PHONY: help dev build release test fmt fmt-check lint run seed clean docker smoke

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

dev: ## Run the development server
	cargo run

build: ## Build a debug binary
	cargo build

release: ## Build an optimised binary
	cargo build --release

test: ## Run every test
	cargo test

fmt: ## Format the source
	cargo fmt --all

fmt-check: ## Check formatting (used by CI)
	cargo fmt --all -- --check

lint: ## Run clippy
	cargo clippy --all-targets --all-features -- -D warnings

run: release ## Build and run the optimised binary
	./target/release/personal-site

seed: ## Recreate the database with demo content
	rm -rf data && mkdir -p data
	./target/release/personal-site

smoke: ## Run the end-to-end smoke test against a running instance
	./scripts/smoke.sh

docker: ## Build the Docker image
	docker build -t personal-site .

clean: ## Remove build artefacts and local data
	cargo clean
	rm -rf data media
