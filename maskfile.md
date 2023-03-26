# Scuffle Tasks

## build

> Build the project

<!-- Default build all  -->

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

$MASK build rust
$MASK build website
```

### rust

> Build all rust code

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

target=$(rustup show active-toolchain | cut -d '-' -f2- | cut -d ' ' -f1)

cargo build --release --target=$target
```

### website

> Build the frontend website

**OPTIONS**

- no_gql_prepare
  - flags: --no-gql-prepare
  - desc: Don't prepare the GraphQL schema

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_gql_prepare" != "true" ]; then
    $MASK gql prepare
    export SCHEMA_URL=$(realpath frontend/website/schema.graphql)
fi

yarn workspace website build
```

## clean

> Clean the project

**OPTIONS**

- all

  - flags: --all
  - desc: Removes everything that isn't tracked by git (use with caution, this is irreversible)

- node_modules

  - flags: --node-modules
  - desc: Removes node_modules

- env
  - flags: --env
  - desc: Removes .env

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [[ "$all" == "true" ]]; then
    git clean -xfd
fi

cargo clean
yarn workspace website clean

if [ "$node_modules" == "true" ]; then
    rm -rf node_modules
fi

if [ "$env" == "true" ]; then
    rm -rf .env
fi
```

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

if [ "$no_rust" != "true" ]; then
    cargo fmt --all
    cargo clippy --fix --allow-dirty --allow-staged
fi

if [ "$no_js" != "true" ]; then
    yarn format
    yarn workspace website format
fi

if [ "$no_terraform" != "true" ]; then
    terraform fmt -recursive
fi

if [ "$no_proto" != "true" ]; then
    find . -name '*.proto' -exec clang-format -i {} \;
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
    cargo sqlx prepare --check --workspace -- --all-targets --all-features
    $MASK gql check
fi

if [ "$no_js" != "true" ]; then
    yarn lint
    yarn workspace website lint
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
    yarn audit
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
- ci
  - flags: --ci
  - type: bool
  - desc: Runs tests in CI mode

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_rust" != "true" ]; then
    cargo llvm-cov clean --workspace
    if [ "$ci" == "true" ]; then
        cargo llvm-cov nextest --lcov --output-path lcov.info --ignore-filename-regex "(main\.rs|tests|.*\.nocov\.rs)" --workspace --no-fail-fast -E "not test(_v6)" --status-level all
    else
        cargo llvm-cov nextest --lcov --output-path lcov.info --ignore-filename-regex "(main\.rs|tests|.*\.nocov\.rs)" --workspace
    fi
fi

if [ "$no_js" != "true" ]; then
    yarn workspace website test
fi
```

## db

> Database tasks

### migrate

> Migrate the database

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

sqlx database create
sqlx migrate run --source ./backend/migrations
```

#### create (name)

> Create a database migration

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

sqlx migrate add "$name" --source ./backend/migrations -r
```

### rollback

> Rollback the database

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

sqlx migrate revert --source ./backend/migrations
```

### prepare

> Prepare the database

**OPTIONS**

- no_format
  - flags: --no-format
  - type: bool
  - desc: Disables formatting

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

cargo sqlx prepare --workspace -- --all-targets --all-features

if [ "$no_format" != "true" ]; then
    yarn prettier --write .sqlx
fi
```

### reset

> Reset the database

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

sqlx database reset --source ./backend/migrations
```

### up

> Starts the docker compose stack

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

docker compose --file ./dev-stack/db.docker-compose.yml up -d
```

### down

> Stops the docker compose stack

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

docker compose --file ./dev-stack/db.docker-compose.yml down
```

### status

> Gets the status of the docker compose db stack

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

docker compose --file ./dev-stack/db.docker-compose.yml ps -a
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
    echo "DATABASE_URL=postgres://postgres:postgres@localhost:5432/scuffle_dev" > .env
    echo "RMQ_URL=amqp://rabbitmq:rabbitmq@localhost:5672/%2fscuffle" >> .env
fi
```

### backup

> Backup the environment files

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ -f .env ]; then
    mv .env .env.bak
fi
```

### restore

> Restore the environment files

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ -f .env.bak ]; then
    mv .env.bak .env
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
- no_docker
  - flags: --no-docker
  - type: bool
  - desc: Disables docker bootstrapping
- no_db
  - flags: --no-db
  - type: bool
  - desc: Disables database bootstrapping

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
    cargo install sqlx-cli --features native-tls,postgres --no-default-features --git https://github.com/launchbadge/sqlx --branch main
    cargo binstall cargo-llvm-cov -y
    cargo binstall cargo-nextest -y
    cargo install cargo-audit --features vendored-openssl
fi

if [ "$no_js" != "true" ]; then
    yarn install

    if [ "$no_js_tests" != "true" ]; then
        yarn playwright install
    fi
fi

if [ "$no_env" != "true" ]; then
    $MASK env generate
fi

if [ "$no_docker" != "true" ]; then
    docker network create scuffle-dev || true

    if [ "$no_db" != "true" ]; then
        $MASK db up
        $MASK db migrate
    fi
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

cargo run --bin api-gql-generator | yarn -s prettier --stdin-filepath schema.graphql > schema.graphql
```

### check

> Check the GraphQL schema

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

cargo run --bin api-gql-generator | yarn -s prettier --stdin-filepath schema.graphql | diff - schema.graphql || (echo "GraphQL schema is out of date. Run 'mask gql prepare' to update it." && exit 1)

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
