# cascade-bridge-rs — Agent Context

The Cascade Bridge for Rust: it runs Cascade Bridge Adapters. The contract is
the Cascade Bridge Specification, `jayostis/cascade-bridge-spec`, and this
repository is one implementation of it, never a second statement of it.

## Documentation is a defect until proven otherwise

Anything that restates something else goes stale, so the default is to write
nothing. What something is and what it must do is carried, in this order, by:

1. **A name** that makes a comment unnecessary: a crate, a module, a type, a
   function, a test.
2. **Structure**: the crates, the modules, the types and what they let through.
3. **A test** whose name is the sentence and whose assertion is the rule.
4. **An error's sentence**, because a failing run prints it.
5. **Prose**, only for what none of the above can hold, and as short as it goes.

So:

- **No comment or doc comment that restates a name, a signature, a test or
  another file.** Rename instead.
- **No reference by number, count, position or line**: "the three stages",
  "below", `run.rs:120`. Name the thing, or link it.
- **No reasoning in files.** Why a change was made, and the alternative it did
  not take, go in the commit message.
- **Deleting prose is always in scope**, in any change, and preferred to editing it.

## The rules

- **The specification is the authority.** A test type's rule is the
  `rdfs:comment` on that type in the specification's `vocab/bridge.ttl`.
  Implement that, not a paraphrase. Where the specification is silent or
  wrong, the fix is a pull request there, not a behaviour invented here.
- **Nothing here pins another repository.** Which version of each
  one a run uses is `jayostis/cascade-bridge-spec`'s
  [`compatibility.md`](https://github.com/jayostis/cascade-bridge-spec/blob/main/compatibility.md).
- **Its own tests use only the tiny adapter,
  `crates/bridge/tests/tiny-adapter`**, built so each outcome is reached by the
  smallest input that can reach it; `meets_a_real_adapter_only_through_the_compatibility_run`.
  An engine tested against the adapters it has met passes those adapters, not
  the contract.
- **The library never touches a filesystem**:
  `lets_only_the_resolver_name_the_filesystem`. A host that is not a
  directory supplies bytes by IRI instead.
- **Nothing is fetched.** The JSON-LD context is bundled in
  `crates/bridge/src/contexts/`.
- **Each unit gets a store of its own**, so no query can reach into another
  record.
- **Every dependency is pinned exactly**, and `Cargo.lock` is committed:
  `pins_every_dependency_to_an_exact_version_or_a_git_rev`.
- **Every query is parsed once per adapter**:
  `reads_each_query_of_the_adapter_once_per_prepare`.

## Conventions

- `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`,
  `sh hosts/node/setup.sh`, `cargo test`. CI runs all four, and
  `cargo test -- --ignored` where it has checked out the counterpart a vector
  reads. setup.sh is the one build step: it builds the module the node host
  tests load, and `cargo test` never builds or installs anything.
- Conventional commits. Impersonal.
