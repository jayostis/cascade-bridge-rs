//! **Cascade Bridge for Rust**: an implementation of the Cascade Bridge
//! Specification's `sparql-1.1` profile. It runs a Cascade Bridge Adapter, a
//! data package for one source format, over a source document, and executes
//! the adapter's test manifest.
//!
//! The specification is the authority: a test type's rule is the
//! `rdfs:comment` on that type in its `vocab/bridge.ttl`.

mod annotation;
mod decode;
mod earl;
mod error;
mod harness;
mod lift;
mod load;
mod rdf;
mod resolver;
mod run;
mod validate;
mod xpath;

pub use earl::{earl_report, earl_report_at, ReportSubject};
pub use error::{Error, Result};
pub use harness::{run_manifest, EntryResult, Outcome, RunOptions, OFFERED_PROFILES};
pub use lift::{lift_slice, lift_text, Lift, Paths, Unit, FX, XYZ};
pub use load::{load_adapter, Adapter, Envelope};
pub use rdf::{canonical_lines, serialise, serialise_at, GraphFormat};
pub use resolver::{file_iri, DirectoryResolver, Resolver};
pub use run::{convert, prepare, Conversion, Form, Ms, Prepared, Source};
