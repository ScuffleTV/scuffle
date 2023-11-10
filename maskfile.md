# Scuffle Tasks

## format

> Format the project

**OPTIONS**

- no_rust
  - flags: --no-rust
  - type: bool
  - desc: Disables Rust formatting
- no_js
  - flags: --no-js
  - type: bool
  - desc: Disables JS formatting
- no_terraform
  - flags: --no-terraform
  - type: bool
  - desc: Disables Terraform formatting
- no_proto
  - flags: --no-proto
  - type: bool
  - desc: Disables Protobuf formatting

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_js" != "true" ]; then
    pnpm --recursive --parallel --stream run format
fi

if [ "$no_terraform" != "true" ]; then
    terraform fmt -recursive
fi

if [ "$no_proto" != "true" ]; then
    find . -name '*.proto' -exec clang-format -i {} \;
fi

if [ "$no_rust" != "true" ]; then
    cargo fmt --all
    cargo clippy --fix --allow-dirty --allow-staged
fi
```

## lint

> Lint the project

**OPTIONS**

- no_rust
  - flags: --no-rust
  - type: bool
  - desc: Disables Rust linting
- no_js
  - flags: --no-js
  - type: bool
  - desc: Disables JS linting
- no_terraform
  - flags: --no-terraform
  - type: bool
  - desc: Disables Terraform linting
- no_proto
  - flags: --no-proto
  - type: bool
  - desc: Disables Protobuf linting

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_rust" != "true" ]; then
    cargo clippy -- -D warnings
    cargo fmt --all --check
    $MASK gql check
fi

if [ "$no_js" != "true" ]; then
    pnpm --recursive --parallel --stream run lint
fi

if [ "$no_terraform" != "true" ]; then
    terraform fmt -check -recursive
fi

if [ "$no_proto" != "true" ]; then
    find . -name '*.proto' -exec clang-format --dry-run --Werror {} \;
fi
```

## audit

> Audit the project

**OPTIONS**

- no_rust
  - flags: --no-rust
  - type: bool
  - desc: Disables Rust linting
- no_js
  - flags: --no-js
  - type: bool
  - desc: Disables JS linting

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_rust" != "true" ]; then
    cargo audit
fi

if [ "$no_js" != "true" ]; then
    pnpm audit
fi
```

## test

> Test the project

**OPTIONS**

- no_rust
  - flags: --no-rust
  - type: bool
  - desc: Disables Rust testing
- no_js
  - flags: --no-js
  - type: bool
  - desc: Disables JS testing
- no_player_build
  - flags: --no-player-build
  - type: bool
  - desc: Disables Player Building

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_rust" != "true" ]; then
  cargo llvm-cov nextest --lcov --output-path lcov.info --ignore-filename-regex "(main\.rs|tests|.*\.nocov\.rs)" --workspace --fail-fast -r --exclude video-player
fi

if [ "$no_js" != "true" ]; then
    if [ "$no_player_build" != "true" ]; then
        $MASK build player
    fi

    pnpm --recursive --parallel --stream run test
fi
```

## dev

> Database tasks

### migrate

> Migrate the database

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

# We load the .env file
export $(cat .env | xargs)

echo "Migrating platform database"
DATABASE_URL=$PLATFORM_DATABASE_URL sqlx database create
DATABASE_URL=$PLATFORM_DATABASE_URL sqlx migrate run --source ./platform/migrations

echo "Migrating video database"
DATABASE_URL=$VIDEO_DATABASE_URL sqlx database create
DATABASE_URL=$VIDEO_DATABASE_URL sqlx migrate run --source ./video/migrations
```

### up

> Starts the docker compose stack

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

docker compose --file ./dev/docker-compose.yml up -d
```

### down

> Stops the docker compose stack

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

docker compose --file ./dev/docker-compose.yml down
```

### status

> Gets the status of the docker compose db stack

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

docker compose --file ./dev/docker-compose.yml ps -a
```

## env

> Environment tasks

### generate

> Generate the environment files

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ ! -f .env ]; then
    echo "PLATFORM_DATABASE_URL=postgres://root@localhost:5432/scuffle_platform" >> .env
    echo "VIDEO_DATABASE_URL=postgres://root@localhost:5432/scuffle_video" >> .env
    echo "NATS_ADDR=localhost:4222" >> .env
    echo "REDIS_ADDR=localhost:6379" >> .env
fi
```

## bootstrap

> Bootstrap the project

**OPTIONS**

- no_rust
  - flags: --no-rust
  - type: bool
  - desc: Disables Rust bootstrapping
- no_js
  - flags: --no-js
  - type: bool
  - desc: Disables JS bootstrapping
- no_js_tests
  - flags: --no-js-tests
  - type: bool
  - desc: Disables JS tests bootstrapping
- no_env
  - flags: --no-env
  - type: bool
  - desc: Disables environment bootstrapping
- no_dev
  - flags: --no-dev
  - type: bool
  - desc: Disables dev docker bootstrapping

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_rust" != "true" ]; then
    rustup update

    rustup component add rustfmt clippy llvm-tools-preview

    cargo install cargo-binstall
    cargo binstall cargo-watch -y
    cargo binstall sqlx-cli -y
    cargo binstall cargo-llvm-cov -y
    cargo binstall cargo-nextest -y
    cargo binstall cargo-audit -y
    cargo binstall wasm-bindgen-cli -y
fi

if [ "$no_js" != "true" ]; then
    pnpm --recursive --stream install --frozen-lockfile

    if [ "$no_js_tests" != "true" ]; then
        pnpm --filter website exec playwright install
    fi
fi

if [ "$no_env" != "true" ]; then
    $MASK env generate
fi

if [ "$no_dev" != "true" ]; then
    $MASK dev up
    $MASK dev migrate
fi
```

## update

> Update the project

**OPTIONS**

- no_rust
  - flags: --no-rust
  - type: bool
  - desc: Disables Rust updating
- rust_up
  - flags: --rust-up
  - type: bool
  - desc: Updates Rust toolchain
- no_js
  - flags: --no-js
  - type: bool
  - desc: Disables JS updating

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$rust_up" == "true" ]; then
    rustup update
fi

if [ "$no_rust" != "true" ]; then

    cargo update
fi

if [ "$no_js" != "true" ]; then
    pnpm --recursive --stream run update
fi
```

## gql

> GraphQL tasks

### prepare

> Generate the GraphQL schema

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

cargo run --bin platform-api -- --export-gql | pnpm exec prettier --stdin-filepath schema.graphql > schema.graphql
```

### check

> Check the GraphQL schema

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

cargo run --bin platform-api -- --export-gql | pnpm exec prettier --stdin-filepath schema.graphql | diff - schema.graphql || (echo "GraphQL schema is out of date. Run 'mask gql prepare' to update it." && exit 1)

echo "GraphQL schema is up to date."
```

## cloc

> Count lines of code

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

cloc $(git ls-files)
```
