#![allow(dead_code)]

use oxrdf::Quad;
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Every IRI a graph written with no base names is absolute, so the base only
/// has to be one.
pub const BASE: &str = "urn:example:base";

/// The constraint component the checkout's shapes draw on the tiny adapter's
/// produced graph, which nothing else in a run writes.
pub const MAX_LENGTH: &str = "http://www.w3.org/ns/shacl#MaxLengthConstraintComponent";

pub fn tiny() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bridge/tests/tiny-adapter")
}

pub fn vocabularies() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bridge/tests/tiny-vocabularies")
}

/// A directory no earlier run wrote to, so a file left behind cannot stand in for one.
pub fn scratch() -> TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory")
}

pub fn quads(bytes: &[u8], format: RdfFormat, base: &str) -> Vec<Quad> {
    RdfParser::from_format(format)
        .with_base_iri(base)
        .expect("base")
        .for_slice(bytes)
        .map(|quad| quad.expect("a parsed graph"))
        .collect()
}

/// A graph canonicalised, so two are compared as graphs: a finding produced
/// twice is two annotations and still counts twice, though a triple written
/// twice counts once.
pub fn canonical(bytes: &[u8], format: RdfFormat, base: &str) -> BTreeSet<String> {
    cascade_bridge::canonical_lines(quads(bytes, format, base)).expect("one canonical graph")
}

pub fn read_at_its_own_iri(path: &Path, format: RdfFormat) -> Vec<Quad> {
    quads(
        &std::fs::read(path).expect("the written file"),
        format,
        &cascade_bridge::file_iri(path).expect("the file's IRI"),
    )
}

pub fn names(quads: &[Quad], iri: &str) -> bool {
    quads
        .iter()
        .any(|quad| matches!(&quad.object, oxrdf::Term::NamedNode(n) if n.as_str() == iri))
}

pub fn copied_to(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the copy's directory");
    for entry in std::fs::read_dir(from).expect("the directory to copy") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copied_to(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a copied file");
        }
    }
}
