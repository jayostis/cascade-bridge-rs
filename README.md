# cascade-bridge-rs

[![compatibility](https://github.com/jayostis/cascade-bridge-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/jayostis/cascade-bridge-rs/actions/workflows/ci.yml?query=branch%3Amain)

**Cascade Bridge for Rust**: an implementation of the
[Cascade Bridge Specification](https://github.com/jayostis/cascade-bridge-spec).
It runs a Cascade Bridge Adapter, a data package for one source format, and
executes the adapter's test manifest.

## Status: DRAFT

It implements the `sparql-1.1` profile: a mapping is SPARQL 1.1 CONSTRUCT over
a generic lift of the source XML. No compatibility is promised. Which version
of every repository a run uses is
[`compatibility.md`](https://github.com/jayostis/cascade-bridge-spec/blob/main/compatibility.md).

Built: loading an adapter, the lift, running the mappings and findings queries
per unit, XSD 1.0 validation of every record and document, the detect query,
converting a document a caller holds, and the test harness with EARL output.
Not built yet: the stamp stage and streaming a referenced dataset.

Known limitation: Oxigraph 0.5.11 returns derived XSD integer types such as
`xsd:positiveInteger` as `xsd:integer`, which SPARQL 1.1 does not allow, so an
expected graph that uses them cannot pass on this Bridge.

## Converting a document

```bash
cargo run -p cascade-bridge-cli -- convert <adapter-dir> <document.xml> [--out <file>] [--format turtle|ntriples]
```

Installed, the same command is
`cascade-bridge convert <adapter-dir> <document.xml>`.

It runs the adapter over every record of the document and writes their union as
one graph: prefixed Turtle on standard output, `--format ntriples` for a reader
that consumes a stream, `--out` to a file. The prefixes are the names the
adapter's own mappings give the namespaces the graph uses. Standard output
carries the graph and nothing else, so the run's adapter, record count and
detect answer go to standard error and the output pipes into a store as it
stands. The exit status is 0 when a graph was written, 2 on a usage error, an
adapter that cannot be loaded or a document that cannot be converted.

`bridge:detectQuery` is reported, never enforced: a document whose ASK answers
false is converted anyway, and the answer is on standard error.

What it does not do:

- **Findings are counted, not written.** Standard output carries the converted
  graph alone, so the findings a run made are a count on standard error. Which
  flag writes them out waits for a caller that wants one.
- **No provenance stamp**, because the stamp stage is not built. What it writes
  is the mapping's output, with nothing added.
- **The whole graph is held in memory**, so a document the size of a ClinVar
  release is out of scope.

## Running an adapter's tests

```bash
cargo run -p cascade-bridge-cli -- test <adapter-dir> [--earl report.ttl] [--datasets]
```

Installed, the same command is `cascade-bridge test <adapter-dir>`.

It prints one line per manifest entry and, with `--earl`, writes one
`earl:Assertion` per entry. The exit status is 0 when no entry failed and none
was inapplicable, 1 otherwise, and 2 on a usage error or an adapter that
cannot be loaded.

| outcome | when |
|---|---|
| `passed` | the rule the entry's type carries held |
| `failed` | it did not, or the entry could not be run |
| `cantTell` | an input-only entry: the output is recorded, never judged |
| `untested` | a dataset entry: datasets are not fetched |
| `inapplicable` | the adapter requires a profile this Bridge does not offer |

## What it does with an adapter

1. Loads `ro-crate-metadata.json` (JSON-LD) and the crate's test manifest
   (Turtle) as one graph, each with its own location as base. The RO-Crate
   context is bundled; nothing is fetched.
2. Parses every query the adapter names, once, and records each one's form.
3. Per document, lifts the XML in one pass, yielding one unit at a time as a
   store of its own. Triples go straight into the store: there is no
   N-Triples text between the parser and Oxigraph. The rest of the document
   becomes the skeleton the `bridge:detectQuery` ASK reads, finished when the
   last unit has been yielded.
4. Per unit: validates the record against `bridge:sourceSchema`, loads any
   Turtle `bridge:table` beside the lift, unions every `bridge:mapping`
   CONSTRUCT, and makes every `bridge:findingsQuery` CONSTRUCT's annotations
   about that record. A document is validated against its envelope's
   `bridge:documentSchema`. Validation reports; it never refuses.
5. Judges each manifest entry by its type's rule: graphs compared as RDFC-1.0
   canonical form after every `bridge:stampPredicate` triple is removed from
   both sides, and findings compared the same way.

## Two things the specification leaves open, and what this Bridge does

- **A name that is not ASCII.** The specification says every name in its lift
  vectors is ASCII and does not say how another character is written in an IRI.
  This Bridge percent-encodes each UTF-16 code unit outside `A-Za-z0-9_.-`; it
  is not a specified behaviour and may change when the specification settles it.
- **A document that is not UTF-8.** The specification does not mention input
  encoding. This Bridge honours the byte-order mark and the XML declaration. A
  UTF-8 document streams into the parser; a document in any other encoding is
  transcoded whole first.

## Layout

```
crates/bridge/                the library, cascade-bridge
  src/lift.rs                 XML to Facade-X-shaped triples: units and skeleton
  src/decode.rs               the byte-order mark and the XML declaration
  src/load.rs                 the crate and the manifest as one graph
  src/run.rs                  mappings and findings queries, per unit
  src/annotation.rs           a finding as a Web Annotation
  src/validate.rs             XSD 1.0, through the host for every include
  src/rdf.rs                  the shared terms, the canonical form, the serialiser
  src/harness.rs              executing a test manifest
  src/earl.rs                 the EARL report
  src/resolver.rs             the only module that touches a filesystem
  src/contexts/               bundled JSON-LD contexts
  tests/lift/                 the specification's lift vectors, copied
  tests/tiny-adapter/         the engine's own synthetic adapter
crates/bridge-cli/            the cascade-bridge command
```

## Dependencies

Exact versions, `Cargo.lock` committed, all open source: `oxigraph` 0.5.11 with
`oxrdf` 0.3.4, `oxrdfio` 0.2.6, `oxsdatatypes` 0.2.3 and `spargebra` 0.4.7, the
versions it pins itself (MIT OR Apache-2.0); `quick-xml` 0.37.5 (MIT);
`encoding_rs` 0.8.35 (Apache-2.0 OR MIT OR BSD-3-Clause); `oxiri` 0.2.11 and
`xsd-schema` 0.2.0 (MIT OR Apache-2.0). The bundled RO-Crate 1.2 context is CC0.

`oxrdf` carries the RDFC-1.0 canonicalisation the isomorphism comparison needs,
and `oxrdfio` the JSON-LD parser the crate is read with, so neither is a second
implementation of something Oxigraph already has.

`xsd-schema` carries its own `quick-xml` 0.41 beside the 0.37.5 the lift uses.
No `quick-xml` type crosses between them, so the two versions coexist.

## Licence

Apache-2.0.
