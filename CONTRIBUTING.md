# Scuffle Contribution Guide

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
