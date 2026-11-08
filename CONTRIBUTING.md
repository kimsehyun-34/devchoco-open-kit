# Contributing

Thanks for helping out. This repository holds two independent parts, a
TypeScript package (`packages/channel-icons`) and a Rust crate
(`crates/safe-download`). Small, focused changes are easiest to review.

## Before you start

- Search existing issues and pull requests first.
- For anything larger than a small fix, open an issue to agree on the approach
  before writing code.
- Read the [Code of Conduct](CODE_OF_CONDUCT.md). Report vulnerabilities
  privately as described in [SECURITY.md](SECURITY.md), not in public issues.

## Branch and pull request flow

1. Fork the repository, or create a branch if you have write access.
2. Branch from `main` with a short, prefixed name:
   `feat/`, `fix/`, `docs/`, `test/`, `ci/`, `refactor/` or `chore/`, for
   example `fix/safe-download-trailing-dots`.
3. Keep each pull request to one logical change, with tests and docs in the
   same pull request.
4. Open the pull request against `main` and fill in the template. Mark it as a
   draft while it is still in progress.
5. Wait for CI to pass on Ubuntu and Windows. Maintainers review and merge;
   merges are normally squashed, so the **pull request title becomes the commit
   message** and must follow the style below.

## Commit style

We use [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<scope>): <summary in the imperative, lower case, no trailing period>
```

- Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`,
  `chore`.
- Scopes (optional): `channel-icons`, `safe-download`, `ci`, `deps`, `repo`.
- Mark breaking changes with `!` after the scope (`feat(safe-download)!: ...`)
  and explain them in the body.

Examples:

```text
feat(channel-icons): add a glyph for announcement channels
fix(safe-download): reject names that end in a dot on Windows
docs: describe the progress callback contract
```

## Running the test suites

### TypeScript packages

Requires Node.js 20.19 or newer and pnpm 10.

```sh
pnpm install
pnpm -r build        # compile every package
pnpm -r test         # run every package's tests
pnpm --filter channel-icons test   # one package only
```

### Rust crate

Requires a stable Rust toolchain with `rustfmt` and `clippy`.

```sh
# from the repository root
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p safe-download   # one crate only
```

Run `cargo fmt --all` to fix formatting instead of only checking it. The style is set by
`rustfmt.toml` at the repository root.

## Pull request checklist

- Tests cover the new or changed behaviour, and both suites above pass locally.
- Public APIs have short doc comments, and READMEs are updated when behaviour
  changes.
- No secrets, credentials, personal data or references to private systems are
  included in code, tests, fixtures or commit history.
- Changes are written for Windows and Linux; do not rely on path separators or
  case-sensitive file systems.

## Licensing

By contributing you agree that your contribution is licensed under the
[MIT License](LICENSE) that covers this repository.
