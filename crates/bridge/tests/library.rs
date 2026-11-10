use cascade_bridge::oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use cascade_bridge::oxrdf::{Dataset, Quad, Term};
use cascade_bridge::oxrdfio::{RdfFormat, RdfParser};
use cascade_bridge::{
    describe, load as load_named, load_skipping, Budgets, Conversion, ConvertOptions, Description,
    Document, Error, ErrorKind, Executor, Facts, Files, Format, Loaded, Map, Named, QueryProfile,
    Result, Stage, TestOptions,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

const ADAPTER: &str = "https://example.org/adapters/catalog/";
const VOCABULARY: &str = "https://example.org/vocabularies/";
const DOCUMENT: &str = "https://example.org/documents/catalog.xml";
const FACTS: &str = "https://example.org/facts/catalog.ttl";
const MAPPING: &str = "mapping/item.rq";
const VOCABULARY_FILE: &str = "ontologies/catalog/v1/catalog.ttl";
const OA_HAS_SOURCE: &str = "http://www.w3.org/ns/oa#hasSource";
const EARL_TEST: &str = "http://www.w3.org/ns/earl#test";
const DOAP_REVISION: &str = "http://usefulinc.com/ns/doap#revision";

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

fn kind<T>(result: Result<T>) -> Option<ErrorKind> {
    result.err().map(|error| error.kind().clone())
}

fn load(adapter: &Files, vocabulary: &Files) -> Loaded {
    load_named(named(adapter, ADAPTER), Some(named(vocabulary, VOCABULARY)))
        .unwrap_or_else(|error| panic!("the adapter does not load: {error}"))
}

fn convert(adapter: &Loaded, bytes: &[u8]) -> Conversion {
    adapter
        .convert(&document(bytes))
        .unwrap_or_else(|error| panic!("the document does not convert: {error}"))
}

fn written(bytes: Result<Vec<u8>>) -> Vec<u8> {
    bytes.unwrap_or_else(|error| panic!("the graph is not written: {error}"))
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

fn described() -> Description {
    describe(ADAPTER, &tiny()["ro-crate-metadata.json"])
        .unwrap_or_else(|error| panic!("the metadata is not described: {error}"))
}

fn profiled_queries() -> (usize, Vec<QueryProfile>) {
    let loaded = load(&tiny_to_load(), &vocabulary());
    let two = fixture("fixtures/in/two.xml");
    let records = convert(&loaded, &two).records;
    let (_, queries) = loaded
        .convert_profiled(&document(&two))
        .unwrap_or_else(|error| panic!("the document does not convert: {error}"));
    (records, queries)
}

fn loaded_skipping_nothing() -> Loaded {
    let (adapter, vocabulary) = (tiny_to_load(), vocabulary());
    load_skipping(
        named(&adapter, ADAPTER),
        Some(named(&vocabulary, VOCABULARY)),
        &[],
    )
    .unwrap_or_else(|error| panic!("the adapter does not load: {error}"))
}

fn past_its_time_budget(loaded: &Loaded, bytes: &[u8]) -> Error {
    loaded
        .convert_with(
            &document(bytes),
            ConvertOptions {
                executor: Executor::Compiled,
                budgets: Budgets {
                    time: Duration::ZERO,
                    ..Budgets::default()
                },
            },
        )
        .err()
        .expect("a convert past its time budget stops")
}

fn the_first_failure(adapter: &Files, vocabulary: &Files) -> Option<ErrorKind> {
    match load_named(named(adapter, ADAPTER), Some(named(vocabulary, VOCABULARY))) {
        Err(error) => Some(error.kind().clone()),
        Ok(loaded) => kind(loaded.convert(&document(&fixture("fixtures/in/two.xml")))),
    }
}

/// Describing an adapter before loading it · describe(), Description
///
/// A host asks what an adapter is, and which of its files `load` will read, from the adapter's
/// metadata file alone. A host that reads files over a network then fetches only those.
mod describing_an_adapter_before_loading_it {
    use super::*;

    /// `describe(adapter_iri, metadata)`
    mod describe {
        use super::*;

        /// returns the adapter's identifier, version, source media type and envelopes
        #[test]
        fn returns_the_adapter_s_identifier_version_source_media_type_and_envelopes() {
            let described = described();
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
        }

        /// lists in `load_files` every query and no fixture, each a file the adapter holds
        #[test]
        fn lists_in_load_files_every_query_and_no_fixture_each_a_file_the_adapter_holds() {
            let described = described();
            let to_load: BTreeSet<&str> = described.load_files.iter().map(String::as_str).collect();
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
            let tiny = tiny();
            assert!(
                to_load.iter().all(|path| tiny.contains_key(*path)),
                "a file to load that the adapter does not hold: {to_load:?}"
            );
        }

        /// lists in `crate_files` the test manifest
        #[test]
        fn lists_in_crate_files_the_test_manifest() {
            let described = described();
            assert!(
                described
                    .crate_files
                    .iter()
                    .any(|path| path == "fixtures/manifest.ttl"),
                "{:?}",
                described.crate_files
            );
        }

        /// lists in `vocabulary_files` the vocabulary's files the adapter names
        #[test]
        fn lists_in_vocabulary_files_the_vocabulary_s_files_the_adapter_names() {
            let mut vocabulary_files = described().vocabulary_files;
            vocabulary_files.sort();
            assert_eq!(
                vocabulary_files,
                [
                    "ontologies/catalog/v1/catalog.shapes.ttl",
                    "ontologies/catalog/v1/catalog.ttl"
                ]
            );
        }
    }
}

/// Loading an adapter from maps of files · load(), Named, Files, Loaded
///
/// A host hands the Bridge an adapter, and the vocabulary it writes in, as maps of files held
/// in memory; the Bridge reads no disk. `load` refuses, by the file's name, an adapter it could
/// not convert with.
mod loading_an_adapter_from_maps_of_files {
    use super::*;

    /// `load(adapter, vocabulary)`
    mod load {
        use super::*;

        /// loads an adapter handed over without its test manifest and fixtures, and converts with it
        #[test]
        fn loads_an_adapter_handed_over_without_its_test_manifest_and_fixtures_and_converts_with_it(
        ) {
            let adapter = tiny_to_load();
            assert!(!adapter.contains_key("fixtures/manifest.ttl"));
            let loaded = load(&adapter, &vocabulary());
            let two = fixture("fixtures/in/two.xml");
            let facts = fixture("fixtures/facts/catalog.ttl");
            let converted = loaded
                .convert(&Document {
                    facts: Some(Facts {
                        iri: FACTS,
                        bytes: &facts,
                    }),
                    ..document(&two)
                })
                .unwrap_or_else(|error| panic!("the document does not convert: {error}"));
            assert!(
                names_an_item(&canonical(&written(converted.graph(Format::Turtle)))),
                "{}",
                String::from_utf8_lossy(&written(converted.graph(Format::Turtle)))
            );
        }
    }

    mod given_a_file_the_adapter_names_and_its_map_lacks {
        use super::*;

        /// fails with `Missing`, naming the adapter's map and the mapping's path, and loads once the mapping is added
        #[test]
        fn fails_with_missing_naming_the_adapter_s_map_and_the_mapping_s_path_and_loads_once_the_mapping_is_added(
        ) {
            let mut adapter = tiny_to_load();
            let mapping = adapter.remove(MAPPING).expect("the tiny adapter's mapping");
            let vocabulary = vocabulary();
            assert_eq!(
                the_first_failure(&adapter, &vocabulary),
                Some(ErrorKind::Missing {
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
                .graph(Format::Turtle)
                .expect("a graph written"),
            );
            assert!(names_an_item(&graph), "{graph:?}");
        }

        /// fails with `Missing`, naming the vocabulary's map and the file's path, and loads once the file is added
        #[test]
        fn fails_with_missing_naming_the_vocabulary_s_map_and_the_file_s_path_and_loads_once_the_file_is_added(
        ) {
            let adapter = tiny_to_load();
            let mut vocabulary = vocabulary();
            let file = vocabulary
                .remove(VOCABULARY_FILE)
                .expect("the tiny vocabulary's file");
            assert_eq!(
                the_first_failure(&adapter, &vocabulary),
                Some(ErrorKind::Missing {
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
                .graph(Format::Turtle)
                .expect("a graph written"),
            );
            assert!(names_an_item(&graph), "{graph:?}");
        }
    }
}

/// Converting a document · Loaded::accepts(), Loaded::convert(), Conversion
///
/// One loaded adapter converts any number of documents, each to a graph of records and a graph
/// of findings about the document. The document is named by the IRI the host gives, so nothing
/// written names a file on the host's disk.
mod converting_a_document {
    use super::*;

    /// `accepts(document)`
    mod accepts {
        use super::*;

        #[test]
        fn returns_true_for_a_document_the_detect_query_claims_and_false_for_one_it_does_not() {
            let loaded = load(&tiny_to_load(), &vocabulary());
            let two = fixture("fixtures/in/two.xml");
            let shelf = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shelf><item id=\"1\"/></shelf>\n";
            assert!(matches!(loaded.accepts(&document(&two)), Ok(true)));
            assert!(matches!(loaded.accepts(&document(shelf)), Ok(false)));
        }
    }

    /// `convert(document)`
    mod convert {
        use super::*;

        #[test]
        fn converts_each_of_two_documents_to_the_graph_a_fresh_load_gives() {
            let (adapter, vocabulary) = (tiny_to_load(), vocabulary());
            let loaded = load(&adapter, &vocabulary);
            for path in [
                "fixtures/in/two.xml",
                "fixtures/in/output-fails-a-shape.xml",
            ] {
                let bytes = fixture(path);
                let again = canonical(&written(convert(&loaded, &bytes).graph(Format::Turtle)));
                let fresh = canonical(&written(
                    convert(&load(&adapter, &vocabulary), &bytes).graph(Format::Turtle),
                ));
                assert!(names_an_item(&fresh), "{path}: {fresh:?}");
                assert_eq!(again, fresh, "{path}");
            }
        }

        /// converts the next document as a fresh load does, after a document that failed
        #[test]
        fn converts_the_next_document_as_a_fresh_load_does_after_a_document_that_failed() {
            let (adapter, vocabulary) = (tiny_to_load(), vocabulary());
            let loaded = load(&adapter, &vocabulary);
            assert_eq!(
                kind(loaded.convert(&document(b"<catalog><item"))),
                Some(ErrorKind::Document)
            );
            let two = fixture("fixtures/in/two.xml");
            let after = canonical(&written(convert(&loaded, &two).graph(Format::Turtle)));
            let fresh = canonical(&written(
                convert(&load(&adapter, &vocabulary), &two).graph(Format::Turtle),
            ));
            assert!(names_an_item(&fresh), "{fresh:?}");
            assert_eq!(after, fresh);
        }

        /// names no `file:` IRI in the graph or the findings, and the document by the IRI the host gave
        #[test]
        fn names_no_file_iri_in_the_graph_or_the_findings_and_the_document_by_the_iri_the_host_gave(
        ) {
            let vocabulary = vocabulary();
            let loaded = load(&tiny_to_load(), &vocabulary);
            let two = fixture("fixtures/in/two.xml");
            let facts = fixture("fixtures/facts/catalog.ttl");
            let converted = loaded
                .convert(&Document {
                    facts: Some(Facts {
                        iri: FACTS,
                        bytes: &facts,
                    }),
                    ..document(&two)
                })
                .unwrap_or_else(|error| panic!("the document does not convert: {error}"));
            let (graph_bytes, findings_bytes) = (
                written(converted.graph(Format::Turtle)),
                written(converted.findings(Format::Turtle, None)),
            );
            let graph = String::from_utf8_lossy(&graph_bytes);
            let findings = String::from_utf8_lossy(&findings_bytes);
            assert!(names_an_item(&canonical(&graph_bytes)), "{graph}");
            assert!(!graph.contains("file:"), "{graph}");
            assert!(!findings.contains("file:"), "{findings}");
            let sources = objects_of(&findings_bytes, OA_HAS_SOURCE);
            assert!(
                !sources.is_empty() && sources.iter().all(|source| source == DOCUMENT),
                "each finding names the document by the IRI given: {sources:?}"
            );
        }
    }
}

/// Telling whose failure an error is · Error, ErrorKind
///
/// Every failure says whose it is: the document's, the facts', the adapter's, the vocabulary's,
/// a missing file's, or the Bridge's own. A host tells a person "this file is damaged" and
/// sends an adapter's failure to its author.
mod telling_whose_failure_an_error_is {
    use super::*;

    /// `kind()`
    mod kind {
        use super::*;

        /// returns `Document` for a document that does not parse
        #[test]
        fn returns_document_for_a_document_that_does_not_parse() {
            let loaded = load(&tiny_to_load(), &vocabulary());
            let failure = kind(loaded.convert(&document(b"<catalog><item")));
            assert_eq!(failure, Some(ErrorKind::Document));
        }

        /// returns `Facts` for facts that do not parse
        #[test]
        fn returns_facts_for_facts_that_do_not_parse() {
            let loaded = load(&tiny_to_load(), &vocabulary());
            let two = fixture("fixtures/in/two.xml");
            let failure = kind(loaded.convert(&Document {
                facts: Some(Facts {
                    iri: FACTS,
                    bytes: b"this is not turtle",
                }),
                ..document(&two)
            }));
            assert_eq!(failure, Some(ErrorKind::Facts));
        }

        /// returns `Adapter` for a mapping that does not parse
        #[test]
        fn returns_adapter_for_a_mapping_that_does_not_parse() {
            let mut adapter = tiny_to_load();
            adapter.insert(MAPPING.to_owned(), b"CONSTRUCT { nonsense".to_vec());
            assert_eq!(
                the_first_failure(&adapter, &vocabulary()),
                Some(ErrorKind::Adapter)
            );
        }

        /// returns `Vocabulary` for a vocabulary file that does not parse
        #[test]
        fn returns_vocabulary_for_a_vocabulary_file_that_does_not_parse() {
            let mut vocabulary = vocabulary();
            vocabulary.insert(VOCABULARY_FILE.to_owned(), b"this is not turtle".to_vec());
            assert_eq!(
                the_first_failure(&tiny_to_load(), &vocabulary),
                Some(ErrorKind::Vocabulary)
            );
        }
    }
}

/// Stopping a convert at its budgets · Budgets, ConvertOptions, Loaded::convert_with()
///
/// A host gives a convert a time budget and a memory budget. A convert that passes either is
/// stopped as a failure of the Bridge that names the budget, and the adapter converts the next
/// document as before.
mod stopping_a_convert_at_its_budgets {
    use super::*;

    mod given_a_time_budget {
        use super::*;

        /// stops a convert past its time budget as a failure of the Bridge, naming the adapter and the budget
        #[test]
        fn stops_a_convert_past_its_time_budget_as_a_failure_of_the_bridge_naming_the_adapter_and_the_budget(
        ) {
            let loaded = load(&tiny_to_load(), &vocabulary());
            let stopped = past_its_time_budget(&loaded, &fixture("fixtures/in/two.xml"));
            assert_eq!(stopped.kind(), &ErrorKind::Bridge);
            assert!(
                stopped.message().starts_with(ADAPTER)
                    && stopped
                        .message()
                        .ends_with("outgrew its time budget of 0ns"),
                "{stopped}"
            );
        }

        #[test]
        fn converts_the_same_graph_afterwards_as_an_adapter_loaded_again() {
            let loaded = load(&tiny_to_load(), &vocabulary());
            let two = fixture("fixtures/in/two.xml");
            past_its_time_budget(&loaded, &two);
            let graph =
                |adapter: &Loaded| canonical(&written(convert(adapter, &two).graph(Format::Turtle)));
            let reloaded = load(&tiny_to_load(), &vocabulary());
            assert_eq!(graph(&loaded), graph(&reloaded));
            assert!(names_an_item(&graph(&reloaded)));
        }
    }
}

/// Measuring a load and a convert · load_skipping(), Loaded::convert_profiled(), Stage, Skip
///
/// For finding where time goes: a profiled convert times each stage and counts each query's
/// executions, and an adapter can be loaded to skip a stage. What a skipping adapter converts
/// says so, and is not a validated conversion.
mod measuring_a_load_and_a_convert {
    use super::*;

    /// `convert_profiled(document)`
    mod convert_profiled {
        use super::*;

        /// converts to the graph and findings `convert` gives
        #[test]
        fn converts_to_the_graph_and_findings_convert_gives() {
            let loaded = load(&tiny_to_load(), &vocabulary());
            let two = fixture("fixtures/in/two.xml");
            let plain = convert(&loaded, &two);
            let (profiled, _) = loaded
                .convert_profiled(&document(&two))
                .unwrap_or_else(|error| panic!("the document does not convert: {error}"));
            let findings =
                |conversion: &Conversion| canonical(&written(conversion.findings(Format::Turtle, None)));
            assert_eq!(
                canonical(&written(profiled.graph(Format::Turtle))),
                canonical(&written(plain.graph(Format::Turtle)))
            );
            assert!(!findings(&plain).is_empty());
            assert_eq!(findings(&profiled), findings(&plain));
        }

        /// counts each query's executions: one per record, and one for the detect query
        #[test]
        fn counts_each_query_s_executions_one_per_record_and_one_for_the_detect_query() {
            let (records, queries) = profiled_queries();
            let executions: BTreeSet<(String, usize)> = queries
                .iter()
                .map(|query| {
                    (
                        query.iri.trim_start_matches(ADAPTER).to_owned(),
                        query.executions,
                    )
                })
                .collect();
            let per_record = |path: &str| (path.to_owned(), records);
            assert_eq!(
                executions,
                BTreeSet::from([
                    per_record("mapping/item.rq"),
                    per_record("mapping/item-code.rq"),
                    per_record("mapping/item-shelf.rq"),
                    per_record("mapping/item-tag.rq"),
                    per_record("mapping/item-findings.rq"),
                    per_record("mapping/item-note-findings.rq"),
                    ("mapping/detect.rq".to_owned(), 1),
                ])
            );
        }

        #[test]
        fn counts_the_rows_a_mapping_answered() {
            let (_, queries) = profiled_queries();
            let item = queries
                .iter()
                .find(|query| query.iri == format!("{ADAPTER}{MAPPING}"))
                .expect("the item mapping is profiled");
            assert!(item.rows > 0, "{item:?}");
        }

        #[test]
        fn returns_no_stage_times_from_a_plain_load_or_a_plain_convert() {
            let (adapter, vocabulary) = (tiny_to_load(), vocabulary());
            let loaded = load(&adapter, &vocabulary);
            let two = fixture("fixtures/in/two.xml");
            assert!(convert(&loaded, &two).stages().is_none());
            assert!(loaded.load_stages().is_empty());
        }

        /// times serialising last, and only once the graph is written
        #[test]
        fn times_serialising_last_and_only_once_the_graph_is_written() {
            let loaded = loaded_skipping_nothing();
            let (profiled, _) = loaded
                .convert_profiled(&document(&fixture("fixtures/in/two.xml")))
                .unwrap_or_else(|error| panic!("the document does not convert: {error}"));
            let serialising = |conversion: &Conversion| {
                let stages = conversion.stages().expect("a profiled convert");
                assert_eq!(
                    stages.last().map(|time| time.stage),
                    Some(Stage::Serialising)
                );
                stages.last().expect("a stage").elapsed
            };
            assert_eq!(serialising(&profiled), Duration::ZERO);
            written(profiled.graph(Format::Turtle));
            assert!(serialising(&profiled) > Duration::ZERO);
        }
    }

    /// `load_skipping(adapter, vocabulary, skips)`
    mod load_skipping {
        use super::*;

        /// times a load made with `load_skipping`
        #[test]
        fn times_a_load_made_with_load_skipping() {
            assert!(!loaded_skipping_nothing().load_stages().is_empty());
        }
    }
}

/// Running an adapter's test manifest · test(), TestOptions, TestReport
///
/// An adapter carries a manifest of test entries, each a source document and what converting it
/// must give. `test` runs them and returns an EARL report (the W3C format for test results),
/// which the compatibility run and the adapters page read.
mod running_an_adapter_s_test_manifest {
    use super::*;

    /// given the EARL report
    mod given_the_earl_report {
        use super::*;

        /// names no `file:` IRI, and each entry under the adapter's IRI
        #[test]
        fn names_no_file_iri_and_each_entry_under_the_adapter_s_iri() {
            let (tiny, vocabulary) = (tiny(), vocabulary());
            let report = cascade_bridge::test(
                named(&tiny, ADAPTER),
                Some(named(&vocabulary, VOCABULARY)),
                TestOptions::default(),
            )
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

        /// names this Bridge at its own version
        #[test]
        fn names_this_bridge_at_its_own_version() {
            let (tiny, vocabulary) = (tiny(), vocabulary());
            let report = cascade_bridge::test(
                named(&tiny, ADAPTER),
                Some(named(&vocabulary, VOCABULARY)),
                TestOptions::default(),
            )
            .unwrap_or_else(|error| panic!("the adapter is not tested: {error}"));
            let revisions: Vec<String> = quads(&report.earl)
                .into_iter()
                .filter(|quad| quad.predicate.as_str() == DOAP_REVISION)
                .map(|quad| match quad.object {
                    Term::Literal(revision) => revision.value().to_owned(),
                    other => panic!("a revision that is not a literal: {other}"),
                })
                .collect();
            assert_eq!(revisions, [env!("CARGO_PKG_VERSION")]);
        }
    }
}
