# cascade-bridge-rs — Agent Context

The Cascade Bridge for Rust: it runs Cascade Bridge Adapters. The contract is
the Cascade Bridge Specification, `jayostis/cascade-bridge-spec`, and this
repository is one implementation of it, never a second statement of it.

## The rules

- **The specification is the authority.** A test type's rule is the
  `rdfs:comment` on that type in the specification's `vocab/bridge.ttl`.
  Implement that, not a paraphrase. Where the specification is silent or
  wrong, the fix is a pull request there, not a behaviour invented here.
- **Nothing here pins another repository.** Which version of each
  one a run uses is `jayostis/cascade-bridge-spec`'s
  [`compatibility.md`](https://github.com/jayostis/cascade-bridge-spec/blob/main/compatibility.md).
- **Its own tests use only the synthetic adapter; real adapters are checked
  only through `compatibility.json`.** The test subject is
  `crates/bridge/tests/tiny-adapter`, built so each outcome is reached by the
  smallest input that can reach it. An engine tested against the adapters it
  has met passes those adapters, not the contract.
- **The library never touches a filesystem.** Only `crates/bridge/src/resolver.rs`
  names `std::fs` or `std::path`, and `crates/bridge/tests/boundary.rs` holds it
  to that. A host that is not a directory — a browser, an object store — supplies
  bytes by IRI instead, and a stray `std::fs` would be found by the first one
  that tried.
- **Nothing is fetched.** The JSON-LD context is bundled in
  `crates/bridge/src/contexts/`. A crate naming another context fails to load
  instead of reaching the network.
- **Each unit gets a store of its own.** A mapping sees one unit, lifted with
  the unit as root, so no query can reach into another record.
- **Every dependency is pinned exactly**, `=x.y.z`, and `Cargo.lock` is
  committed. A range lets the engine change under a green run.
- **Every query is parsed once per adapter.** `prepare` parses and keeps the
  algebra; a unit clones it. Re-parsing per unit was a measured cost of a
  binding that exposes no prepared query.

## Conventions

- `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
  CI runs all three, and `cargo test -- --ignored` where it has checked out the
  counterpart a vector reads; there is no other build step.
- Conventional commits. Impersonal. No archaeology: what a file used to be is
  git's job.
- Why, never what. A comment restating the line below it goes.
