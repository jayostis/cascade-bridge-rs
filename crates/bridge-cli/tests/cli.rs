mod common;

use cascade_bridge::oxrdf::Term;
use cascade_bridge::oxrdfio::{self, RdfFormat};
use common::{
    canonical, canonical_lines, copied_to, names, quads, read_at_its_own_iri, scratch, tiny,
    vocabularies, BASE, MAX_LENGTH,
};
use std::collections::{BTreeMap, BTreeSet};
use std::process::{Command, Stdio};

const QUALIFIED_ASSOCIATION: &str = "http://www.w3.org/ns/prov#qualifiedAssociation";
const AGENT: &str = "http://www.w3.org/ns/prov#agent";

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

fn outcomes(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, outcome)| ((*name).to_owned(), (*outcome).to_owned()))
        .collect()
}

#[test]
fn prints_a_line_per_entry_and_exits_non_zero_when_an_entry_fails() {
    let run = cascade_bridge(&[
        "test",
        &tiny().to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
    ]);
    let stdout = String::from_utf8(run.stdout).expect("utf-8");
    assert_eq!(run.status.code(), Some(1), "{stdout}");
    assert!(stdout.starts_with("Adapter  catalog"), "{stdout}");
    assert_eq!(
        entry_lines(&stdout),
        outcomes(&[
            ("pass", "passed"),
            ("graph-fail", "failed"),
            ("findings-fail", "failed"),
            ("findings-repeated", "passed"),
            ("census", "passed"),
            ("shapes", "passed"),
            ("input-only", "cantTell"),
            ("dataset", "untested"),
        ]),
        "{stdout}"
    );
    assert!(
        stdout
            .lines()
            .any(|line| line == "4 passed, 2 failed, 1 cantTell, 1 untested"),
        "{stdout}"
    );
}

#[test]
fn offers_the_vocabularies_directory_on_both_commands() {
    let run = cascade_bridge(&["validate"]);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(
        stderr.matches("--vocabularies <directory>").count(),
        2,
        "{stderr}"
    );
}

#[test]
fn runs_the_manifest_against_the_vocabularies_directory_it_was_given() {
    let run = cascade_bridge(&[
        "test",
        &tiny().to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
    ]);
    let stdout = String::from_utf8(run.stdout).expect("utf-8");
    assert_eq!(
        run.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        stdout.contains("4 passed, 2 failed, 1 cantTell, 1 untested"),
        "the entry whose expected findings only the vocabulary draws passes: {stdout}"
    );
}

fn refused_for_want_of_the_vocabularies(run: &std::process::Output) {
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("bridge:vocabularyFile") && stderr.contains("--vocabularies"),
        "the refusal names what the crate names and the flag that reads it: {stderr}"
    );
}

#[test]
fn refuses_to_test_an_adapter_naming_vocabulary_files_without_the_vocabularies_directory() {
    let run = cascade_bridge(&["test", &tiny().to_string_lossy()]);
    refused_for_want_of_the_vocabularies(&run);
}

#[test]
fn converts_without_the_vocabularies_directory_and_says_the_graph_went_unvalidated() {
    let document = tiny().join("fixtures/in/two.xml");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--facts",
        &facts().to_string_lossy(),
    ]);
    succeeded(&run);
    assert_eq!(
        triples(&run.stdout, RdfFormat::Turtle, BASE),
        expected(),
        "{}",
        String::from_utf8_lossy(&run.stdout)
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr
            .lines()
            .any(|line| line.contains("2 bridge:vocabularyFile")
                && line.contains("--vocabularies")
                && line.contains("not validated")),
        "one line names the flag, the count of vocabulary files and the check not run: {stderr}"
    );
}

#[test]
fn writes_what_the_shapes_draw_given_the_vocabularies_directory_and_no_findings_without_it() {
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

#[test]
fn refuses_an_unknown_command_with_a_usage_line() {
    let run = cascade_bridge(&["validate"]);
    assert_eq!(run.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("usage: cascade-bridge test"), "{stderr}");
    assert!(stderr.contains("cascade-bridge convert"), "{stderr}");
}

#[test]
fn refuses_a_format_it_does_not_write() {
    let document = tiny().join("fixtures/in/two.xml");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--format",
        "rdfxml",
    ]);
    assert_eq!(run.status.code(), Some(2));
    assert!(run.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("--format turtle|ntriples"),
        "the refusal names the formats it does write: {stderr}"
    );
}

#[test]
fn converts_a_document_to_the_graph_the_adapter_expects_of_it() {
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

#[test]
fn names_a_facts_file_it_cannot_read_and_writes_no_graph() {
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

#[test]
fn says_the_adapter_the_records_and_the_detect_answer_on_standard_error() {
    let document = tiny().join("fixtures/in/two.xml");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
    ]);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("Adapter  catalog"), "{stderr}");
    assert!(stderr.contains("2 record(s)"), "{stderr}");
    assert!(stderr.contains("Detect   true"), "{stderr}");
}

#[test]
fn reports_a_standard_output_that_has_gone_away_rather_than_panicking() {
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
}

#[test]
fn writes_the_same_graph_as_n_triples_and_to_the_file_out_names() {
    let scratch = scratch();
    let document = tiny().join("fixtures/in/two.xml");
    let written = scratch.path().join("out.nt");
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
        &written.to_string_lossy(),
    ]);
    succeeded(&run);
    assert!(run.stdout.is_empty());
    assert_eq!(
        triples(
            &std::fs::read(&written).expect("the written graph"),
            RdfFormat::NTriples,
            BASE,
        ),
        expected()
    );
}

fn expected_findings() -> BTreeSet<String> {
    let path = tiny().join("fixtures/findings/two.ttl");
    canonical(
        &std::fs::read(&path).expect("the expected findings"),
        RdfFormat::Turtle,
        &cascade_bridge_cli::file_iri(&path).expect("the fixture's IRI"),
    )
}

#[test]
fn writes_the_findings_the_adapter_expects_of_the_document_where_findings_names() {
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

#[test]
fn writes_findings_the_adapter_can_commit_and_a_checkout_at_another_path_can_read() {
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
        "committed where the adapter's own oracle stands, it names that checkout's document: {}",
        String::from_utf8_lossy(&produced)
    );
}

#[test]
fn leaves_standard_output_the_graph_it_is_without_the_flag() {
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

#[test]
fn writes_both_files_as_n_triples_and_neither_to_standard_output() {
    let scratch = scratch();
    let document = tiny().join("fixtures/in/two.xml");
    let graph = scratch.path().join("graph.nt");
    let found = scratch.path().join("findings.nt");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--facts",
        &facts().to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
        "--out",
        &graph.to_string_lossy(),
        "--findings",
        &found.to_string_lossy(),
        "--format",
        "ntriples",
    ]);
    succeeded(&run);
    assert!(run.stdout.is_empty());
    assert_eq!(
        triples(
            &std::fs::read(&graph).expect("the written graph"),
            RdfFormat::NTriples,
            BASE,
        ),
        expected()
    );
    assert_eq!(
        canonical(
            &std::fs::read(&found).expect("the written findings"),
            RdfFormat::NTriples,
            BASE,
        ),
        expected_findings()
    );
}

#[test]
fn exits_non_zero_and_writes_no_graph_when_the_findings_file_cannot_be_written() {
    let scratch = scratch();
    let document = tiny().join("fixtures/in/two.xml");
    let absent = scratch.path().join("no-such-directory/findings.ttl");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
        "--findings",
        &absent.to_string_lossy(),
    ]);
    assert_ne!(run.status.code(), Some(0));
    assert!(run.stdout.is_empty());
    // The path it could not write, rather than the usage line an unknown flag
    // earns: the failure has to be the one this case is about.
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("findings.ttl"), "{stderr}");
    assert!(!stderr.contains("usage:"), "{stderr}");
}

/// A namespace no mapping query has reason to declare.
const OA: &str = "http://www.w3.org/ns/oa#";
const SH: &str = "http://www.w3.org/ns/shacl#";
/// The tiny adapter's gap scheme names its gaps here.
const GAPS: &str = "urn:example:catalog#";

#[test]
fn writes_findings_under_the_prefixes_a_findings_graph_uses() {
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
    let text =
        String::from_utf8(std::fs::read(&written).expect("the written findings")).expect("utf-8");
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

/// The command carrying `--format` through to the library's serialiser.
#[test]
fn writes_the_same_findings_graph_as_turtle_as_it_does_as_n_triples() {
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
fn names_what_is_wrong_with_the_adapter_where_the_document_is_missing_too() {
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

#[test]
fn test_says_the_adapter_this_bridge_at_its_own_version_and_the_tally() {
    let scratch = scratch();
    let run = run_in(scratch.path(), &tiny_with_vocabularies("test"));
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
    assert_eq!(
        lines.last(),
        Some(&"4 passed, 2 failed, 1 cantTell, 1 untested"),
        "{stdout}"
    );
    assert_eq!(run.status.code(), Some(1), "an entry failed");
}

#[test]
fn test_says_the_adapter_and_this_bridge_before_any_entry() {
    let scratch = scratch();
    let run = run_in(scratch.path(), &tiny_with_vocabularies("test"));
    let stdout = String::from_utf8_lossy(&run.stdout);
    let at = |starting: &str| {
        stdout
            .lines()
            .position(|line| line.starts_with(starting))
            .unwrap_or_else(|| panic!("no line starting {starting:?}: {stdout}"))
    };
    assert!(at("Adapter  catalog") < at("Bridge   "), "{stdout}");
    assert!(at("Bridge   ") < at("  passed "), "{stdout}");
}

#[test]
fn test_says_where_it_wrote_the_report_and_writes_none_where_none_was_asked_for() {
    let scratch = common::scratch();
    let report = scratch.path().join("report.ttl");
    let mut arguments = tiny_with_vocabularies("test");
    arguments.extend(["--earl".to_owned(), report.to_string_lossy().into_owned()]);
    let run = run_in(scratch.path(), &arguments);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.ends_with(&format!("EARL     {}\n", report.display())),
        "{stdout}"
    );
    assert!(report.is_file(), "the report --earl asked for");

    let bare = common::scratch();
    run_in(bare.path(), &tiny_with_vocabularies("test"));
    let written: Vec<_> = std::fs::read_dir(bare.path())
        .expect("the scratch directory")
        .collect();
    assert!(written.is_empty(), "{written:?}");
}

#[test]
fn refuses_what_it_cannot_parse_with_the_usage_and_status_two() {
    let adapter = tiny().to_string_lossy().into_owned();
    for argv in [
        &["validate"][..],
        &["test"],
        &["test", &adapter, "--earl"],
        &["convert", &adapter, "document.xml", "--format", "rdfxml"],
    ] {
        let run = cascade_bridge(argv);
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert_eq!(run.status.code(), Some(2), "{argv:?}");
        assert!(
            stderr.starts_with("usage: cascade-bridge test"),
            "{argv:?}: {stderr}"
        );
        assert!(run.stdout.is_empty(), "{argv:?}");
    }
}

#[test]
fn convert_says_it_wrote_the_findings_then_the_graph_and_names_the_document_by_its_iri() {
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
    let iri = cascade_bridge_cli::file_iri(&document).expect("the document's IRI");
    assert!(
        said.contains(&format!("Document {iri}  2 record(s)")),
        "{said}"
    );
    assert!(!read_at_its_own_iri(&findings, RdfFormat::Turtle).is_empty());
}

#[test]
fn convert_writes_the_graph_to_standard_output_where_no_file_is_named() {
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

#[test]
fn stops_with_status_two_and_the_reason_where_a_file_cannot_be_read() {
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

#[test]
fn convert_reads_the_document_in_the_envelope_named_as_the_crate_names_it() {
    let run = list_in_the_tiny_json_adapter("#envelope-item");
    let said = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(0), "{said}");
    assert!(said.contains(" 1 record(s)"), "{said}");
}

#[test]
fn convert_refuses_an_envelope_the_adapter_does_not_declare() {
    let run = list_in_the_tiny_json_adapter("#envelope-none");
    let said = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(2), "{said}");
    assert!(said.contains("declares no envelope"), "{said}");
}
