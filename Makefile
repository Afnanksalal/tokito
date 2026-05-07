.PHONY: dev test lint fmt check docker-up docker-down

# Run API locally (Postgres via docker-compose assumed on port 5433)
dev:
	docker compose up -d postgres
	set TOKITO_DATABASE_URL=postgres://tokito:tokito@localhost:5433/tokito?sslmode=disable && cargo run -p tokito

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --check

check: fmt lint test

docker-up:
	docker compose up -d postgres

docker-down:
	docker compose down
