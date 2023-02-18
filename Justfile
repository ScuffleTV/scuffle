set dotenv-load

arch := `uname -m | sed 's/amd64/x86_64/' | sed 's/arm64/aarch64/'`

build:
	yarn --cwd ./frontend/website build
	cargo build --release

build-container: env-backup
	docker run --rm -v $(pwd):/pwd -w /pwd ghcr.io/scuffletv/base-build:1.66.1 just build

env-backup:
	test -f .env && (\
		mv .env .env.bak \
	) || true

format:
	yarn --cwd ./frontend/website format
	yarn format
	cargo fmt --all
	cargo clippy --fix --allow-dirty --allow-staged
	cargo clippy --fix --allow-dirty --allow-staged --package player --target wasm32-unknown-unknown

lint:
	yarn lint
	yarn --cwd ./frontend/website lint
	cargo clippy
	cargo clippy --package player --target wasm32-unknown-unknown
	cargo fmt --all --check
	cargo sqlx prepare --check --merged -- --all-targets --all-features

test:
	cargo test
	yarn --cwd ./frontend/website test

setup: setup-deps env
	yarn global add wasm-pack
	cargo install cargo-watch
	cargo install sqlx-cli
	rustup target add wasm32-unknown-unknown
	rustup target add {{arch}}-unknown-linux-musl

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
