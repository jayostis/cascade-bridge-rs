//! **Cascade Bridge for Rust**: an implementation of the Cascade Bridge
//! Specification's `sparql-1.1` profile. It runs a Cascade Bridge Adapter, a
//! data package for one source format, over a source document, and executes
//! the adapter's test manifest, reading every file from a map held in memory.

mod accounting;
mod annotation;
mod budget;
mod compiled;
mod decode;
mod earl;
mod error;
#[cfg(test)]
mod fixtures;
mod harness;
mod json;
mod library;
mod lift;
mod load;
mod measure;
mod query;
mod rdf;
mod records;
mod resolver;
mod run;
mod shapes;
mod syntax;
pub mod terms;
mod validate;
mod vocabulary;
mod xpath;

pub use budget::{Budgets, Counted};
pub use error::{Error, ErrorKind, Map, Result};
pub use library::{
    describe, load, load_skipping, test, Conversion, ConvertOptions, Description, Document,
    Executor, Facts, Fallback, Files, Format, Loaded, Named, Outcome, Planned, QueryProfile,
    TestEntry, TestOptions, TestReport, NAME, VERSION,
};
pub use measure::{Skip, Stage, StageTime};
pub use oxrdf;
pub use oxrdfio;
