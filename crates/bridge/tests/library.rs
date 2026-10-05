use cascade_bridge::oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use cascade_bridge::oxrdf::{Dataset, Quad, Term};
use cascade_bridge::oxrdfio::{RdfFormat, RdfParser};
use cascade_bridge::{
    describe, Adapter, Conversion, Document, Facts, Files, Format, Kind, Map, Named, Result,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const ADAPTER: &str = "https://example.org/adapters/catalog/";
const VOCABULARY: &str = "https://example.org/vocabularies/";
const DOCUMENT: &str = "https://example.org/documents/catalog.xml";
const FACTS: &str = "https://example.org/facts/catalog.ttl";
const MAPPING: &str = "mapping/item.rq";
const VOCABULARY_FILE: &str = "ontologies/catalog/v1/catalog.ttl";
const OA_HAS_SOURCE: &str = "http://www.w3.org/ns/oa#hasSource";
const EARL_TEST: &str = "http://www.w3.org/ns/earl#test";

fn tests_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn files_under(root: &Path, directory: &Path, files: &mut Files) {
    for entry in std::fs::read_dir(directory).expect("read a directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            files_under(root, &path, files);
            continue;
        }
        let key = path
            .strip_prefix(root)
            .expect("under the root")
            .to_string_lossy()
            .replace('\\', "/");
        files.insert(key, std::fs::read(&path).expect("read a file"));
    }
}

fn files(directory: &str) -> Files {
    let root = tests_directory().join(directory);
    let mut files = Files::new();
    files_under(&root, &root, &mut files);
    files
}

fn tiny() -> Files {
    files("tiny-adapter")
}

fn tiny_to_load() -> Files {
    tiny()
        .into_iter()
        .filter(|(path, _)| !path.starts_with("fixtures/"))
        .collect()
}

fn vocabulary() -> Files {
    files("tiny-vocabularies")
}

fn fixture(path: &str) -> Vec<u8> {
    tiny()
        .remove(path)
        .unwrap_or_else(|| panic!("no {path} in the tiny adapter"))
}

fn named<'a>(files: &'a Files, iri: &'a str) -> Named<'a> {
    Named { iri, files }
}

fn document(bytes: &[u8]) -> Document<'_> {
    Document {
        iri: DOCUMENT,
        bytes,
        facts: None,
        envelope: None,
    }
}

fn kind<T>(result: Result<T>) -> Option<Kind> {
    result.err().map(|error| error.kind().clone())
}

fn load(adapter: &Files, vocabulary: &Files) -> Adapter {
    Adapter::load(named(adapter, ADAPTER), Some(named(vocabulary, VOCABULARY)))
        .unwrap_or_else(|error| panic!("the adapter does not load: {error}"))
}

fn convert(adapter: &Adapter, bytes: &[u8]) -> Conversion {
    adapter
        .convert(&document(bytes), Format::Turtle)
        .unwrap_or_else(|error| panic!("the document does not convert: {error}"))
}

fn quads(bytes: &[u8]) -> Vec<Quad> {
    RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(ADAPTER)
        .expect("base")
        .for_slice(bytes)
        .map(|quad| quad.expect("a parsed graph"))
        .collect()
}

fn canonical(bytes: &[u8]) -> BTreeSet<String> {
    let mut dataset = Dataset::new();
    for quad in quads(bytes) {
        dataset.insert(&quad);
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
        hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
    });
    dataset.iter().map(|quad| quad.to_string()).collect()
}

fn names_an_item(graph: &BTreeSet<String>) -> bool {
    graph
        .iter()
        .any(|line| line.contains("<urn:example:item:1>"))
}

fn objects_of(bytes: &[u8], predicate: &str) -> Vec<String> {
    quads(bytes)
        .into_iter()
        .filter(|quad| quad.predicate.as_str() == predicate)
        .map(|quad| match quad.object {
            Term::NamedNode(node) => node.into_string(),
            other => other.to_string(),
        })
        .collect()
}

fn the_first_failure(adapter: &Files, vocabulary: &Files) -> Option<Kind> {
    match Adapter::load(named(adapter, ADAPTER), Some(named(vocabulary, VOCABULARY))) {
        Err(error) => Some(error.kind().clone()),
        Ok(loaded) => {
            kind(loaded.convert(&document(&fixture("fixtures/in/two.xml")), Format::Turtle))
        }
    }
}

#[test]
fn describes_the_tiny_adapter_from_its_metadata_alone() {
    let tiny = tiny();
    let described = describe(&tiny["ro-crate-metadata.json"], ADAPTER)
        .unwrap_or_else(|error| panic!("the metadata is not described: {error}"));
    assert_eq!(described.identifier.as_deref(), Some("catalog"));
    assert_eq!(described.version.as_deref(), Some("1"));
    assert_eq!(
        described.source_media_type.as_deref(),
        Some("application/xml")
    );
    assert_eq!(
        described.envelopes,
        [format!("{ADAPTER}ro-crate-metadata.json#envelope-catalog")]
    );
    let to_load: BTreeSet<&str> = described.files_to_load.iter().map(String::as_str).collect();
    for query in [
        "mapping/item.rq",
        "mapping/item-code.rq",
        "mapping/item-shelf.rq",
        "mapping/item-tag.rq",
        "mapping/item-findings.rq",
        "mapping/item-note-findings.rq",
        "mapping/detect.rq",
    ] {
        assert!(to_load.contains(query), "{query} is not among {to_load:?}");
    }
    assert!(
        to_load.iter().all(|path| !path.starts_with("fixtures/")),
        "{to_load:?}"
    );
    assert!(
        to_load.iter().all(|path| tiny.contains_key(*path)),
        "a file to load that the adapter does not hold: {to_load:?}"
    );
    assert!(
        described
            .files_to_test
            .iter()
            .any(|path| path == "fixtures/manifest.ttl"),
        "{:?}",
        described.files_to_test
    );
    let mut vocabulary_files = described.vocabulary_files.clone();
    vocabulary_files.sort();
    assert_eq!(
        vocabulary_files,
        [
            "ontologies/catalog/v1/catalog.shapes.ttl",
            "ontologies/catalog/v1/catalog.ttl"
        ]
    );
}

#[test]
fn loads_with_the_test_manifest_and_every_fixture_left_out_and_then_converts() {
    let adapter = tiny_to_load();
    assert!(!adapter.contains_key("fixtures/manifest.ttl"));
    let loaded = load(&adapter, &vocabulary());
    let two = fixture("fixtures/in/two.xml");
    let facts = fixture("fixtures/facts/catalog.ttl");
    let converted = loaded
        .convert(
            &Document {
                facts: Some(Facts {
                    iri: FACTS,
                    bytes: &facts,
                }),
                ..document(&two)
            },
            Format::Turtle,
        )
        .unwrap_or_else(|error| panic!("the document does not convert: {error}"));
    assert!(
        names_an_item(&canonical(&converted.graph)),
        "{}",
        String::from_utf8_lossy(&converted.graph)
    );
}

#[test]
fn one_loaded_adapter_converts_two_documents_each_to_the_graph_a_fresh_load_gives() {
    let (adapter, vocabulary) = (tiny_to_load(), vocabulary());
    let loaded = load(&adapter, &vocabulary);
    for path in ["fixtures/in/two.xml", "fixtures/in/order.xml"] {
        let bytes = fixture(path);
        let again = canonical(&convert(&loaded, &bytes).graph);
        let fresh = canonical(&convert(&load(&adapter, &vocabulary), &bytes).graph);
        assert!(names_an_item(&fresh), "{path}: {fresh:?}");
        assert_eq!(again, fresh, "{path}");
    }
}

#[test]
fn accepts_a_catalog_and_not_a_document_the_detect_query_does_not_claim() {
    let loaded = load(&tiny_to_load(), &vocabulary());
    let two = fixture("fixtures/in/two.xml");
    let shelf = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shelf><item id=\"1\"/></shelf>\n";
    assert!(matches!(loaded.accepts(&document(&two)), Ok(true)));
    assert!(matches!(loaded.accepts(&document(shelf)), Ok(false)));
}

#[test]
fn a_document_that_does_not_parse_is_a_document_failure() {
    let loaded = load(&tiny_to_load(), &vocabulary());
    let failure = kind(loaded.convert(&document(b"<catalog><item"), Format::Turtle));
    assert_eq!(failure, Some(Kind::Document));
}

#[test]
fn facts_that_do_not_parse_are_a_facts_failure() {
    let loaded = load(&tiny_to_load(), &vocabulary());
    let two = fixture("fixtures/in/two.xml");
    let failure = kind(loaded.convert(
        &Document {
            facts: Some(Facts {
                iri: FACTS,
                bytes: b"this is not turtle",
            }),
            ..document(&two)
        },
        Format::Turtle,
    ));
    assert_eq!(failure, Some(Kind::Facts));
}

#[test]
fn a_mapping_that_does_not_parse_is_an_adapter_failure() {
    let mut adapter = tiny_to_load();
    adapter.insert(MAPPING.to_owned(), b"CONSTRUCT { nonsense".to_vec());
    assert_eq!(
        the_first_failure(&adapter, &vocabulary()),
        Some(Kind::Adapter)
    );
}

#[test]
fn a_vocabulary_file_that_does_not_parse_is_a_vocabulary_failure() {
    let mut vocabulary = vocabulary();
    vocabulary.insert(VOCABULARY_FILE.to_owned(), b"this is not turtle".to_vec());
    assert_eq!(
        the_first_failure(&tiny_to_load(), &vocabulary),
        Some(Kind::Vocabulary)
    );
}

#[test]
fn a_mapping_left_out_is_missing_from_the_adapter_by_its_path_and_loads_once_it_is_added() {
    let mut adapter = tiny_to_load();
    let mapping = adapter.remove(MAPPING).expect("the tiny adapter's mapping");
    let vocabulary = vocabulary();
    assert_eq!(
        the_first_failure(&adapter, &vocabulary),
        Some(Kind::Missing {
            map: Map::Adapter,
            path: MAPPING.to_owned()
        })
    );
    adapter.insert(MAPPING.to_owned(), mapping);
    assert_eq!(the_first_failure(&adapter, &vocabulary), None);
    let graph = canonical(
        &convert(
            &load(&adapter, &vocabulary),
            &fixture("fixtures/in/two.xml"),
        )
        .graph,
    );
    assert!(names_an_item(&graph), "{graph:?}");
}

#[test]
fn a_vocabulary_file_left_out_is_missing_from_the_vocabulary_by_its_path_and_loads_once_it_is_added(
) {
    let adapter = tiny_to_load();
    let mut vocabulary = vocabulary();
    let file = vocabulary
        .remove(VOCABULARY_FILE)
        .expect("the tiny vocabulary's file");
    assert_eq!(
        the_first_failure(&adapter, &vocabulary),
        Some(Kind::Missing {
            map: Map::Vocabulary,
            path: VOCABULARY_FILE.to_owned()
        })
    );
    vocabulary.insert(VOCABULARY_FILE.to_owned(), file);
    assert_eq!(the_first_failure(&adapter, &vocabulary), None);
    let graph = canonical(
        &convert(
            &load(&adapter, &vocabulary),
            &fixture("fixtures/in/two.xml"),
        )
        .graph,
    );
    assert!(names_an_item(&graph), "{graph:?}");
}

#[test]
fn a_document_failure_spoils_nothing_for_the_next_document() {
    let (adapter, vocabulary) = (tiny_to_load(), vocabulary());
    let loaded = load(&adapter, &vocabulary);
    assert_eq!(
        kind(loaded.convert(&document(b"<catalog><item"), Format::Turtle)),
        Some(Kind::Document)
    );
    let two = fixture("fixtures/in/two.xml");
    let after = canonical(&convert(&loaded, &two).graph);
    let fresh = canonical(&convert(&load(&adapter, &vocabulary), &two).graph);
    assert!(names_an_item(&fresh), "{fresh:?}");
    assert_eq!(after, fresh);
}

#[test]
fn names_no_file_iri_in_the_graph_the_findings_or_the_report_and_every_entry_under_the_adapter_iri()
{
    let vocabulary = vocabulary();
    let loaded = load(&tiny_to_load(), &vocabulary);
    let two = fixture("fixtures/in/two.xml");
    let facts = fixture("fixtures/facts/catalog.ttl");
    let converted = loaded
        .convert(
            &Document {
                facts: Some(Facts {
                    iri: FACTS,
                    bytes: &facts,
                }),
                ..document(&two)
            },
            Format::Turtle,
        )
        .unwrap_or_else(|error| panic!("the document does not convert: {error}"));
    let graph = String::from_utf8_lossy(&converted.graph);
    let findings = String::from_utf8_lossy(&converted.findings);
    assert!(names_an_item(&canonical(&converted.graph)), "{graph}");
    assert!(!graph.contains("file:"), "{graph}");
    assert!(!findings.contains("file:"), "{findings}");
    let sources = objects_of(&converted.findings, OA_HAS_SOURCE);
    assert!(
        !sources.is_empty() && sources.iter().all(|source| source == DOCUMENT),
        "each finding names the document by the IRI given: {sources:?}"
    );

    let tiny = tiny();
    let report = cascade_bridge::test(named(&tiny, ADAPTER), Some(named(&vocabulary, VOCABULARY)))
        .unwrap_or_else(|error| panic!("the adapter is not tested: {error}"));
    let earl = String::from_utf8_lossy(&report.earl);
    assert!(!earl.contains("file:"), "{earl}");
    let tested: BTreeSet<String> = objects_of(&report.earl, EARL_TEST).into_iter().collect();
    let entries: BTreeSet<String> = [
        "pass",
        "graph-fail",
        "findings-fail",
        "findings-repeated",
        "census",
        "shapes",
        "input-only",
        "dataset",
    ]
    .iter()
    .map(|name| format!("{ADAPTER}fixtures/manifest.ttl#{name}"))
    .collect();
    assert_eq!(tested, entries, "{earl}");
}
