# Contributing

Thank you for your interest in contributing to `senc`!

This document lists some information that contributors should be aware of when contributing.


## Development

`senc` is a multi-language project:
- The core CLI runtime is implemented in [rust](https://www.rust-lang.org/).
- Builtin functions are written in JavaScript.
- Builtin types are written in TypeScript, specifically [as
  DTS](https://www.typescriptlang.org/docs/handbook/declaration-files/templates/module-d-ts.html).

For `rust` components, we rely on `cargo` for the build chain and dependency management.

For the JavaScript and TypeScript components, we rely on [pnpm](https://pnpm.io/) for package management.


### Running tests

To run the tests, you need to first pull down the JavaScript dependencies used in the fixtures with `pnpm`:

```
cd ./tests/fixtures
pnpm install
```

Afterwards, you can run the `rust` tests with `cargo`:

```
cargo test
```


## CI

We primarily use [CircleCI](https://circleci.com/) for our build process. This is to leverage the wide support of OS and
architectures available to us (we ship binaries for `linux_amd64`, `linux_arm64`, `darwin_arm64`, and `windows_amd64`).

We also use GitHub Actions for very limited workflows that require tighter integration with GitHub, such as for PR title
validation.

Note that the CircleCI config files are [maintained using `senc`](https://github.com/fensak-io/senc/tree/main/_ci).


## CD

We use [semantic-release](https://github.com/semantic-release/semantic-release) to manage our release process.

All tags are cut from the [release branch](https://github.com/fensak-io/senc/tree/release). The consequence of this is
that the source level versioning (e.g., in `Cargo.toml`) is managed directly on the `release` branch instead of `main`.
