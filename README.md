[![ci](https://github.com/radhermit/bugbite/workflows/ci/badge.svg)](https://github.com/radhermit/bugbite/actions/workflows/ci.yml)
[![coverage](https://codecov.io/gh/radhermit/bugbite/branch/main/graph/badge.svg)](https://codecov.io/gh/radhermit/bugbite)

# Bugbite

Library and tools for bug, issue, and ticket mangling.

- bugbite: core library
- [bite]: CLI client
- chew: TUI client

## Test

Testing is supported via [nextest]. To run all bugbite workspace unit tests:

```bash
cargo nextest run --all-features --workspace --tests
```

## Containers

Some services have containers in `docker/*` that provide the following user for
testing purposes:

    username: bugbite
    password: bugbite
    email: bugbite@bugbite.test

Start a Bugzilla instance:

```bash
docker compose -f docker/bugzilla.yml up --wait
```

In addition, some services have integration tests that can be run against the
related container. For example, to run the bugzilla integration tests:

```bash
cargo nextest run --all-features --profile bugzilla --workspace --tests
```

[bite]: <https://github.com/radhermit/bugbite/tree/main/crates/cli>
[nextest]: <https://nexte.st/>
