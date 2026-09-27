use crate::error::Result;
use oxigraph::model::{BlankNode, GraphName, NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;
use std::collections::HashSet;

pub(crate) mod json;
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

/// What admits a document to an envelope: its document element's local name, or the
/// name of a member of its value and that member's value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Admission {
    pub(crate) root: Option<String>,
    pub(crate) value: Option<String>,
    /// A JSON envelope's `bridge:jsonPathOfEachRecord`.
    pub(crate) records: Option<String>,
}

/// How a document is split: by the adapter's `bridge:elementNameOfEachRecord`, or by
/// the path of the envelope it is read in, `named` or the one that admits it.
pub(crate) struct Reading<'a> {
    pub(crate) element: Option<&'a str>,
    pub(crate) envelopes: Vec<&'a Admission>,
    pub(crate) named: Option<usize>,
}

/// The envelope named, or else the first that admits the document, one naming a
/// member's value before one naming none.
pub(crate) fn admitting(lift: &dyn Lift, reading: &Reading<'_>) -> Option<usize> {
    reading.named.or_else(|| {
        let admits = |(_, admission): &(usize, &&Admission)| lift.admits(admission);
        let valued = reading
            .envelopes
            .iter()
            .enumerate()
            .filter(|(_, admission)| admission.value.is_some())
            .find(admits);
        valued
            .or_else(|| reading.envelopes.iter().enumerate().find(admits))
            .map(|(index, _)| index)
    })
}

/// One document being lifted: its records one at a time, then the skeleton the
/// detect query reads.
pub(crate) trait Lift {
    fn next_unit(&mut self) -> Result<Option<Unit>>;

    /// Known of an XML document once its document element has been read.
    fn admits(&self, admission: &Admission) -> bool;

    /// `described` is the envelope's, for a document whose root was never read.
    fn document_selector(&self, described: Option<&str>) -> String;

    fn into_skeleton(self: Box<Self>) -> Result<Store>;
}

fn store_of(quads: Vec<Quad>) -> Result<Store> {
    let store = Store::new()?;
    store.extend(quads)?;
    Ok(store)
}

pub(crate) const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub(crate) const FX: &str = "http://sparql.xyz/facade-x/ns/";
pub(crate) const XYZ: &str = "http://sparql.xyz/facade-x/data/";

/// RFC 3987's `iunreserved`.
fn iunreserved(character: char) -> bool {
    matches!(character,
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~'
        | '\u{A0}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFEF}'
        | '\u{10000}'..='\u{1FFFD}' | '\u{20000}'..='\u{2FFFD}' | '\u{30000}'..='\u{3FFFD}'
        | '\u{40000}'..='\u{4FFFD}' | '\u{50000}'..='\u{5FFFD}' | '\u{60000}'..='\u{6FFFD}'
        | '\u{70000}'..='\u{7FFFD}' | '\u{80000}'..='\u{8FFFD}' | '\u{90000}'..='\u{9FFFD}'
        | '\u{A0000}'..='\u{AFFFD}' | '\u{B0000}'..='\u{BFFFD}' | '\u{C0000}'..='\u{CFFFD}'
        | '\u{D0000}'..='\u{DFFFD}' | '\u{E1000}'..='\u{EFFFD}')
}

/// `%` is never `iunreserved`, so two names never land on one IRI.
pub(crate) fn name(namespace: &str, local: &str) -> Result<NamedNode> {
    let mut iri = String::with_capacity(namespace.len() + local.len());
    iri.push_str(namespace);
    for character in local.chars() {
        if iunreserved(character) {
            iri.push(character);
            continue;
        }
        let mut octets = [0; 4];
        for octet in character.encode_utf8(&mut octets).as_bytes() {
            iri.push_str(&format!("%{octet:02X}"));
        }
    }
    Ok(NamedNode::new(iri)?)
}

pub(crate) fn member(index: usize) -> Result<NamedNode> {
    Ok(NamedNode::new(format!("{RDF}_{index}"))?)
}

pub(crate) fn triple(subject: &BlankNode, predicate: NamedNode, object: impl Into<Term>) -> Quad {
    Quad::new(
        NamedOrBlankNode::from(subject.clone()),
        predicate,
        object,
        GraphName::DefaultGraph,
    )
}
