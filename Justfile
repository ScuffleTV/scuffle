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

test:
	cargo test
	yarn --cwd ./frontend/website test

setup:
	yarn
	yarn --cwd ./frontend/website
	cargo install cargo-watch
	cargo install wasm-pack
	rustup target add wasm32-unknown-unknown

clean:
	cargo clean
	yarn --cwd ./frontend/website clean
