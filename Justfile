set dotenv-load

arch := `uname -m | sed 's/amd64/x86_64/' | sed 's/arm64/aarch64/'`

build:
	yarn workspace website build
	cargo build --release

build-container: env-backup
	docker run --rm -v $(pwd):/pwd -w /pwd ghcr.io/scuffletv/base-build:1.66.1 just build

env-backup:
	test -f .env && (\
		mv .env .env.bak \
	) || true

format:
	yarn format
	yarn workspace website format
	cargo fmt --all
	cargo clippy --fix --allow-dirty --allow-staged
	cargo clippy --fix --allow-dirty --allow-staged --package player --target wasm32-unknown-unknown

lint:
	yarn lint
	yarn workspace website lint
	cargo clippy
	cargo clippy --package player --target wasm32-unknown-unknown
	cargo fmt --all --check
	cargo sqlx prepare --check --merged -- --all-targets --all-features

test: test-rust test-js

test-rust:
	cargo test

test-js:
	yarn workspace website test

audit:
	cargo audit
	yarn audit

setup: setup-deps env
	cargo install cargo-watch
	cargo install sqlx-cli
	cargo install cargo-audit --features=fix,vendored-openssl
	rustup target add wasm32-unknown-unknown
	rustup target add {{arch}}-unknown-linux-musl

setup-deps:
	yarn

setup-tests:
	yarn playwright install

clean:
	cargo clean
	yarn workspace website clean

db-migrate:
	sqlx database create
	sqlx migrate run --source ./backend/migrations

db-prepare:
	cargo sqlx prepare --merged -- --all-targets --all-features
	yarn prettier --write sqlx-data.json

db-migrate-create *ARGS:
	sqlx migrate add "{{ ARGS }}" --source ./backend/migrations -r

db-rollback:
	sqlx migrate revert --source ./backend/migrations

db-reset:
	sqlx database reset --source ./backend/migrations
	just db-migrate

db-up:
	docker network create --driver bridge scuffle-dev || true
	docker compose --file ./dev-stack/db.docker-compose.yaml up -d
	just db-migrate

env:
	test -f .env || (\
		test -f .env.bak && (\
			mv .env.bak .env \
		) || (\
			echo "DATABASE_URL=postgres://postgres:postgres@localhost:5432/scuffle-dev" > .env \
		) \
	)

db-down:
	docker compose --file ./dev-stack/db.docker-compose.yaml down

stack-init:
	cp ./dev-stack/stack-example.docker-compose.yaml ./dev-stack/stack.docker-compose.yaml

stack-up:
	docker network create --driver bridge scuffle-dev || true
	docker compose --file ./dev-stack/stack.docker-compose.yaml up -d --build

stack-down:
	docker compose --file ./dev-stack/stack.docker-compose.yaml down

stack-logs *ARGS:
	docker compose --file ./dev-stack/stack.docker-compose.yaml logs {{ ARGS }}
