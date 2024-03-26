# Scuffle Contribution Guide

## Code of Conduct

Before diving in, please familiarize yourself with our [Code of Conduct](./CODE_OF_CONDUCT.md). Adherence to it ensures a harmonious community.

## Design Documents

To understand the project's structure and functionality, refer to our [design document](./design/README.md).

## CLA

Before accepting contributions, we require all contributors to sign a [Contributor License Agreement](./CLA.md). To sign the CLA, visit [cla.scuffle.tv](https://cla.scuffle.tv).

## Monorepo

This project is structured as a [monorepo](https://semaphoreci.com/blog/what-is-monorepo), meaning all code for every project resides here. We chose a monorepo for several reasons:

- Code sharing across services and products is streamlined.
- It simplifies testing and integration across the platform.
- Multiple projects can be maintained and contributed to within a single PR or ticket.

However, this approach necessitates a more intricate build system.

## Commit Messages

When committing to our `main` branch, please adhere to our conventions:

- Ensure every commit can be compiled successfully and is formatted.
- Follow the format detailed [here](https://karma-runner.github.io/6.4/dev/git-commit-msg.html).

Example:

```bash
doc(api): Added documentation for xyz
```

The general format is:

```bash
type(scope): <description>
```

In the commit message body, provide a detailed description and link to the ticket addressed by the commit:

```bash
Closes #1, #2
```

If there are breaking changes, mention them:

```bash
Breaking changes:

`abc` is no longer supported and has been replaced with `xyz`
```

For any queries about commit message formatting, feel free to ask.

## Pull Requests

Each commit in a pull request should address one or more tickets. Aim for a `many to one` relationship: multiple tickets can be addressed in a single commit, but each ticket should have only one associated commit. While developing, you can commit freely. However, before merging, you'll be asked to squash your commits and adjust their names. After ensuring CI passes, we can merge your contributions.

To squash commits:

```bash
git rebase -i HEAD~<number of commits>
```

or

```bash
git rebase -i <commit hash>
```

## Code Formatting

While formatting isn't mandatory during development, it's encouraged for easier PR reviews. Before merging, ensure your PR is formatted.

## Testing

Each subproject has specific testing requirements. Refer to the README of each subproject for details. Integration tests will be run on your PR to ensure overall system integrity.

## Documentation

Update the documentation of the subproject you're working on when making a PR. This isn't mandatory during development but is encouraged. Before merging, documentation updates are required.

## Getting Started

To begin, ensure the following are installed:

- Recommended IDE: [VSCode](https://code.visualstudio.com/)
- For Windows users, use [WSL2](https://docs.microsoft.com/en-us/windows/wsl/install-win10) for Linux commands.

### WSL2

For WSL2 users, setting up systemd is recommended to run services like Docker inside WSL2, bypassing Docker Desktop. Instructions are available [here](https://devblogs.microsoft.com/commandline/systemd-support-is-now-available-in-wsl/).

### Install Components

A guide for installing everyone on Ubuntu:

#### Rust

```bash
sudo apt-get update
sudo apt-get install -y curl gnupg ca-certificates git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Update your environment variables by adding the following to `~/.bashrc` or `~/.zshrc`:

```bash
source $HOME/.cargo/env
export PATH="$HOME/.cargo/bin:$PATH"
```

To install Mask:

```bash
cargo install mask
```

#### NodeJS

```bash
NODE_MAJOR=20

sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | sudo gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg
echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_$NODE_MAJOR.x nodistro main" | sudo tee /etc/apt/sources.list.d/nodesource.list
sudo apt-get update
sudo apt-get install -y nodejs
```

#### Pnpm

```bash
curl -fsSL https://get.pnpm.io/install.sh | bash -
```

#### Docker

```bash
sudo apt-get install -y docker.io

DOCKER_CONFIG=${DOCKER_CONFIG:-$HOME/.docker}
mkdir -p $DOCKER_CONFIG/cli-plugins
curl -SL https://github.com/docker/compose/releases/download/v2.16.0/docker-compose-linux-x86_64 -o $DOCKER_CONFIG/cli-plugins/docker-compose
chmod +x $DOCKER_CONFIG/cli-plugins/docker-compose
```

To run Docker without sudo:

```bash
sudo groupadd docker
sudo usermod -aG docker $(whoami)
```

#### C External Libraries

```bash
sudo apt-get update
sudo apt-get install pkg-config software-properties-common meson ninja-build nasm clang cmake make build-essential yasm autoconf automake libtool

git clone https://github.com/ScuffleTV/external.git --depth 1 --recurse-submodule /tmp/scuffle-external
sudo /tmp/scuffle-external/build.sh --prefix /usr/local
sudo rm -rf /tmp/scuffle-external
```

## Setting up the project

After installation, clone the project and set up dependencies:

```bash
git clone --recurse-submodules https://github.com/ScuffleTV/scuffle.git scuffle
cd scuffle
mask bootstrap
```

The bootstrap command will handle:

- Dependency installation
- Development environment setup
- .env file setup

## Development Environment

We utilize:

- [CockroachDB](https://www.cockroachlabs.com/) as our database.
- [NATs](https://nats.io/)
- [S3](https://aws.amazon.com/s3/) (or any S3-compatible service; we use [MiniIO](https://min.io/) for development)

To run local third-party services:

```bash
mask dev up
```

To shut them down:

```bash
mask dev down
```

### Database Migrations

We employ sqlx-cli for database migrations. To run migrations:

```bash
mask dev migrate
```

### Turnstile

We use [Turnstile](https://www.cloudflare.com/products/turnstile/) for captcha services. For local login request validation, set up a local Turnstile instance. Instructions are provided in the guide.

## Dev Env Setup (NOTE: this will change)

<details>

<summary>Expand</summary>

There are currently three major components to get the dev environment up and running

### Video API

Create a local/video-api-config.toml

```toml
[grpc]
bind_address = "127.0.0.1:0"

[database]
uri = "postgres://root@localhost:5432/scuffle_video"
```

```bash
cargo run --bin video-api -- --config-file local/video-api-config.toml
```

You will need to generate organization id and access token via the video-cli

Create an organization

```bash
cargo run --bin video-cli -- --config-file local/video-api-config.toml organization create --name <org_name>
```

Create an access token using the organization id just generated

```bash
cargo run --bin video-cli -- --config-file local/video-api-config.toml --organization-id XXXXXXXXXXXXXXXXXXXXXXXXXX access-token create --scopes all:admin
```

### Platform API

Create a local/platform-api-config.toml

Add the ids you generated in previous step

```toml
[grpc]
bind_address = "127.0.0.1:0"

[database]
uri = "postgres://root@localhost:5432/scuffle_platform"

[video_api]
organization_id = "XXXXXXXXXXXXXXXXXXXXXXXXXX"
access_key = "XXXXXXXXXXXXXXXXXXXXXXXXXX"
secret_key = "XXXXXXXXXXXXXXXXXXXXXXXXXX"
```

```bash
cargo run --bin platform-api -- --config-file local/platform-api-config.toml
```

### Website

The website uses vite + svelte and uses pnpm to run in dev mode

```bash
cd platform/website
pnpm run dev
```

</details>

## Questions

For any questions, join our [discord server](https://discord.gg/scuffle), create an issue on the repo, or engage in the discussion section. We're here to assist and ensure a smooth contribution process.

## Thank You

Your interest and contributions are invaluable. We're thrilled to have you on board and hope you find the experience rewarding.
