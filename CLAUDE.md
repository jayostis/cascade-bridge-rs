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
- **Its own tests use the tiny adapters, `crates/bridge/tests/tiny-adapter`
  and `tiny-json-adapter`**, built so each outcome is reached by the smallest
  input that can reach it. The specification's synthetic adapters are
  met only through its vector, `specification_vector.rs`:
  `meets_the_specification_s_synthetic_adapter_only_through_its_vector`. A
  real adapter is met only through `compatibility.json`:
  `meets_a_real_adapter_only_through_the_compatibility_run`. An engine tested
  against the adapters it has met passes those adapters, not the contract.
- **The library never touches a filesystem**:
  `names_no_filesystem_outside_its_test_fixtures`. A host hands it maps of files
  instead, each keyed by path under the IRI its paths resolve against.
- **Nothing is fetched.** The JSON-LD context is bundled in
  `crates/bridge/src/contexts/`.
- **Each unit gets a store of its own**, so no query can reach into another
  record.
- **Every dependency is pinned exactly**, and `Cargo.lock` is committed:
  `pins_every_dependency_to_an_exact_version_or_a_git_rev`.
- **Every query is read once per `prepare`**:
  `reads_each_query_of_the_adapter_once_per_prepare`.

## Conventions

- `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`,
  `sh package/build.sh`, `cargo test`, `cargo test -p cascade-bridge-cli --test node_host`,
  `node --test package/tests/`. CI runs them all, and the specification's vector,
  `specification_vector.rs`, with `--include-ignored` where it has checked out the
  specification. build.sh is the one build step: it builds the package in
  `package/dist` that `node_host` and the Node tests load, and `cargo test`
  never builds or installs anything.
- Conventional commits. Impersonal.
