//! **Cascade Bridge for Rust**: an implementation of the Cascade Bridge
//! Specification's `sparql-1.1` profile. It runs a Cascade Bridge Adapter, a
//! data package for one source format, over a source document, and executes
//! the adapter's test manifest.

mod accounting;
mod annotation;
pub mod command;
mod decode;
mod earl;
mod error;
#[cfg(test)]
mod fixtures;
mod harness;
mod lift;
mod load;
mod query;
mod rdf;
mod resolver;
mod run;
mod shapes;
pub mod terms;
mod validate;
mod vocabulary;
mod xpath;

pub use error::{Error, Result};
pub use oxrdf;
pub use oxrdfio;
pub use resolver::{
    authority, file_iri, file_iri_to_path, path_to_file_iri, unread, DirectoryResolver, Resolver,
};
