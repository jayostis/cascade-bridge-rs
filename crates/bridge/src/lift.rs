use crate::error::Result;
use oxigraph::model::Quad;
use oxigraph::store::Store;
use std::collections::HashSet;

pub(crate) mod xml;

/// `within` reaches the path's first occurrence from the record, and is none for
/// the record itself or a value it carries directly.
pub(crate) struct Occurrence {
    pub(crate) path: String,
    pub(crate) within: Option<String>,
    pub(crate) count: usize,
}

pub(crate) struct Valued {
    pub(crate) path: String,
    pub(crate) value: String,
    pub(crate) within: Option<String>,
    pub(crate) count: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Paths {
    Kept { valued: HashSet<String> },
    Dropped,
}

/// `text` is the record written out on its own, for a validator and an address
/// check that bring their own parser.
pub(crate) struct Unit {
    pub(crate) store: Store,
    pub(crate) text: String,
    selector: String,
    occurrences: Vec<Occurrence>,
    values: Vec<Valued>,
}

impl Unit {
    pub(crate) fn selector(&self) -> String {
        self.selector.clone()
    }

    pub(crate) fn occurrences(&self) -> &[Occurrence] {
        &self.occurrences
    }

    pub(crate) fn values(&self) -> &[Valued] {
        &self.values
    }
}

/// One document being lifted: its records one at a time, then the skeleton the
/// detect query reads.
pub(crate) trait Lift {
    fn next_unit(&mut self) -> Result<Option<Unit>>;

    /// What an envelope is told apart by, once it has been read.
    fn document_root(&self) -> Option<&str>;

    /// `described` is the envelope's, for a document whose root was never read.
    fn document_selector(&self, described: Option<&str>) -> String;

    fn into_skeleton(self: Box<Self>) -> Result<Store>;
}

fn store_of(quads: Vec<Quad>) -> Result<Store> {
    let store = Store::new()?;
    store.extend(quads)?;
    Ok(store)
}
