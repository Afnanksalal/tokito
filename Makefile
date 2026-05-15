.PHONY: dev test lint fmt check

dev:
	cargo run -p tokito-native

test:
	cargo test --workspace

# Requires pg-embed binary download/extract (network)
test-db:
	TOKITO_RUN_DB_INTEGRATION=1 cargo test -p tokito --test api_designs --test api_parts --test api_schematic -- --nocapture

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --check

check: fmt lint test
