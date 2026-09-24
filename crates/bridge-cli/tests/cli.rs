mod common;

use common::{
    canonical, copied_to, names, quads, read_at_its_own_iri, scratch, tiny, vocabularies, BASE,
    MAX_LENGTH,
};
use oxrdfio::RdfFormat;
use std::collections::{BTreeMap, BTreeSet};
use std::process::{Command, Stdio};

/// The stamp the tiny adapter's expected graph carries and no stage of this
/// Bridge writes yet: the harness drops it from both sides, and a converted
/// graph has to be read the same way until the stamp stage exists.
const STAMP: &str = "http://www.w3.org/ns/prov#generatedAtTime";

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

/// The graph a text holds, one line per triple, the stamp left out.
fn triples(bytes: &[u8], format: RdfFormat, base: &str) -> BTreeSet<String> {
    quads(bytes, format, base)
        .into_iter()
        .filter(|quad| quad.predicate.as_str() != STAMP)
        .map(|quad| quad.to_string())
        .collect()
}

fn expected() -> BTreeSet<String> {
    let path = tiny().join("fixtures/expected/two.ttl");
    triples(
        &std::fs::read(&path).expect("the expected graph"),
        RdfFormat::Turtle,
        BASE,
    )
}

/// Each entry's outcome, by its name, as `test` prints them.
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
fn writes_what_the_shapes_draw_given_the_vocabularies_directory_and_nothing_without_it() {
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

/// What `| head -5` does to the command: the reader goes away mid-graph, and
/// the documented statuses are the only ones a caller is given.
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

/// The oracle the tiny adapter commits for the document, read against its own
/// IRI so the document it names relatively is the document the entry names.
fn expected_findings() -> BTreeSet<String> {
    let path = tiny().join("fixtures/findings/two.ttl");
    canonical(
        &std::fs::read(&path).expect("the expected findings"),
        RdfFormat::Turtle,
        &cascade_bridge::file_iri(&path).expect("the fixture's IRI"),
    )
}

/// The one that matters: the oracle a Bridge writes is the oracle a Bridge
/// judges.
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
            &cascade_bridge::file_iri(&written).expect("the written file's IRI")
        ),
        expected_findings(),
        "{}",
        String::from_utf8_lossy(&produced)
    );
}

/// The failure the flag exists to prevent. An oracle is written once and read
/// on every checkout afterwards, from whatever path that checkout stands at,
/// so a finding naming its document absolutely holds where it was written and
/// nowhere else — and the flag is worth nothing unless what it writes can be
/// committed as it stands.
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
            &cascade_bridge::file_iri(&oracle).expect("the oracle's IRI")
        ),
        expected_findings(),
        "committed where the adapter's own oracle stands, it names that checkout's document: {}",
        String::from_utf8_lossy(&produced)
    );
}

#[test]
fn leaves_standard_output_byte_for_byte_what_it_is_without_the_flag() {
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
    assert_eq!(beside.stdout, bare.stdout);
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

/// The namespaces nearly every IRI of a findings graph is in, none of which a
/// mapping query has reason to declare.
const OA: &str = "http://www.w3.org/ns/oa#";
const SH: &str = "http://www.w3.org/ns/shacl#";
/// The tiny adapter's gap scheme names its gaps here.
const GAPS: &str = "urn:example:catalog#";

/// A committed oracle is what a reviewer reads, so a findings file names
/// what it can by a prefix rather than in full.
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
        .with_base_iri(cascade_bridge::file_iri(&written).expect("an IRI"))
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

/// Prefixes spell a graph; they may not change a triple of it. That every
/// committed input's findings read back the same both ways is the library's
/// serialiser to show; this is the command carrying `--format` through to it.
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
            &cascade_bridge::file_iri(&turtle).expect("an IRI")
        ),
        as_ntriples,
        "{}",
        String::from_utf8_lossy(&turtle_text)
    );
}
