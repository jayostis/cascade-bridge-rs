// Preparing an adapter is most of what a conversion test costs, so an adapter
// every test reads unchanged is prepared once per test thread rather than once
// per test. Per thread, because a compiled XML schema is not `Sync`.
#![allow(dead_code)]

use cascade_bridge::{
    convert, load_adapter, prepare, Conversion, DirectoryResolver, Prepared, Resolver, Result,
    Source,
};
use oxrdf::Quad;
use std::path::PathBuf;

pub fn tiny_directory() -> DirectoryResolver {
    DirectoryResolver::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"))
        .expect("resolver")
}

pub fn prepared(resolver: &dyn Resolver) -> Result<Prepared> {
    prepare(&load_adapter(resolver)?, resolver)
}

pub fn convert_input(
    prepared: &Prepared,
    resolver: &dyn Resolver,
    input: &str,
) -> Result<Conversion> {
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    let xml = resolver.read(&iri)?;
    convert(
        prepared,
        Source {
            iri: &iri,
            envelope: None,
            xml: &xml,
        },
    )
}

/// The whole run, from the crate to the conversion, as a result: an adapter may
/// be refused at any stage of it.
pub fn conversion(resolver: &dyn Resolver, input: &str) -> Result<Conversion> {
    convert_input(&prepared(resolver)?, resolver, input)
}

/// An adapter prepared once and converted as often as a test asks.
pub struct Subject<R> {
    pub resolver: R,
    pub prepared: Prepared,
}

impl<R: Resolver> Subject<R> {
    pub fn of(resolver: R) -> Self {
        let prepared = prepared(&resolver).expect("prepared");
        Self { resolver, prepared }
    }

    pub fn conversion(&self, input: &str) -> Conversion {
        convert_input(&self.prepared, &self.resolver, input).expect("conversion")
    }

    pub fn findings(&self, input: &str) -> Vec<Quad> {
        self.conversion(input).findings
    }
}

thread_local! {
    static ON_DISK: Subject<DirectoryResolver> = Subject::of(tiny_directory());
}

/// The tiny adapter as it stands on disk, converting one of its committed inputs.
pub fn on_disk(input: &str) -> Conversion {
    ON_DISK.with(|tiny| tiny.conversion(input))
}
