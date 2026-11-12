# Contributing to cascade-bridge-rs

Thank you for helping. This is the Cascade Bridge for Rust: it runs Cascade Bridge Adapters. The contract is the
[Cascade Bridge Specification](https://github.com/jayostis/cascade-bridge-spec), and this repository is one
implementation of it, never a second statement of it. Where the specification is silent or wrong, the fix is a pull
request there. [`CLAUDE.md`](CLAUDE.md) holds the rules a change here follows.

Please follow the organisation's code of conduct and security policy. Report a security problem privately, as that
policy says, never in a public issue.

## Set up

You need git, Node.js 22 or newer, and Rust through `rustup`. [`rust-toolchain.toml`](rust-toolchain.toml) pins the Rust
version with its `rustfmt` and `clippy` components, and `rustup` installs them the first time you run `cargo`. Add the
WebAssembly target for the package build and the wasm lint:

```sh
git clone git@github.com:<you>/cascade-bridge-rs.git
cd cascade-bridge-rs
rustup target add wasm32-unknown-unknown
```

Some checks read the real adapters from sibling folders:
[cascade-bridge-adapter-ccda](https://github.com/jayostis/cascade-bridge-adapter-ccda) and
[cascade-bridge-adapter-fhir-r4](https://github.com/jayostis/cascade-bridge-adapter-fhir-r4), cloned beside this
repository. The unit tests do not need them; one step of the `tests, format and lints` check does, and CI checks them out
for it.

`package/build.sh` is a POSIX shell script, so on Windows run it from Git Bash or WSL. Its first run installs
`wasm-bindgen-cli` with `cargo install`, which needs the network and is slow.

## Run the checks

These are the checks CI runs, written as plain `cargo` and `node` so you can run them without CI's extra tooling.

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked -p cascade-bridge-wasm --target wasm32-unknown-unknown -- -D warnings
sh package/build.sh
cargo test --locked
cargo test --locked -p cascade-bridge-cli --test node_host -- --include-ignored
node --test package/tests/
```

`cargo fmt --all` fixes formatting. `sh package/build.sh` builds the WebAssembly package into `package/dist`, which the
`node_host` test and the Node tests load; `cargo test` never builds or installs anything, so build first.
`sh package/build.sh --fast` skips the slow optimisation, and is what the `compatibility` check uses. The
specification's vector, `specification_vector.rs`, also runs with `--include-ignored` where the specification is
checked out beside this repository.

## What a pull request must pass

All of these must be green before a merge. Their names are required status checks, so do not rename a job in a pull
request.

- **`tests, format and lints`**: `cargo fmt --check`, clippy for the host and for WebAssembly with warnings as errors,
  `cargo test`, and a check that the conformance this repository holds matches the ccda adapter's.
- **`compatibility`**: builds the package, runs the specification's compatibility tool with your pull request and the
  counterparts it names, runs the test suites and the Node tests, type-checks the package's TypeScript consumer, and
  fails when a shipped adapter query falls back to stock Oxigraph.
- **`ready-to-merge`**: holds a pull request until every pull request it depends on can merge (see "A change that spans two repositories").

A pull request from this repository also gets a `build-<commit>` pre-release from the `publish` workflow, which is what
other repositories read when they depend on your change. A pull request from a fork gets none.

Dependencies are pinned exactly and `Cargo.lock` is committed; [`CLAUDE.md`](CLAUDE.md) has the rule.

## Commits and pull requests

Use conventional commits, in the impersonal voice: `feat(bridge): ...`, `fix: ...`. The commit message says why a change
was made and what alternative it did not take; no file does. Do not put a closing keyword (`closes`, `fixes`,
`resolves`) before an issue the pull request does not finish.

[`CLAUDE.md`](CLAUDE.md) says what a comment may not do, and how to name a test.

## A change that spans two repositories

Nothing here pins another repository. A change that must land here and in another, the specification or an adapter, is
a pair of pull requests, each naming the other in its description on a line of its own, with the full URL:

```
Depends-On: https://github.com/jayostis/cascade-bridge-spec/pull/123
```

The `compatibility` check then runs both together, and `ready-to-merge` holds each until the other can merge. A person
merges them one after another, and a default branch is red between the first merge and the last. A change to a named
pull request retests nothing: re-run the dependent pull request's workflow with `gh run rerun`, and re-run
`compatibility` and `ready-to-merge` just before you merge.

A named pull request must be from a branch of its repository itself: one from a fork fails the check. A contributor
without write access to the other repository asks a maintainer to open the counterpart.

The full rules are in the specification's
[`compatibility.md`](https://github.com/jayostis/cascade-bridge-spec/blob/main/compatibility.md).

## Licence

Apache-2.0.
