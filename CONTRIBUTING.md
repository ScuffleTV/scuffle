# Scuffle Contribution Guide

## Code of Conduct

We have a [Code of Conduct](./CODE_OF_CONDUCT.md) that we expect all contributors to follow. Please read it before contributing.

## CLA

We require all contributors to sign a [Contributor License Agreement](./CLA.md) before we can accept any contributions.

To sign the CLA, please head over to [cla.scuffle.tv](https://cla.scuffle.tv) and sign the CLA.

## Getting Started

In order to get started, you will need to have the following installed on your machine:

For this project we recommend using [VSCode](https://code.visualstudio.com/) as your IDE.

We also advise you to use a linux based operating system, however, if you are on windows you can use [WSL2](https://docs.microsoft.com/en-us/windows/wsl/install-win10) to run Linux commands.

### WSL2

If you are using WSL2, we recommend you also setup systemd so you can run services like docker inside of WSL2 rather than using Docker Desktop.

You can find instructions on how to do that [here](https://devblogs.microsoft.com/commandline/systemd-support-is-now-available-in-wsl/).

- [Git](https://git-scm.com/)
- [NodeJS](https://nodejs.org/en/)
- [PNPM](https://pnpm.io/)
- [Docker](https://www.docker.com/)
- [Docker Compose V2](https://docs.docker.com/compose/install)
- [Rust](https://www.rust-lang.org/tools/install)
- [Mask](https://github.com/jacobdeichert/mask)
- [Terraform](https://developer.hashicorp.com/terraform)

### For Ubuntu

If you are using Ubuntu you can install everything with the following commands:

```bash
# Installing Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Configuring apt to find nodejs
curl -sL https://deb.nodesource.com/setup_18.x | sudo -E bash -

# install pnpm
curl -fsSL https://get.pnpm.io/install.sh | sh -

# Running the install for nodejs, make, docker and git
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev nodejs docker.io git gnupg software-properties-common

# Add Hashicorp's GPG key
wget -O- https://apt.releases.hashicorp.com/gpg | gpg --dearmor | sudo tee /usr/share/keyrings/hashicorp-archive-keyring.gpg > /dev/null

# Add Hashicorp's repository
echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] \
https://apt.releases.hashicorp.com $(lsb_release -cs) main" | \
sudo tee /etc/apt/sources.list.d/hashicorp.list > /dev/null

# Install Terraform
sudo apt-get update && sudo apt-get install terraform

# Installing docker compose v2
DOCKER_CONFIG=${DOCKER_CONFIG:-$HOME/.docker}
mkdir -p $DOCKER_CONFIG/cli-plugins
curl -SL https://github.com/docker/compose/releases/download/v2.16.0/docker-compose-linux-x86_64 -o $DOCKER_CONFIG/cli-plugins/docker-compose
chmod +x $DOCKER_CONFIG/cli-plugins/docker-compose
```

You should also make it so you can run docker without sudo.

```bash
sudo groupadd docker
sudo usermod -aG docker $USER
```

Now you need to setup your environment variables.

You should add the following to your `~/.bashrc` or `~/.zshrc` file.

```bash
source $HOME/.cargo/env
export PATH="$HOME/.cargo/bin:$PATH"
```

Installing Mask

```
cargo install mask
```

## Setting up the project

Once you have everything installed, you can clone the project and install the dependencies.

```bash
git clone --recurse-submodules https://github.com/ScuffleTV/scuffle.git scuffle
cd scuffle
mask bootstrap
```

The boostrap command will setup the project for you.

This includes:

- Installing all the dependencies
- Setting up the database
- Setting up .env files

## Development Database

We use Postgres for our database.

You can run a local instance of Postgres with the following command:

```bash
mask db up
```

To shut down the local instance of Postgres you can run the following command:

```bash
mask db down
```

### Database Migrations

We use sqlx-cli to manage our database migrations.

You can run the migrations with the following command:

```bash
mask db migrate
```

You can create a new migration with the following command:

```bash
mask db migrate add "migration name"
```

Then you can find the SQL for the migration in the [migrations](./backend/migrations) folder.
You can then edit the up migration file to add your SQL.
You must also provide a down migration file so we can rollback the migration.

You will then be prompted to rerun the prepare command

```bash
mask db prepare
```

This will run the migrations and generate the SQLx code for the database. So that compile time querying can be used.

### Turnstile

We use [Turnstile](https://www.cloudflare.com/products/turnstile/) as our captcha service.

In order to validate local login requests, you will need to setup a local instance of Turnstile.

You can go to cloudflare's website and register for a free account and then create a new application.

Set the domain to `localhost` and the widget type to managed.

Then create a `.env.local` file in the `frontend/website` folder and add the following:

```
VITE_CF_TURNSTILE_KEY=<site-key>
VITE_GQL_ENDPOINT=http://localhost:8080/v1/gql
VITE_GQL_WS_ENDPOINT=ws://localhost:8080/v1/gql
VITE_GQL_VERSION=1.0
```

Then you can export the following environment variables:

```bash
export SCUF_TURNSTILE_SECRET_KEY=<secret-key>
```

Then when you start the API server it will use the local instance of Turnstile.

## Monorepo

For starters, you will notice that this project is a [monorepo](https://semaphoreci.com/blog/what-is-monorepo).

This means that all our code for every project is stored here in this repo.

We opted to use a monorepo for a few reasons:

- We can minimize code duplication since we can share everything between services and products.
- We can easily test and integrate the entire platform.
- We can easily maintain and contribute to multiple projects within a single PR or ticket.

Monorepos come at a cost of a more complex build system.

## Commit Messages

When we make commits to our `main` branch we have a few conventions we would like you to follow.

Every commit you commit must be able to be compiled successfully, and also must be formatted.

Commits must be in the format specified here https://karma-runner.github.io/6.4/dev/git-commit-msg.html

`doc(api): Added documentation for xyz`

where the format is basically

`type(scope): <description>`

Then in the commit message body, you can give a more detailed description and link to the ticket that this commit aims to resolve.

`Closes #1, #2`

We then would like you to mention breaking changes if any.

```
Breaking changes:

`abc` is no longer supported and has been replaced with `xyz`
```

If you have any questions regarding how commit messages should be formatted please ask.

## Pull Requests

Each commit in a pull request should resolve one or more tickets.

There should be one commit per ticket. If we need more tickets then we can create sub-issues and tasks around those.

So the relationship between tickets to commits is `many to one` where we can have many tickets in a single commit but only one commit per ticket.

You should try and break up the commits as one ticket per commit but sometimes the trivial tickets might be small enough that we can just combine them into a single commit.

A maintainer and or reviewer will advise you on what you should do to make your PR mergeable.

However, you do not need to do this for the development stage of your PR. While developing you can commit as many times as you want with any names you like, however, once it is ready for merge someone will ask you to squash your commits into tickets and fix up the naming on them. Once that is done and CI passes we can then merge your contributions!

Squashing commits can be done with the following command:

```
git rebase -i HEAD~<number of commits>
```

or

```
git rebase -i <commit hash>
```

Then you can change the `pick` to `squash` for all the commits you want to squash into the first commit. Then you can change the commit message to the ticket number and description.

## Code Formatting

Formatting is not required during the development stage but it is encouraged since it makes it easier to review your PR. Once we get to the merging phase we would require you to format the PR so it is ready to be merged.

## Testing

Each subproject will have its testing requirements. Please read the README for each subproject to see what is required.

Integration tests will run on your PR to ensure nothing else breaks during your implementation.

## Documentation

When you make a PR, you should also update the documentation for the subproject you are working on. This is not required during the development stage, but it is encouraged since it makes it easier to review your PR. Once we get to the merging phase, we will require you to update the documentation, so it is ready to be merged.

## Questions

If you have any questions, please ask in the [discord server](https://scuffle.tv/discord) or create an issue on the repo or in the discussion section

Please do not hesitate to ask questions; we are here to help you and make sure you are comfortable contributing to this project. If you need help following the design documents or need clarification about the codebase, please ask us, and we will help you.

## Thank you

Thank you for taking the time to read this document and for contributing to this project. We are very excited to have you on board, and we hope you enjoy your time here.
