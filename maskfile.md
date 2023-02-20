# Scuffle Tasks

## build

> Build the project

<!-- Default build all  -->

**OPTIONS**

- container
  - flags: --container
  - desc: Build the project in a container

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$container" == "true" ]; then
    $MASK env backup

    function cleanup {
        $MASK env restore
        docker stop $PID >> /dev/null
    }
    trap cleanup EXIT

    PID=$(docker run -d --stop-signal SIGKILL --rm -v "$(pwd)":/pwd -w /pwd ghcr.io/scuffletv/build:1.67.1 mask build)
    docker logs -f $PID
else
    $MASK build rust
    $MASK build website
fi
```

### rust

> Build all rust code

**OPTIONS**

- container
  - flags: --container
  - desc: Build the project in a container

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$container" == "true" ]; then
    $MASK env backup

    function cleanup {
        $MASK env restore
        docker stop $PID >> /dev/null
    }
    trap cleanup EXIT

    PID=$(docker run -d --stop-signal SIGKILL --rm -v "$(pwd)":/pwd -w /pwd ghcr.io/scuffletv/build:1.67.1 cargo build --release)
    docker logs -f $PID
else
    cargo build --release
fi
```

### website

> Build the frontend website

**OPTIONS**

- container
  - flags: --container
  - desc: Build the project in a container

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$container" == "true" ]; then
    $MASK env backup

    function cleanup {
        $MASK env restore
        docker stop $PID >> /dev/null
    }
    trap cleanup EXIT

    PID=$(docker run -d --stop-signal SIGKILL --rm -v "$(pwd)":/pwd -w /pwd ghcr.io/scuffletv/build:1.67.1 yarn workspace website build)
    docker logs -f $PID
else
    yarn workspace website build
fi
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

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_rust" != "true" ]; then
    cargo fmt --all
    cargo clippy --fix --allow-dirty --allow-staged
    cargo clippy --fix --allow-dirty --allow-staged --package player --target wasm32-unknown-unknown
fi

if [ "$no_js" != "true" ]; then
    yarn format
    yarn workspace website format
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

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_rust" != "true" ]; then
    cargo clippy
    cargo clippy --package player --target wasm32-unknown-unknown
    cargo fmt --all --check
    cargo sqlx prepare --check --merged -- --all-targets --all-features
fi

if [ "$no_js" != "true" ]; then
    yarn lint
    yarn workspace website lint
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

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

if [ "$no_rust" != "true" ]; then
    cargo llvm-cov nextest --all-features --workspace --lcov --output-path lcov.info
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

cargo sqlx prepare --merged -- --all-targets --all-features

if [ "$no_format" != "true" ]; then
    yarn prettier --write sqlx-data.json
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
    echo "DATABASE_URL=postgres://postgres:postgres@localhost:5432/scuffle-dev" > .env
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

## stack

> Development stack tasks

### up

> Starts the docker compose stack

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

docker compose --file ./dev-stack/docker-compose.yml up -d --build
```

### down

> Stops the docker compose stack

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

docker compose --file ./dev-stack/docker-compose.yml down
```

### init

> Initializes the development stack

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

cp ./dev-stack/example.docker-compose.yml ./dev-stack/docker-compose.yml
```

### logs (service)

> Prints the logs of the given service
> You can show logs of multiple services by passing a single string with space separated service names

**OPTIONS**

- follow
  - flags: -f, --follow
  - type: bool
  - desc: Follow log output

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

follow=${follow:-false}

docker compose --file ./dev-stack/docker-compose.yml logs --follow=$follow $service
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
- no_stack
  - flags: --no-stack
  - type: bool
  - desc: Disables stack bootstrapping
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
    rustup target add wasm32-unknown-unknown
    rustup target add x86_64-unknown-linux-musl

    rustup component add rustfmt
    rustup component add clippy
    rustup component add llvm-tools-preview

    cargo install cargo-watch
    cargo install sqlx-cli
    cargo install wasm-pack
    cargo install cargo-llvm-cov
    cargo install cargo-nextest
    cargo install cargo-audit --features=fix,vendored-openssl
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

    if [ "$no_stack" != "true" ]; then
        $MASK stack init
    fi

    if [ "$no_db" != "true" ]; then
        $MASK db up
        $MASK db migrate
    fi
fi
```
