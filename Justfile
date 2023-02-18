build:
	yarn --cwd ./frontend/website build
	cargo build --release

format:
	yarn --cwd ./frontend/website format
	yarn format
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged

lint:
	yarn lint
	yarn --cwd ./frontend/website lint
	cargo clippy
	cargo fmt -- --check
	cargo sqlx prepare --check --merged -- --all-targets --all-features

test:
	cargo test
	yarn --cwd ./frontend/website test

setup: setup-deps
	yarn global add wasm-pack
	cargo install cargo-watch
	cargo install sqlx-cli
	rustup target add wasm32-unknown-unknown

setup-deps:
	yarn
	yarn --cwd ./frontend/website

clean:
	cargo clean
	yarn --cwd ./frontend/website clean

db-migrate:
	sqlx database create
	sqlx migrate run --source ./backend/migrations

db-prepare:
	cargo sqlx prepare --merged -- --all-targets --all-features
	prettier --write sqlx-data.json

db-migrate-create *ARGS:
	sqlx migrate add "{{ ARGS }}" --source ./backend/migrations -r

db-rollback:
	sqlx migrate revert --source ./backend/migrations

db-reset:
	sqlx database reset --source ./backend/migrations
	just db-migrate

db-up:
	docker network create --driver bridge scuffle-dev || true
	docker compose --file ./development-docker/db.docker-compose.yaml up -d
	echo "DATABASE_URL=postgres://postgres:postgres@localhost:5432/scuffle-dev" > .env
	just db-migrate

db-down:
	docker compose --file ./development-docker/db.docker-compose.yaml down
