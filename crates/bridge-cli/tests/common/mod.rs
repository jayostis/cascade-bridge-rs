#![allow(dead_code)]

use cascade_bridge::oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use cascade_bridge::oxrdf::{self, Dataset, NamedOrBlankNode, Quad, Term};
use cascade_bridge::oxrdfio::{RdfFormat, RdfParser};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const EARL: &str = "http://www.w3.org/ns/earl#";
const DCT_TITLE: &str = "http://purl.org/dc/terms/title";

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
    canonical_lines(quads(bytes, format, base))
}

pub fn canonical_lines(quads: impl IntoIterator<Item = Quad>) -> BTreeSet<String> {
    let mut dataset = Dataset::new();
    for quad in quads {
        dataset.insert(&quad);
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
        hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
    });
    dataset.iter().map(|quad| quad.to_string()).collect()
}

pub fn read_at_its_own_iri(path: &Path, format: RdfFormat) -> Vec<Quad> {
    quads(
        &std::fs::read(path).expect("the written file"),
        format,
        &cascade_bridge_cli::file_iri(path).expect("the file's IRI"),
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

pub fn outcomes(report: &Path) -> BTreeMap<String, String> {
    let quads: Vec<_> = RdfParser::from_format(RdfFormat::Turtle)
        .for_slice(&std::fs::read(report).expect("the written report"))
        .map(|quad| quad.expect("a parsed report"))
        .collect();
    let object = |subject: &NamedOrBlankNode, predicate: &str| -> Option<Term> {
        quads
            .iter()
            .find(|q| &q.subject == subject && q.predicate.as_str() == predicate)
            .map(|q| q.object.clone())
    };
    let as_subject = |term: Term| match term {
        Term::NamedNode(n) => Some(NamedOrBlankNode::from(n)),
        Term::BlankNode(b) => Some(NamedOrBlankNode::from(b)),
        _ => None,
    };
    quads
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{EARL}test"))
        .map(|q| {
            let test = match &q.object {
                Term::NamedNode(n) => n.as_str().to_owned(),
                other => match object(&as_subject(other.clone()).expect("a node"), DCT_TITLE) {
                    Some(Term::Literal(title)) => title.value().to_owned(),
                    _ => panic!("an anonymous test with no title"),
                },
            };
            let result = object(&q.subject, &format!("{EARL}result"))
                .and_then(as_subject)
                .expect("a result");
            let outcome = match object(&result, &format!("{EARL}outcome")) {
                Some(Term::NamedNode(n)) => n.as_str().to_owned(),
                _ => panic!("a result with no outcome"),
            };
            (test, outcome)
        })
        .collect()
}

pub fn files_under(
    root: &Path,
    directory: &Path,
    into: &mut serde_json::Map<String, serde_json::Value>,
) {
    for entry in std::fs::read_dir(directory).expect("a directory") {
        let path = entry.expect("an entry").path();
        let key = path
            .strip_prefix(root)
            .expect("under the root")
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            files_under(root, &path, into);
        } else {
            into.insert(key, serde_json::json!(path.to_string_lossy()));
        }
    }
}

pub fn map_of(directory: &Path, iri: &str, withheld: &[&str]) -> serde_json::Value {
    let mut files = serde_json::Map::new();
    files_under(directory, directory, &mut files);
    files.retain(|key, _| !withheld.iter().any(|left| key.starts_with(left)));
    serde_json::json!({ "iri": iri, "files": files })
}

/// The answers, kinds and graphs a results directory holds, each graph canonicalised.
pub fn results(directory: &Path) -> BTreeMap<String, String> {
    let mut held = BTreeMap::new();
    for case in std::fs::read_dir(directory).expect("the results") {
        let case = case.expect("a case").path();
        for written in std::fs::read_dir(&case).expect("a case's results") {
            let written = written.expect("a result").path();
            let name = format!(
                "{}/{}",
                case.file_name().expect("a case name").to_string_lossy(),
                written.file_name().expect("a file name").to_string_lossy()
            );
            let bytes = std::fs::read(&written).expect("a result");
            let said = match written.extension().and_then(|e| e.to_str()) {
                Some("json") => {
                    let mut answer: serde_json::Value =
                        serde_json::from_slice(&bytes).expect("a JSON result");
                    if let Some(failure) = answer.get_mut("failure") {
                        failure
                            .as_object_mut()
                            .expect("a failure")
                            .remove("message");
                    }
                    answer.to_string()
                }
                Some("report") => format!("{:?}", outcomes(&written)),
                _ => format!(
                    "{:?}",
                    canonical(&bytes, RdfFormat::Turtle, "https://example.org/base")
                ),
            };
            held.insert(name, said);
        }
    }
    held
}
