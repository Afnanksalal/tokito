.PHONY: dev test test-db lint fmt check deny

dev:
	cargo run -p tokito-native

test:
	cargo nextest run --workspace || cargo test --workspace

# Requires pg-embed binary download/extract (network)
test-db:
	TOKITO_RUN_DB_INTEGRATION=1 cargo nextest run -p tokito --test integration \
		|| TOKITO_RUN_DB_INTEGRATION=1 cargo test -p tokito --test integration -- --nocapture

lint:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all -- --check

deny:
	cargo deny check

check: fmt lint test
