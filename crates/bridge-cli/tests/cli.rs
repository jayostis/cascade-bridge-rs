mod common;

use cascade_bridge::oxrdf::Term;
use cascade_bridge::oxrdfio::{self, RdfFormat};
use common::{
    canonical, canonical_lines, copied_to, map_of, names, outcomes, quads, read_at_its_own_iri,
    results, scratch, tiny, vocabularies, BASE, MAX_LENGTH,
};
use std::collections::{BTreeMap, BTreeSet};
use std::process::{Command, Stdio};

const QUALIFIED_ASSOCIATION: &str = "http://www.w3.org/ns/prov#qualifiedAssociation";
const AGENT: &str = "http://www.w3.org/ns/prov#agent";
const ADAPTER_IRI: &str = "https://example.org/adapters/catalog/";

/// A namespace no mapping query has reason to declare.
const OA: &str = "http://www.w3.org/ns/oa#";
const SH: &str = "http://www.w3.org/ns/shacl#";
/// The tiny adapter's gap scheme names its gaps here.
const GAPS: &str = "urn:example:catalog#";

const ENTRIES: [(&str, &str); 8] = [
    ("pass", "passed"),
    ("graph-fail", "failed"),
    ("findings-fail", "failed"),
    ("findings-repeated", "passed"),
    ("census", "passed"),
    ("shapes", "passed"),
    ("input-only", "cantTell"),
    ("dataset", "untested"),
];

fn cascade_bridge(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cascade-bridge"))
        .args(arguments)
        .output()
        .expect("run the command")
}

fn succeeded(run: &std::process::Output) {
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

/// The graph a text holds, canonicalised, the Bridge's release set aside as the harness sets it aside.
fn triples(bytes: &[u8], format: RdfFormat, base: &str) -> BTreeSet<String> {
    let quads = quads(bytes, format, base);
    let objects = |predicate: &str| -> Vec<(Term, Term)> {
        quads
            .iter()
            .filter(|quad| quad.predicate.as_str() == predicate)
            .map(|quad| (Term::from(quad.subject.clone()), quad.object.clone()))
            .collect()
    };
    let associations: BTreeSet<String> = objects(QUALIFIED_ASSOCIATION)
        .into_iter()
        .map(|(_, association)| association.to_string())
        .collect();
    let releases: BTreeSet<String> = objects(AGENT)
        .into_iter()
        .filter(|(association, _)| associations.contains(&association.to_string()))
        .map(|(_, release)| release.to_string())
        .collect();
    canonical_lines(
        quads
            .iter()
            .filter(|quad| !releases.contains(&quad.subject.to_string()))
            .cloned(),
    )
}

fn facts() -> std::path::PathBuf {
    tiny().join("fixtures/facts/catalog.ttl")
}

fn expected() -> BTreeSet<String> {
    let path = tiny().join("fixtures/expected/two.ttl");
    triples(
        &std::fs::read(&path).expect("the expected graph"),
        RdfFormat::Turtle,
        BASE,
    )
}

fn expected_findings() -> BTreeSet<String> {
    let path = tiny().join("fixtures/findings/two.ttl");
    canonical(
        &std::fs::read(&path).expect("the expected findings"),
        RdfFormat::Turtle,
        &cascade_bridge_cli::file_iri(&path).expect("the fixture's IRI"),
    )
}

fn entry_lines(stdout: &str) -> BTreeMap<String, String> {
    stdout
        .lines()
        .filter(|line| line.starts_with("  "))
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((
                fields.nth(1)?.to_owned(),
                line.split_whitespace().next()?.to_owned(),
            ))
        })
        .collect()
}

fn expected_outcomes() -> BTreeMap<String, String> {
    ENTRIES
        .iter()
        .map(|(name, outcome)| ((*name).to_owned(), (*outcome).to_owned()))
        .collect()
}

fn outcome_of_each_entry(report: &std::path::Path) -> BTreeMap<String, String> {
    let after_hash = |iri: String| iri.rsplit('#').next().unwrap_or_default().to_owned();
    outcomes(report)
        .into_iter()
        .map(|(entry, outcome)| (after_hash(entry), after_hash(outcome)))
        .collect()
}

fn refused_for_want_of_the_vocabularies(run: &std::process::Output) {
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("bridge:vocabularyFile") && stderr.contains("--vocabularies"),
        "the refusal names what the crate names and the flag that reads it: {stderr}"
    );
}

fn says_the_graph_went_unvalidated(stderr: &str) {
    assert!(
        stderr
            .lines()
            .any(|line| line.contains("2 bridge:vocabularyFile")
                && line.contains("--vocabularies")
                && line.contains("not validated")),
        "one line names the flag, the count of vocabulary files and the check not run: {stderr}"
    );
}

fn refused_with_the_usage(arguments: &[&str]) {
    let run = cascade_bridge(arguments);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(2), "{arguments:?}");
    assert!(
        stderr.starts_with("usage: cascade-bridge test"),
        "{arguments:?}: {stderr}"
    );
    assert!(run.stdout.is_empty(), "{arguments:?}");
}

fn adapter() -> String {
    tiny().to_string_lossy().into_owned()
}

fn tiny_with_vocabularies(command: &str) -> Vec<String> {
    vec![
        command.to_owned(),
        tiny().to_string_lossy().into_owned(),
        "--vocabularies".to_owned(),
        vocabularies().to_string_lossy().into_owned(),
    ]
}

fn run_in(directory: &std::path::Path, arguments: &[String]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cascade-bridge"))
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("run the command")
}

/// What `convert` says on standard error of the tiny adapter's two records.
fn said_converting_two() -> String {
    let document = tiny().join("fixtures/in/two.xml");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
    ]);
    succeeded(&run);
    String::from_utf8_lossy(&run.stderr).into_owned()
}

fn converted_with_no_vocabularies() -> std::process::Output {
    let document = tiny().join("fixtures/in/two.xml");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--facts",
        &facts().to_string_lossy(),
    ]);
    succeeded(&run);
    run
}

fn converted_with_findings_it_cannot_write() -> std::process::Output {
    let scratch = scratch();
    let document = tiny().join("fixtures/in/two.xml");
    let absent = scratch.path().join("no-such-directory/findings.ttl");
    cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
        "--findings",
        &absent.to_string_lossy(),
    ])
}

fn list_in_the_tiny_json_adapter(envelope: &str) -> std::process::Output {
    let adapter = tiny().join("../tiny-json-adapter");
    let list = adapter.join("fixtures/in/list.json");
    cascade_bridge(&[
        "convert",
        &adapter.to_string_lossy(),
        &list.to_string_lossy(),
        "--envelope",
        envelope,
    ])
}

fn tested_in(directory: &std::path::Path) -> std::process::Output {
    run_in(directory, &tiny_with_vocabularies("test"))
}

fn explained() -> String {
    let run = cascade_bridge(&[
        "explain",
        &tiny().to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
    ]);
    succeeded(&run);
    String::from_utf8_lossy(&run.stdout).into_owned()
}

fn said_profiling_two() -> String {
    let document = tiny().join("fixtures/in/two.xml");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
        "--profile",
    ]);
    succeeded(&run);
    String::from_utf8_lossy(&run.stderr).into_owned()
}

fn the_vocabulary() -> serde_json::Value {
    map_of(&vocabularies(), "https://example.org/vocabularies/", &[])
}

fn the_adapter_without(withheld: &[&str]) -> serde_json::Value {
    map_of(&tiny(), ADAPTER_IRI, withheld)
}

fn load_with_the_vocabulary(withheld: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "load": {
            "adapter": the_adapter_without(withheld),
            "vocabulary": the_vocabulary(),
        }
    })
}

fn test_with_the_vocabulary() -> serde_json::Value {
    serde_json::json!({
        "test": {
            "adapter": the_adapter_without(&[]),
            "vocabulary": the_vocabulary(),
        }
    })
}

fn the_document(path: &std::path::Path) -> serde_json::Value {
    serde_json::json!({
        "iri": "https://example.org/documents/catalog.xml",
        "path": path.to_string_lossy(),
    })
}

/// The results directory `library` wrote for a calls file, after it exited 0.
fn library(calls: serde_json::Value) -> BTreeMap<String, String> {
    let scratch = scratch();
    let file = scratch.path().join("calls.json");
    std::fs::write(&file, calls.to_string()).expect("the calls file");
    let directory = scratch.path().join("results");
    std::fs::create_dir(&directory).expect("an empty results directory");
    let run = cascade_bridge(&[
        "library",
        &file.to_string_lossy(),
        &directory.to_string_lossy(),
    ]);
    succeeded(&run);
    results(&directory)
}

fn in_results(held: &BTreeMap<String, String>, file: &str) -> String {
    match held.get(file) {
        Some(text) => text.clone(),
        None => panic!("no {file} in {held:#?}"),
    }
}

/// Converting a document with an adapter · cascade-bridge convert
///
/// `convert` turns one source document into a graph of records with an adapter, and prints on
/// standard error what it converted. Its exit status is all a calling program learns: 0 with the
/// graph written, 2 with nothing on standard output.
///
/// [engine/command.md](https://github.com/jayostis/cascade-bridge-spec/blob/main/engine/command.md#convert)
mod converting_a_document {
    use super::*;

    #[test]
    fn writes_to_standard_output_the_graph_the_adapter_expects_of_the_document() {
        let document = tiny().join("fixtures/in/two.xml");
        let run = cascade_bridge(&[
            "convert",
            &tiny().to_string_lossy(),
            &document.to_string_lossy(),
            "--facts",
            &facts().to_string_lossy(),
            "--vocabularies",
            &vocabularies().to_string_lossy(),
        ]);
        succeeded(&run);
        assert_eq!(
            triples(&run.stdout, RdfFormat::Turtle, BASE),
            expected(),
            "{}",
            String::from_utf8_lossy(&run.stdout)
        );
    }

    /// leaves the working directory as it was, and prints no `Graph` line, where no file is named
    #[test]
    fn leaves_the_working_directory_as_it_was_and_prints_no_graph_line_where_no_file_is_named() {
        let scratch = scratch();
        let document = tiny().join("fixtures/in/two.xml");
        let run = run_in(
            scratch.path(),
            &[
                "convert".to_owned(),
                tiny().to_string_lossy().into_owned(),
                document.to_string_lossy().into_owned(),
            ],
        );
        succeeded(&run);
        assert!(!run.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&run.stderr).contains("Graph"));
        let written: Vec<_> = std::fs::read_dir(scratch.path())
            .expect("the scratch directory")
            .collect();
        assert!(written.is_empty(), "{written:?}");
    }

    /// with `--out <file>`
    mod with_out {
        use super::*;

        #[test]
        fn writes_the_graph_to_the_file_and_nothing_to_standard_output() {
            let scratch = scratch();
            let document = tiny().join("fixtures/in/two.xml");
            let out = scratch.path().join("out.ttl");
            let run = cascade_bridge(&[
                "convert",
                &tiny().to_string_lossy(),
                &document.to_string_lossy(),
                "--facts",
                &facts().to_string_lossy(),
                "--vocabularies",
                &vocabularies().to_string_lossy(),
                "--out",
                &out.to_string_lossy(),
            ]);
            succeeded(&run);
            assert!(run.stdout.is_empty());
            assert_eq!(
                triples(
                    &std::fs::read(&out).expect("the written graph"),
                    RdfFormat::Turtle,
                    BASE,
                ),
                expected()
            );
        }
    }

    /// with `--format ntriples`
    mod with_format_ntriples {
        use super::*;

        /// writes the same graph as N-Triples
        #[test]
        fn writes_the_same_graph_as_n_triples() {
            let scratch = scratch();
            let document = tiny().join("fixtures/in/two.xml");
            let out = scratch.path().join("out.nt");
            let run = cascade_bridge(&[
                "convert",
                &tiny().to_string_lossy(),
                &document.to_string_lossy(),
                "--facts",
                &facts().to_string_lossy(),
                "--vocabularies",
                &vocabularies().to_string_lossy(),
                "--format",
                "ntriples",
                "--out",
                &out.to_string_lossy(),
            ]);
            succeeded(&run);
            assert_eq!(
                triples(
                    &std::fs::read(&out).expect("the written graph"),
                    RdfFormat::NTriples,
                    BASE,
                ),
                expected()
            );
        }
    }

    /// with `--envelope <iri>`
    mod with_envelope {
        use super::*;

        /// reads the document in the envelope the adapter's `ro-crate-metadata.json` names by that IRI
        #[test]
        fn reads_the_document_in_the_envelope_the_adapter_names_by_that_iri() {
            let run = list_in_the_tiny_json_adapter("#envelope-item");
            let said = String::from_utf8_lossy(&run.stderr);
            assert_eq!(run.status.code(), Some(0), "{said}");
            assert!(said.contains(" 1 record(s)"), "{said}");
        }

        /// refuses an envelope the adapter does not declare, with status 2
        #[test]
        fn refuses_an_envelope_the_adapter_does_not_declare_with_status_2() {
            let run = list_in_the_tiny_json_adapter("#envelope-none");
            let said = String::from_utf8_lossy(&run.stderr);
            assert_eq!(run.status.code(), Some(2), "{said}");
            assert!(said.contains("declares no envelope"), "{said}");
        }
    }

    /// with `--time-budget-seconds <n>`
    mod with_time_budget_seconds {
        use super::*;

        /// stops a conversion that goes over the budget, with status 2, naming the query it stopped and the budget
        #[test]
        fn stops_a_conversion_over_its_budget_with_status_2_naming_the_query_and_the_budget() {
            let document = tiny().join("fixtures/in/two.xml");
            let run = cascade_bridge(&[
                "convert",
                &tiny().to_string_lossy(),
                &document.to_string_lossy(),
                "--time-budget-seconds",
                "0.000001",
            ]);
            let said = String::from_utf8_lossy(&run.stderr);
            assert_eq!(run.status.code(), Some(2), "{said}");
            assert!(run.stdout.is_empty());
            assert!(said.starts_with("cascade-bridge: "), "{said}");
            assert!(
                said.contains(".rq was stopped: the convert outgrew its time budget of 1µs"),
                "{said}"
            );
        }
    }

    mod when_it_has_converted_the_document {
        use super::*;

        /// prints `Adapter` and the adapter's identifier
        #[test]
        fn prints_the_adapter_and_its_identifier() {
            let stderr = said_converting_two();
            assert!(stderr.contains("Adapter  catalog"), "{stderr}");
        }

        /// prints `Document`, the document's IRI and its count of records
        #[test]
        fn prints_the_document_its_iri_and_its_count_of_records() {
            let document = tiny().join("fixtures/in/two.xml");
            let iri = cascade_bridge_cli::file_iri(&document).expect("the document's IRI");
            let stderr = said_converting_two();
            assert!(
                stderr.contains(&format!("Document {iri}  2 record(s)")),
                "{stderr}"
            );
        }

        /// prints `Detect true` for a document the adapter's detect query claims
        #[test]
        fn prints_detect_true_for_a_document_the_adapters_detect_query_claims() {
            let stderr = said_converting_two();
            assert!(stderr.contains("Detect   true"), "{stderr}");
        }
    }
}

/// Writing a conversion's findings to a file · --findings
///
/// Findings are what a conversion noticed about a document: what the mapping could not carry over
/// and what the vocabulary's shapes refuse. `--findings <file>` writes them beside the graph, in a
/// form an adapter commits as the findings it expects of that document.
mod writing_findings {
    use super::*;

    #[test]
    fn writes_the_findings_the_adapter_expects_of_the_document() {
        let scratch = scratch();
        let document = tiny().join("fixtures/in/two.xml");
        let written = scratch.path().join("findings.ttl");
        let run = cascade_bridge(&[
            "convert",
            &tiny().to_string_lossy(),
            &document.to_string_lossy(),
            "--vocabularies",
            &vocabularies().to_string_lossy(),
            "--findings",
            &written.to_string_lossy(),
        ]);
        succeeded(&run);
        let produced = std::fs::read(&written).expect("the written findings");
        assert_eq!(
            // A findings file is read against its own IRI, as the harness reads a
            // committed oracle against the oracle's.
            canonical(
                &produced,
                RdfFormat::Turtle,
                &cascade_bridge_cli::file_iri(&written).expect("the written file's IRI")
            ),
            expected_findings(),
            "{}",
            String::from_utf8_lossy(&produced)
        );
    }

    /// names the document relative to the file, so a checkout at another path reads the same findings
    #[test]
    fn names_the_document_relative_to_the_file_so_a_checkout_at_another_path_reads_the_same() {
        let scratch = scratch();
        let elsewhere = scratch.path().join("another-checkout/tiny-adapter");
        copied_to(&tiny(), &elsewhere);
        let written = elsewhere.join("fixtures/findings/produced.ttl");
        let run = cascade_bridge(&[
            "convert",
            &elsewhere.to_string_lossy(),
            &elsewhere.join("fixtures/in/two.xml").to_string_lossy(),
            "--vocabularies",
            &vocabularies().to_string_lossy(),
            "--findings",
            &written.to_string_lossy(),
        ]);
        succeeded(&run);
        let produced = std::fs::read(&written).expect("the written findings");
        let oracle = tiny().join("fixtures/findings/two.ttl");
        assert_eq!(
            canonical(
                &produced,
                RdfFormat::Turtle,
                &cascade_bridge_cli::file_iri(&oracle).expect("the oracle's IRI")
            ),
            expected_findings(),
            "committed where the adapter's oracle stands, it names that checkout's document: {}",
            String::from_utf8_lossy(&produced)
        );
    }

    /// writes each IRI of the findings under a declared prefix, `oa:` and `sh:` among them
    #[test]
    fn writes_each_iri_of_the_findings_under_a_declared_prefix_oa_and_sh_among_them() {
        let scratch = scratch();
        let document = tiny().join("fixtures/in/two.xml");
        let written = scratch.path().join("findings-prefixed.ttl");
        let run = cascade_bridge(&[
            "convert",
            &tiny().to_string_lossy(),
            &document.to_string_lossy(),
            "--vocabularies",
            &vocabularies().to_string_lossy(),
            "--findings",
            &written.to_string_lossy(),
        ]);
        succeeded(&run);
        let bytes = std::fs::read(&written).expect("the written findings");
        let text = String::from_utf8(bytes).expect("utf-8");
        let mut parsed = oxrdfio::RdfParser::from_format(RdfFormat::Turtle)
            .with_base_iri(cascade_bridge_cli::file_iri(&written).expect("an IRI"))
            .expect("base")
            .for_slice(text.as_bytes());
        for quad in parsed.by_ref() {
            quad.expect("the findings parse as Turtle");
        }
        let prefixes: Vec<(String, String)> = parsed
            .prefixes()
            .map(|(name, iri)| (name.to_owned(), iri.to_owned()))
            .collect();
        let declared = |namespace: &str| {
            prefixes
                .iter()
                .find(|(_, iri)| iri == namespace)
                .map(|(name, _)| name.clone())
        };
        for (namespace, used) in [
            (OA, "hasTarget"),
            (SH, "resultSeverity"),
            (GAPS, "noteHasNoTerm"),
        ] {
            let name = declared(namespace)
                .unwrap_or_else(|| panic!("no prefix declared for {namespace}:\n{text}"));
            assert!(
                text.contains(&format!("{name}:{used}")),
                "{namespace} is declared as {name}: and not used:\n{text}"
            );
            assert_eq!(
                text.matches(&format!("<{namespace}")).count(),
                1,
                "an IRI in {namespace} is written in full beside its prefix declaration:\n{text}"
            );
        }
        assert_eq!(declared(OA).as_deref(), Some("oa"), "{text}");
        assert_eq!(declared(SH).as_deref(), Some("sh"), "{text}");
    }

    /// writes the same findings as N-Triples with `--format ntriples`
    #[test]
    fn writes_the_same_findings_as_n_triples_with_format_ntriples() {
        let scratch = scratch();
        let document = tiny().join("fixtures/in/two.xml");
        let written = |name: &str, format: &str| {
            let path = scratch.path().join(name);
            let run = cascade_bridge(&[
                "convert",
                &tiny().to_string_lossy(),
                &document.to_string_lossy(),
                "--vocabularies",
                &vocabularies().to_string_lossy(),
                "--out",
                &scratch
                    .path()
                    .join(format!("{name}.graph"))
                    .to_string_lossy(),
                "--findings",
                &path.to_string_lossy(),
                "--format",
                format,
            ]);
            succeeded(&run);
            path
        };
        let turtle = written("findings.ttl", "turtle");
        let ntriples = written("findings.nt", "ntriples");
        let as_ntriples = canonical(
            &std::fs::read(&ntriples).expect("the N-Triples findings"),
            RdfFormat::NTriples,
            BASE,
        );
        assert!(!as_ntriples.is_empty(), "the document draws findings");
        let turtle_text = std::fs::read(&turtle).expect("the Turtle findings");
        assert_eq!(
            canonical(
                &turtle_text,
                RdfFormat::Turtle,
                &cascade_bridge_cli::file_iri(&turtle).expect("an IRI")
            ),
            as_ntriples,
            "{}",
            String::from_utf8_lossy(&turtle_text)
        );
    }

    #[test]
    fn writes_to_standard_output_the_graph_it_writes_without_the_flag() {
        let scratch = scratch();
        let document = tiny().join("fixtures/in/two.xml");
        let written = scratch.path().join("findings-beside.ttl");
        let bare = cascade_bridge(&[
            "convert",
            &tiny().to_string_lossy(),
            &document.to_string_lossy(),
            "--vocabularies",
            &vocabularies().to_string_lossy(),
        ]);
        let beside = cascade_bridge(&[
            "convert",
            &tiny().to_string_lossy(),
            &document.to_string_lossy(),
            "--vocabularies",
            &vocabularies().to_string_lossy(),
            "--findings",
            &written.to_string_lossy(),
        ]);
        succeeded(&bare);
        succeeded(&beside);
        assert!(!bare.stdout.is_empty(), "the command wrote a graph");
        assert_eq!(
            canonical(&beside.stdout, RdfFormat::Turtle, BASE),
            canonical(&bare.stdout, RdfFormat::Turtle, BASE)
        );
    }

    /// prints `Findings` and the file's path, then `Graph` and its path, last on standard error
    #[test]
    fn prints_findings_and_the_files_path_then_graph_and_its_path_last_on_standard_error() {
        let scratch = scratch();
        let document = tiny().join("fixtures/in/two.xml");
        let findings = scratch.path().join("findings.ttl");
        let graph = scratch.path().join("graph.ttl");
        let mut arguments = tiny_with_vocabularies("convert");
        arguments.insert(2, document.to_string_lossy().into_owned());
        arguments.extend([
            "--findings".to_owned(),
            findings.to_string_lossy().into_owned(),
            "--out".to_owned(),
            graph.to_string_lossy().into_owned(),
        ]);
        let run = run_in(scratch.path(), &arguments);
        succeeded(&run);
        assert!(run.stdout.is_empty(), "the graph went to --out");
        let said = String::from_utf8_lossy(&run.stderr);
        assert!(
            said.ends_with(&format!(
                "Findings {}\nGraph    {}\n",
                findings.display(),
                graph.display()
            )),
            "{said}"
        );
    }

    mod when_the_findings_file_cannot_be_written {
        use super::*;

        #[test]
        fn exits_2_naming_the_file() {
            let run = converted_with_findings_it_cannot_write();
            assert_eq!(run.status.code(), Some(2));
            // The path it could not write, rather than the usage line an unknown flag
            // earns: the failure has to be the one this case is about.
            let stderr = String::from_utf8_lossy(&run.stderr);
            assert!(stderr.contains("findings.ttl"), "{stderr}");
            assert!(!stderr.contains("usage:"), "{stderr}");
        }

        #[test]
        fn writes_no_graph_to_standard_output() {
            let run = converted_with_findings_it_cannot_write();
            assert!(run.stdout.is_empty());
        }
    }
}

/// Validating against a checkout of the vocabulary · --vocabularies
///
/// An adapter names the vocabulary files its output must satisfy, each a `bridge:vocabularyFile`;
/// `--vocabularies <directory>` is a checkout that holds them. Without it nothing is validated, so
/// `test` refuses, and `convert` writes the graph and says it went unvalidated.
mod validating_against_the_vocabulary {
    use super::*;

    /// with `--vocabularies <directory>`
    mod with_vocabularies {
        use super::*;

        /// writes to `--findings` what the checkout's shapes draw on the graph
        #[test]
        fn writes_to_findings_what_the_checkouts_shapes_draw_on_the_graph() {
            let scratch = scratch();
            let document = tiny().join("fixtures/in/output-fails-a-shape.xml");
            let against = scratch.path().join("vocabularies.ttl");
            let run = cascade_bridge(&[
                "convert",
                &tiny().to_string_lossy(),
                &document.to_string_lossy(),
                "--findings",
                &against.to_string_lossy(),
                "--vocabularies",
                &vocabularies().to_string_lossy(),
            ]);
            succeeded(&run);
            let with = read_at_its_own_iri(&against, RdfFormat::Turtle);
            assert!(names(&with, MAX_LENGTH), "{with:?}");
        }
    }

    /// given an adapter that names vocabulary files, and no `--vocabularies`
    mod given_no_vocabularies {
        use super::*;

        /// refuses to `test`, with status 2, naming `bridge:vocabularyFile` and the flag
        #[test]
        fn refuses_to_test_with_status_2_naming_the_vocabulary_file_and_the_flag() {
            let run = cascade_bridge(&["test", &tiny().to_string_lossy()]);
            refused_for_want_of_the_vocabularies(&run);
        }

        #[test]
        fn converts_to_the_graph_the_adapter_expects() {
            let run = converted_with_no_vocabularies();
            assert_eq!(
                triples(&run.stdout, RdfFormat::Turtle, BASE),
                expected(),
                "{}",
                String::from_utf8_lossy(&run.stdout)
            );
        }

        /// says on one line that the graph was not validated, with the count of vocabulary files and the flag
        #[test]
        fn says_on_one_line_that_the_graph_was_not_validated_naming_the_flag_and_the_files() {
            let run = converted_with_no_vocabularies();
            says_the_graph_went_unvalidated(&String::from_utf8_lossy(&run.stderr));
        }

        /// refuses `--findings`, with status 2, writing neither the file nor a graph
        #[test]
        fn refuses_findings_with_status_2_writing_neither_the_file_nor_a_graph() {
            let scratch = scratch();
            let document = tiny().join("fixtures/in/output-fails-a-shape.xml");
            let bare = scratch.path().join("no-vocabulary.ttl");
            let run = cascade_bridge(&[
                "convert",
                &tiny().to_string_lossy(),
                &document.to_string_lossy(),
                "--findings",
                &bare.to_string_lossy(),
            ]);
            refused_for_want_of_the_vocabularies(&run);
            assert!(
                run.stdout.is_empty(),
                "{}",
                String::from_utf8_lossy(&run.stdout)
            );
            assert!(
                !bare.exists(),
                "an oracle missing what the shapes draw was written"
            );
        }
    }
}

/// Running an adapter's tests · cascade-bridge test
///
/// `test` runs every entry of an adapter's test manifest and prints each entry's outcome; an
/// adapter's CI runs it. It exits 1 where an entry failed.
///
/// [engine/command.md](https://github.com/jayostis/cascade-bridge-spec/blob/main/engine/command.md#test)
mod running_an_adapters_tests {
    use super::*;

    /// prints the adapter, then this Bridge and its version, before any entry
    #[test]
    fn prints_the_adapter_then_this_bridge_and_its_version_before_any_entry() {
        let scratch = scratch();
        let run = tested_in(scratch.path());
        let stdout = String::from_utf8_lossy(&run.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        assert!(
            lines
                .first()
                .is_some_and(|l| l.starts_with("Adapter  catalog")),
            "{stdout}"
        );
        let bridge = format!(
            "Bridge   Cascade Bridge for Rust {}, offers ",
            env!("CARGO_PKG_VERSION")
        );
        assert!(lines.iter().any(|l| l.starts_with(&bridge)), "{stdout}");
        let at = |starting: &str| {
            stdout
                .lines()
                .position(|line| line.starts_with(starting))
                .unwrap_or_else(|| panic!("no line starting {starting:?}: {stdout}"))
        };
        assert!(at("Adapter  catalog") < at("Bridge   "), "{stdout}");
        assert!(at("Bridge   ") < at("  passed "), "{stdout}");
    }

    /// prints a line for each entry, its outcome first
    #[test]
    fn prints_a_line_for_each_entry_with_its_outcome_first() {
        let scratch = scratch();
        let run = tested_in(scratch.path());
        let stdout = String::from_utf8(run.stdout).expect("utf-8");
        assert_eq!(entry_lines(&stdout), expected_outcomes(), "{stdout}");
    }

    #[test]
    fn prints_a_tally_of_the_outcomes_last() {
        let scratch = scratch();
        let run = tested_in(scratch.path());
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert_eq!(
            stdout.lines().last(),
            Some("4 passed, 2 failed, 1 cantTell, 1 untested"),
            "{stdout}"
        );
    }

    #[test]
    fn exits_1_when_an_entry_fails() {
        let scratch = scratch();
        let run = tested_in(scratch.path());
        assert_eq!(run.status.code(), Some(1), "an entry failed");
    }

    /// with `--earl <file>`
    mod with_earl {
        use super::*;

        /// writes the report to the file, an outcome for each entry
        #[test]
        fn writes_the_report_to_the_file_with_an_outcome_for_each_entry() {
            let scratch = scratch();
            let report = scratch.path().join("report.ttl");
            let mut arguments = tiny_with_vocabularies("test");
            arguments.extend(["--earl".to_owned(), report.to_string_lossy().into_owned()]);
            run_in(scratch.path(), &arguments);
            assert_eq!(outcome_of_each_entry(&report), expected_outcomes());
        }

        /// prints `EARL` and the file's path last
        #[test]
        fn prints_earl_and_the_files_path_last() {
            let scratch = scratch();
            let report = scratch.path().join("report.ttl");
            let mut arguments = tiny_with_vocabularies("test");
            arguments.extend(["--earl".to_owned(), report.to_string_lossy().into_owned()]);
            let run = run_in(scratch.path(), &arguments);
            let stdout = String::from_utf8_lossy(&run.stdout);
            assert!(
                stdout.ends_with(&format!("EARL     {}\n", report.display())),
                "{stdout}"
            );
        }
    }

    /// with no `--earl`
    mod with_no_earl {
        use super::*;

        #[test]
        fn writes_no_file() {
            let bare = scratch();
            tested_in(bare.path());
            let written: Vec<_> = std::fs::read_dir(bare.path())
                .expect("the scratch directory")
                .collect();
            assert!(written.is_empty(), "{written:?}");
        }
    }

    /// with `--fallbacks <file>`
    mod with_fallbacks {
        use super::*;

        /// writes each query the compiler did not take, by its path inside the adapter, with the reason
        #[test]
        fn writes_each_query_the_compiler_did_not_take_by_its_path_with_the_reason() {
            let scratch = scratch();
            let fallbacks = scratch.path().join("fallbacks.json");
            let mut arguments = tiny_with_vocabularies("test");
            arguments.extend([
                "--fallbacks".to_owned(),
                fallbacks.to_string_lossy().into_owned(),
            ]);
            run_in(scratch.path(), &arguments);
            let bytes = std::fs::read(&fallbacks).expect("the file --fallbacks named");
            let written: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
            assert_eq!(
                written,
                serde_json::json!([
                    {"query": "mapping/item-findings.rq", "reason": "MINUS"},
                    {"query": "mapping/detect.rq", "reason": "it is not a CONSTRUCT"},
                ]),
            );
        }
    }
}

/// Reading how an adapter's queries will run · cascade-bridge explain
///
/// The Bridge compiles an adapter's SPARQL queries into plans of its own, and runs a query it
/// cannot compile on a slower general evaluator. `explain` prints each query's plan, or why it has
/// none, so an adapter author sees what a query will cost before running it.
mod explaining_queries {
    use super::*;

    /// prints `Query` and each query's path inside the adapter
    #[test]
    fn prints_query_and_each_querys_path_inside_the_adapter() {
        let said = explained();
        assert!(
            said.lines().any(|line| line == "Query    mapping/item.rq"),
            "{said}"
        );
        for line in said.lines().filter(|line| line.starts_with("Query    ")) {
            assert!(!line.contains("file:"), "{line}");
        }
    }

    /// prints the plan of a query it compiles, a step to a line
    #[test]
    fn prints_the_plan_of_a_query_it_compiles_a_step_to_a_line() {
        let said = explained();
        assert!(said.lines().any(|line| line.starts_with("scan ")), "{said}");
        assert!(said.lines().any(|line| line == "construct"), "{said}");
    }

    /// prints `evaluated:` and the reason for a query it does not compile
    #[test]
    fn prints_evaluated_and_the_reason_for_a_query_it_does_not_compile() {
        let said = explained();
        let lines: Vec<&str> = said.lines().collect();
        let at = lines
            .iter()
            .position(|line| *line == "Query    mapping/item-findings.rq")
            .unwrap_or_else(|| panic!("no such query: {said}"));
        assert_eq!(lines.get(at + 1), Some(&"  evaluated: MINUS"), "{said}");
    }
}

/// Measuring a conversion · --profile, --skip
///
/// `--profile` prints where a conversion's time went, and `--skip` leaves out a check so the rest
/// can be timed. A run that skips is a measurement, not a validated conversion, and says so.
mod measuring_a_conversion {
    use super::*;

    /// with `--profile`
    mod with_profile {
        use super::*;

        /// prints `Load` and `Convert`, each with its time
        #[test]
        fn prints_load_and_convert_each_with_its_time() {
            let said = said_profiling_two();
            for heading in ["Load ", "Convert "] {
                assert!(
                    said.lines().any(|line| line.starts_with(heading)),
                    "{heading}: {said}"
                );
            }
        }

        /// prints a line for each stage: `loadQueries`, `lift`, `mapping` and `serialising`
        #[test]
        fn prints_a_line_for_each_stage_load_queries_lift_mapping_and_serialising() {
            let said = said_profiling_two();
            for stage in ["loadQueries", "lift", "mapping", "serialising"] {
                assert!(
                    said.lines()
                        .any(|line| line.trim_start().starts_with(stage)),
                    "{stage}: {said}"
                );
            }
        }

        /// prints a line `other`, for the time no stage counted
        #[test]
        fn prints_a_line_other_for_the_time_no_stage_counted() {
            let said = said_profiling_two();
            assert!(
                said.lines()
                    .any(|line| line.trim_start().starts_with("other")),
                "{said}"
            );
        }

        /// prints a line for each query: how often it ran, what it returned, its time, and whether it was compiled
        #[test]
        fn prints_a_line_for_each_query_with_its_runs_rows_time_and_whether_it_was_compiled() {
            let said = said_profiling_two();
            let queries: Vec<&str> = said
                .lines()
                .filter(|line| line.starts_with("Query    "))
                .collect();
            assert!(!queries.is_empty(), "{said}");
            for line in &queries {
                assert!(
                    line.contains(" run(s), ") && line.contains(" row(s), "),
                    "{line}"
                );
                assert!(
                    line.ends_with("compiled") || line.contains("evaluated"),
                    "{line}"
                );
            }
            assert!(
                queries.iter().any(|line| line.contains("item.rq")),
                "{said}"
            );
        }
    }

    /// with `--skip <check>`
    mod with_skip {
        use super::*;

        fn converted_skipping_findings_and_schema() -> std::process::Output {
            let document = tiny().join("fixtures/in/two.xml");
            let run = cascade_bridge(&[
                "convert",
                &tiny().to_string_lossy(),
                &document.to_string_lossy(),
                "--vocabularies",
                &vocabularies().to_string_lossy(),
                "--skip",
                "findings",
                "--skip",
                "schema",
            ]);
            succeeded(&run);
            run
        }

        /// warns `MEASUREMENT ONLY` on standard error, naming each check it skips
        #[test]
        fn warns_measurement_only_on_standard_error_naming_each_check_it_skips() {
            let run = converted_skipping_findings_and_schema();
            let said = String::from_utf8_lossy(&run.stderr);
            assert!(
                said.lines().any(|line| line.contains("MEASUREMENT ONLY")
                    && line.contains("findings, schema")
                    && line.contains("not a validated one")),
                "{said}"
            );
        }

        /// prints `Skipped` and the checks, once it has converted
        #[test]
        fn prints_skipped_and_the_checks_once_it_has_converted() {
            let run = converted_skipping_findings_and_schema();
            let said = String::from_utf8_lossy(&run.stderr);
            assert!(
                said.lines()
                    .any(|line| line.starts_with("Skipped  schema, findings")),
                "{said}"
            );
        }

        /// refuses `--findings`, with status 2, writing no file
        #[test]
        fn refuses_findings_with_status_2_writing_no_file() {
            let scratch = scratch();
            let document = tiny().join("fixtures/in/two.xml");
            let refused = scratch.path().join("findings.ttl");
            let run = cascade_bridge(&[
                "convert",
                &tiny().to_string_lossy(),
                &document.to_string_lossy(),
                "--vocabularies",
                &vocabularies().to_string_lossy(),
                "--skip",
                "shapes",
                "--findings",
                &refused.to_string_lossy(),
            ]);
            assert_eq!(run.status.code(), Some(2));
            assert!(
                String::from_utf8_lossy(&run.stderr).contains("--findings is not written"),
                "{}",
                String::from_utf8_lossy(&run.stderr)
            );
            assert!(!refused.exists());
        }
    }

    mod with_neither_flag {
        use super::*;

        #[test]
        fn says_nothing_of_a_measurement() {
            let said = said_converting_two();
            assert!(
                !said.contains("MEASUREMENT") && !said.contains("Skipped"),
                "{said}"
            );
        }
    }
}

/// Telling why a command stopped · cascade-bridge
///
/// A command that cannot do what it was asked exits 2 and says why on standard error, after
/// `cascade-bridge: `. Arguments it cannot parse get the usage instead.
mod telling_why_a_command_stopped {
    use super::*;

    mod given_arguments_it_cannot_parse {
        use super::*;

        /// prints the usage, which offers `--vocabularies` on `test`, `convert` and `explain`
        #[test]
        fn prints_the_usage_offering_vocabularies_on_each_command_that_loads_an_adapter() {
            let run = cascade_bridge(&["validate"]);
            let stderr = String::from_utf8_lossy(&run.stderr);
            for command in [
                "cascade-bridge test ",
                "cascade-bridge convert ",
                "cascade-bridge explain ",
            ] {
                let usage: Vec<&str> = stderr
                    .lines()
                    .filter(|line| line.contains(command))
                    .collect();
                assert!(!usage.is_empty(), "{command}: {stderr}");
                for line in usage {
                    assert!(line.contains("--vocabularies <directory>"), "{line}");
                }
            }
            assert!(stderr.contains("--format turtle|ntriples"), "{stderr}");
        }

        #[test]
        fn refuses_a_command_it_does_not_know() {
            refused_with_the_usage(&["validate"]);
        }

        /// refuses `test` with no adapter
        #[test]
        fn refuses_test_with_no_adapter() {
            refused_with_the_usage(&["test"]);
        }

        #[test]
        fn refuses_a_flag_with_no_value() {
            refused_with_the_usage(&["test", &adapter(), "--earl"]);
        }

        #[test]
        fn refuses_a_flag_it_does_not_know() {
            refused_with_the_usage(&["convert", &adapter(), "document.xml", "--nope"]);
        }

        /// refuses `--format rdfxml`
        #[test]
        fn refuses_a_format_it_does_not_write() {
            refused_with_the_usage(&["convert", &adapter(), "document.xml", "--format", "rdfxml"]);
        }

        /// refuses `--memory-budget-mib 0`
        #[test]
        fn refuses_a_memory_budget_of_zero() {
            refused_with_the_usage(&[
                "convert",
                &adapter(),
                "document.xml",
                "--memory-budget-mib",
                "0",
            ]);
        }

        /// refuses `--memory-budget-mib x`
        #[test]
        fn refuses_a_memory_budget_that_is_not_a_number() {
            refused_with_the_usage(&[
                "convert",
                &adapter(),
                "document.xml",
                "--memory-budget-mib",
                "x",
            ]);
        }

        /// refuses `--memory-budget-mib 0.5`
        #[test]
        fn refuses_a_memory_budget_that_is_not_whole() {
            refused_with_the_usage(&[
                "convert",
                &adapter(),
                "document.xml",
                "--memory-budget-mib",
                "0.5",
            ]);
        }

        /// refuses `--memory-budget-mib 4096`
        #[test]
        fn refuses_a_memory_budget_over_the_most_it_allows() {
            refused_with_the_usage(&[
                "convert",
                &adapter(),
                "document.xml",
                "--memory-budget-mib",
                "4096",
            ]);
        }

        /// refuses `--time-budget-seconds -1`
        #[test]
        fn refuses_a_negative_time_budget() {
            refused_with_the_usage(&[
                "convert",
                &adapter(),
                "document.xml",
                "--time-budget-seconds",
                "-1",
            ]);
        }

        /// refuses `--time-budget-seconds never`
        #[test]
        fn refuses_a_time_budget_that_is_not_a_number() {
            refused_with_the_usage(&["test", &adapter(), "--time-budget-seconds", "never"]);
        }

        /// refuses `--executor fast`
        #[test]
        fn refuses_an_executor_it_does_not_know() {
            refused_with_the_usage(&["convert", &adapter(), "document.xml", "--executor", "fast"]);
        }

        /// refuses `--skip everything`
        #[test]
        fn refuses_a_check_to_skip_it_does_not_know() {
            refused_with_the_usage(&[
                "convert",
                &adapter(),
                "document.xml",
                "--skip",
                "everything",
            ]);
        }

        /// refuses a third argument to `library`
        #[test]
        fn refuses_a_third_argument_to_library() {
            refused_with_the_usage(&["library", "calls.json", "results", "more"]);
        }
    }

    mod given_a_file_it_cannot_read {
        use super::*;

        /// names the document, after `cascade-bridge: `
        #[test]
        fn names_the_document_after_the_commands_name() {
            let scratch = scratch();
            let run = run_in(
                scratch.path(),
                &[
                    "convert".to_owned(),
                    tiny().to_string_lossy().into_owned(),
                    "no-such-document.xml".to_owned(),
                ],
            );
            assert_eq!(run.status.code(), Some(2));
            let said = String::from_utf8_lossy(&run.stderr);
            assert!(
                said.starts_with("cascade-bridge: no-such-document.xml: "),
                "{said}"
            );
        }

        /// names the `--facts` file, writing no graph
        #[test]
        fn names_the_facts_file_writing_no_graph() {
            let document = tiny().join("fixtures/in/two.xml");
            let missing = tiny().join("fixtures/facts/missing.ttl");
            let run = cascade_bridge(&[
                "convert",
                &tiny().to_string_lossy(),
                &document.to_string_lossy(),
                "--facts",
                &missing.to_string_lossy(),
            ]);
            let stderr = String::from_utf8_lossy(&run.stderr);
            assert_eq!(run.status.code(), Some(2), "{stderr}");
            assert!(run.stdout.is_empty());
            assert!(stderr.contains("missing.ttl"), "{stderr}");
        }
    }

    mod given_an_adapter_and_a_document_that_are_both_wrong {
        use super::*;

        /// names what is wrong with the adapter, not the document
        #[test]
        fn names_what_is_wrong_with_the_adapter_not_the_document() {
            let scratch = scratch();
            let adapter = scratch.path().join("adapter");
            std::fs::create_dir(&adapter).expect("the adapter's directory");
            std::fs::write(adapter.join("ro-crate-metadata.json"), "{}").expect("the crate");
            let missing = scratch.path().join("missing.xml");
            let run = cascade_bridge(&[
                "convert",
                &adapter.to_string_lossy(),
                &missing.to_string_lossy(),
            ]);
            assert_eq!(run.status.code(), Some(2));
            let stderr = String::from_utf8_lossy(&run.stderr);
            assert!(stderr.contains("names no root entity"), "{stderr}");
            assert!(!stderr.contains("missing.xml"), "{stderr}");
        }
    }

    mod when_standard_output_closes_before_the_graph_is_written {
        use super::*;

        /// exits 2 saying `standard output`, with no panic
        #[test]
        fn exits_2_saying_standard_output_with_no_panic() {
            let document = tiny().join("fixtures/in/two.xml");
            let mut child = Command::new(env!("CARGO_BIN_EXE_cascade-bridge"))
                .args([
                    "convert",
                    &tiny().to_string_lossy(),
                    &document.to_string_lossy(),
                    "--vocabularies",
                    &vocabularies().to_string_lossy(),
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("run the command");
            drop(child.stdout.take());
            let run = child.wait_with_output().expect("wait for the command");
            let stderr = String::from_utf8_lossy(&run.stderr);
            assert!(!stderr.contains("panicked"), "{stderr}");
            assert_eq!(run.status.code(), Some(2), "{stderr}");
            assert!(stderr.contains("standard output"), "{stderr}");
        }
    }
}

/// Answering a calls file for conformance · cascade-bridge library
///
/// `library` makes each call a calls file lists on the Bridge's library and writes what each
/// returned into a results directory; the specification's tooling runs its library cases through
/// it. It judges nothing.
///
/// [engine/command.md](https://github.com/jayostis/cascade-bridge-spec/blob/main/engine/command.md#library)
mod answering_a_calls_file {
    use super::*;

    /// writes `{}` for a `load` that held, and the answer for an `ask`
    #[test]
    fn writes_an_empty_answer_for_a_load_that_held_and_the_answer_for_an_ask() {
        let two = the_document(&tiny().join("fixtures/in/two.xml"));
        let held = library(serde_json::json!({ "cases": [
            { "name": "ask", "calls": [
                load_with_the_vocabulary(&["fixtures/"]),
                { "ask": { "document": two } } ] }
        ] }));
        assert_eq!(in_results(&held, "ask/1.json"), "{}");
        assert_eq!(in_results(&held, "ask/2.json"), r#"{"answer":true}"#);
    }

    /// writes the graph a `describe` or a `convert` returned, and the findings a conversion drew
    #[test]
    fn writes_the_graph_a_describe_or_a_convert_returned_and_the_findings_a_conversion_drew() {
        let two = the_document(&tiny().join("fixtures/in/two.xml"));
        let mut with_facts = two.clone();
        with_facts["facts"] = serde_json::json!({
            "iri": "https://example.org/facts/catalog.ttl",
            "path": facts().to_string_lossy(),
        });
        let held = library(serde_json::json!({ "cases": [
            { "name": "describe", "calls": [
                { "describe": {
                    "adapter": ADAPTER_IRI,
                    "metadata": tiny().join("ro-crate-metadata.json").to_string_lossy(),
                    "format": "turtle" } } ] },
            { "name": "convert", "calls": [
                load_with_the_vocabulary(&["fixtures/"]),
                { "convert": { "document": with_facts, "format": "ntriples" } } ] }
        ] }));
        for file in ["describe/1.graph", "convert/2.graph", "convert/2.findings"] {
            assert!(held.contains_key(file), "{file}: {held:#?}");
        }
    }

    #[test]
    fn writes_no_findings_for_a_conversion_with_no_vocabulary() {
        let two = the_document(&tiny().join("fixtures/in/two.xml"));
        let held = library(serde_json::json!({ "cases": [
            { "name": "convert", "calls": [
                { "load": { "adapter": the_adapter_without(&["fixtures/"]) } },
                { "convert": { "document": two, "format": "turtle" } } ] }
        ] }));
        assert!(held.contains_key("convert/2.graph"), "{held:#?}");
        assert!(!held.contains_key("convert/2.findings"), "{held:#?}");
    }

    /// writes a failure's kind for a call that failed
    #[test]
    fn writes_a_failures_kind_for_a_call_that_failed() {
        let scratch = scratch();
        let not_xml = scratch.path().join("not-xml.xml");
        std::fs::write(&not_xml, "<catalog><item").expect("a document that does not parse");
        let held = library(serde_json::json!({ "cases": [
            { "name": "document", "calls": [
                load_with_the_vocabulary(&["fixtures/"]),
                { "convert": { "document": the_document(&not_xml), "format": "turtle" } } ] },
            { "name": "vocabulary", "calls": [
                { "test": { "adapter": the_adapter_without(&[]) } } ] }
        ] }));
        assert_eq!(
            in_results(&held, "document/2.json"),
            r#"{"failure":{"kind":"documentFailure"}}"#
        );
        assert_eq!(
            in_results(&held, "vocabulary/1.json"),
            r#"{"failure":{"kind":"vocabularyFailure"}}"#
        );
    }

    #[test]
    fn writes_the_map_and_the_path_of_a_file_a_map_lacks() {
        let held = library(serde_json::json!({ "cases": [
            { "name": "missing", "calls": [
                load_with_the_vocabulary(&["fixtures/", "mapping/item.rq"]) ] }
        ] }));
        assert_eq!(
            in_results(&held, "missing/1.json"),
            r#"{"failure":{"kind":"fileMissingFailure","map":"adapter","path":"mapping/item.rq"}}"#
        );
    }

    /// writes nothing for a `convert` made before any `load`
    #[test]
    fn writes_nothing_for_a_convert_made_before_any_load() {
        let two = the_document(&tiny().join("fixtures/in/two.xml"));
        let held = library(serde_json::json!({ "cases": [
            { "name": "convert", "calls": [
                { "convert": { "document": two, "format": "turtle" } } ] }
        ] }));
        assert!(held.is_empty(), "{held:#?}");
    }

    /// writes the report a `test` returned
    #[test]
    fn writes_the_report_a_test_returned() {
        let held = library(serde_json::json!({ "cases": [
            { "name": "test", "calls": [
                test_with_the_vocabulary() ] }
        ] }));
        let report = in_results(&held, "test/1.report");
        assert_eq!(
            report.matches("http://www.w3.org/ns/earl#").count(),
            ENTRIES.len(),
            "{report}"
        );
    }
}
