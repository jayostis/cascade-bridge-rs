# cascade-bridge-rs

[![compatibility](https://github.com/jayostis/cascade-bridge-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/jayostis/cascade-bridge-rs/actions/workflows/ci.yml?query=branch%3Amain)

**Cascade Bridge for Rust**: an implementation of the
[Cascade Bridge Specification](https://github.com/jayostis/cascade-bridge-spec).
It runs a Cascade Bridge Adapter, a data package for one source format, and
executes the adapter's test manifest. `cascade-bridge` with no arguments says
how.

## Status: DRAFT

No compatibility is promised. Which version of every repository a run uses is
[`compatibility.md`](https://github.com/jayostis/cascade-bridge-spec/blob/main/compatibility.md).
Not built yet: the stamp stage and streaming a referenced dataset.

It offers the `sparql-1.1` profile: a mapping is SPARQL 1.1 CONSTRUCT over a
generic lift of the source XML.

## Licence

Apache-2.0.
