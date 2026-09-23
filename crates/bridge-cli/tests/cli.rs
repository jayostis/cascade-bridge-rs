use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The stamp the tiny adapter's expected graph carries and no stage of this
/// Bridge writes yet: the harness drops it from both sides, and a converted
/// graph has to be read the same way until the stamp stage exists.
const STAMP: &str = "http://www.w3.org/ns/prov#generatedAtTime";

/// Every IRI in both graphs is absolute, so the base only has to be one.
const BASE: &str = "urn:example:base";

fn tiny() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bridge/tests/tiny-adapter")
}

fn cascade_bridge(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cascade-bridge"))
        .args(arguments)
        .output()
        .expect("run the command")
}

/// The graph a text holds, one line per triple, the stamp left out.
fn triples(bytes: &[u8], format: RdfFormat, base: &str) -> BTreeSet<String> {
    RdfParser::from_format(format)
        .with_base_iri(base)
        .expect("base")
        .for_slice(bytes)
        .map(|quad| quad.expect("a parsed graph"))
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

#[test]
fn prints_a_line_per_entry_and_exits_non_zero_when_an_entry_fails() {
    let run = cascade_bridge(&["test", &tiny().to_string_lossy()]);
    let stdout = String::from_utf8(run.stdout).expect("utf-8");
    assert_eq!(run.status.code(), Some(1), "{stdout}");
    let outcomes: Vec<(&str, &str)> = stdout
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?, fields.next()?))
        })
        .collect();
    assert!(outcomes.contains(&("passed", "pass")), "{stdout}");
    assert!(outcomes.contains(&("failed", "graph-fail")), "{stdout}");
    assert!(
        stdout.contains("3 passed, 3 failed, 1 cantTell, 1 untested"),
        "{stdout}"
    );
}

/// Where the engine command's `--vocabularies` argument points: the picked
/// checkout of `the-cascade-protocol/spec`, which the compatibility tooling
/// appends as it appends `--earl`.
fn vocabularies() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bridge/tests/tiny-vocabularies")
}

/// The constraint component the checkout's shapes draw on the tiny adapter's
/// produced graph, which nothing else in a run writes.
const MAX_LENGTH: &str = "http://www.w3.org/ns/shacl#MaxLengthConstraintComponent";

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
        "the entry whose expected findings only the vocabulary draws fails wherever \
         the checkout was not read, and the same command without this argument fails it: {stdout}"
    );
}

#[test]
fn writes_what_the_shapes_draw_only_where_it_was_given_the_vocabularies_directory() {
    let document = tiny().join("fixtures/in/output-fails-a-shape.xml");
    let against =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cascade-bridge-vocabularies.ttl");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--findings",
        &against.to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let produced = std::fs::read(&against).expect("the written findings");
    let lines = findings(&produced, RdfFormat::Turtle, BASE);
    assert!(
        lines.iter().any(|line| line.contains(MAX_LENGTH)),
        "{lines:?}"
    );

    let bare = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cascade-bridge-no-vocabulary.ttl");
    cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--findings",
        &bare.to_string_lossy(),
    ]);
    let without = findings(
        &std::fs::read(&bare).expect("the written findings"),
        RdfFormat::Turtle,
        BASE,
    );
    assert!(
        !without.iter().any(|line| line.contains(MAX_LENGTH)),
        "no checkout was named, so there is nothing to read the graph against: {without:?}"
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
}

#[test]
fn converts_a_document_to_the_graph_the_adapter_expects_of_it() {
    let document = tiny().join("fixtures/in/two.xml");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
    ]);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(0), "{stderr}");
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
    let document = tiny().join("fixtures/in/two.xml");
    let written = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cascade-bridge-convert-out.nt");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--format",
        "ntriples",
        "--out",
        &written.to_string_lossy(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
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

/// The findings a text holds, canonicalised, so an oracle and a run are
/// compared as graphs and a finding produced twice still counts twice.
fn findings(bytes: &[u8], format: RdfFormat, base: &str) -> BTreeSet<String> {
    cascade_bridge::canonical_lines(
        RdfParser::from_format(format)
            .with_base_iri(base)
            .expect("base")
            .for_slice(bytes)
            .map(|quad| quad.expect("a parsed graph")),
    )
    .expect("the findings as one canonical graph")
}

/// The oracle the tiny adapter commits for the document, read against its own
/// IRI so the document it names relatively is the document the entry names.
fn expected_findings() -> BTreeSet<String> {
    let path = tiny().join("fixtures/findings/two.ttl");
    findings(
        &std::fs::read(&path).expect("the expected findings"),
        RdfFormat::Turtle,
        &cascade_bridge::file_iri(&path).expect("the fixture's IRI"),
    )
}

/// The one that matters: the oracle a Bridge writes is the oracle a Bridge
/// judges.
#[test]
fn writes_the_findings_the_adapter_expects_of_the_document_where_findings_names() {
    let document = tiny().join("fixtures/in/two.xml");
    let written = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cascade-bridge-findings.ttl");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--findings",
        &written.to_string_lossy(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let produced = std::fs::read(&written).expect("the written findings");
    assert_eq!(
        // A findings file is read against its own IRI, as the harness reads a
        // committed oracle against the oracle's.
        findings(
            &produced,
            RdfFormat::Turtle,
            &cascade_bridge::file_iri(&written).expect("the written file's IRI")
        ),
        expected_findings(),
        "{}",
        String::from_utf8_lossy(&produced)
    );
}

/// The whole adapter where a different checkout would stand, so an oracle
/// written under one path can be read back under another.
fn copied_to(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the directory");
    for entry in std::fs::read_dir(from).expect("the directory") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copied_to(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a copied file");
        }
    }
}

/// The failure the flag exists to prevent. An oracle is written once and read
/// on every checkout afterwards, from whatever path that checkout stands at,
/// so a finding naming its document absolutely holds where it was written and
/// nowhere else — and the flag is worth nothing unless what it writes can be
/// committed as it stands.
#[test]
fn writes_findings_the_adapter_can_commit_and_a_checkout_at_another_path_can_read() {
    let elsewhere =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("another-checkout/tiny-adapter");
    if elsewhere.exists() {
        std::fs::remove_dir_all(&elsewhere).expect("a clean copy");
    }
    copied_to(&tiny(), &elsewhere);
    let written = elsewhere.join("fixtures/findings/produced.ttl");
    let run = cascade_bridge(&[
        "convert",
        &elsewhere.to_string_lossy(),
        &elsewhere.join("fixtures/in/two.xml").to_string_lossy(),
        "--findings",
        &written.to_string_lossy(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let produced = std::fs::read(&written).expect("the written findings");
    let oracle = tiny().join("fixtures/findings/two.ttl");
    assert_eq!(
        findings(
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
    let document = tiny().join("fixtures/in/two.xml");
    let written =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cascade-bridge-findings-beside.ttl");
    let bare = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
    ]);
    let beside = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--findings",
        &written.to_string_lossy(),
    ]);
    assert_eq!(beside.stdout, bare.stdout);
    assert_eq!(beside.status.code(), bare.status.code());
}

#[test]
fn writes_both_files_as_n_triples_and_neither_to_standard_output() {
    let document = tiny().join("fixtures/in/two.xml");
    let graph = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cascade-bridge-both-graph.nt");
    let found = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cascade-bridge-both-findings.nt");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        "--out",
        &graph.to_string_lossy(),
        "--findings",
        &found.to_string_lossy(),
        "--format",
        "ntriples",
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
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
        findings(
            &std::fs::read(&found).expect("the written findings"),
            RdfFormat::NTriples,
            BASE,
        ),
        expected_findings()
    );
}

#[test]
fn exits_non_zero_and_writes_no_graph_when_the_findings_file_cannot_be_written() {
    let document = tiny().join("fixtures/in/two.xml");
    let absent = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("cascade-bridge-no-such-directory/findings.ttl");
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
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

/// The bytes the command leaves at `path`, converting `document` through the
/// tiny adapter against the checkout, so a second run can be compared with a
/// first over the same path the way an author regenerates a file in place.
fn written_at(flag: &str, document: &Path, path: &Path) -> String {
    let run = cascade_bridge(&[
        "convert",
        &tiny().to_string_lossy(),
        &document.to_string_lossy(),
        flag,
        &path.to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::read_to_string(path).expect("the written file")
}

/// The defect a committed oracle's digest turns on: one binary, one input, one
/// adapter, seconds apart, and a different file each time.
#[test]
fn writes_one_input_s_findings_as_the_same_bytes_on_every_run() {
    let document = tiny().join("fixtures/in/two.xml");
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cascade-bridge-findings-twice.ttl");
    let first = written_at("--findings", &document, &path);
    let second = written_at("--findings", &document, &path);
    assert_eq!(first, second);
}

#[test]
fn writes_one_input_s_graph_as_the_same_bytes_on_every_run() {
    // The one input whose produced graph carries a blank node, which is what
    // there is here to write down two ways.
    let document = tiny().join("fixtures/in/output-fails-two-shapes.xml");
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cascade-bridge-out-twice.ttl");
    let first = written_at("--out", &document, &path);
    assert!(first.contains("_:"), "{first}");
    let second = written_at("--out", &document, &path);
    assert_eq!(first, second);
}

/// The oracles of `fixtures/findings/` this adapter's own run writes, each
/// named as its input is. `order-missing-a-note-finding` is not among them: it
/// is an oracle written wrong on purpose, and regenerating it would take the
/// entry that reads it green.
const ORACLES: [&str; 4] = ["two", "order", "unaccounted-child", "output-fails-a-shape"];

/// What the two rules of an adapter's `fixtures/` ask of a generator together:
/// an oracle is the Bridge's own output committed unedited, and a regeneration
/// that changed nothing is a regeneration that changed no byte.
#[test]
fn regenerates_the_adapter_s_own_oracles_byte_for_byte_and_leaves_the_run_where_it_was() {
    let elsewhere = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("regenerated/tiny-adapter");
    if elsewhere.exists() {
        std::fs::remove_dir_all(&elsewhere).expect("a clean copy");
    }
    copied_to(&tiny(), &elsewhere);
    let regenerated = || {
        ORACLES
            .iter()
            .map(|name| {
                let oracle = elsewhere.join(format!("fixtures/findings/{name}.ttl"));
                let run = cascade_bridge(&[
                    "convert",
                    &elsewhere.to_string_lossy(),
                    &elsewhere
                        .join(format!("fixtures/in/{name}.xml"))
                        .to_string_lossy(),
                    "--findings",
                    &oracle.to_string_lossy(),
                    "--vocabularies",
                    &vocabularies().to_string_lossy(),
                ]);
                assert_eq!(
                    run.status.code(),
                    Some(0),
                    "{}",
                    String::from_utf8_lossy(&run.stderr)
                );
                std::fs::read_to_string(&oracle).expect("the regenerated oracle")
            })
            .collect::<Vec<String>>()
    };
    // The copy keeps the layout an oracle names its input by, so what the
    // regeneration writes is what the committed file holds, byte for byte,
    // until the serialiser stops writing what was committed.
    let committed: Vec<String> = ORACLES
        .iter()
        .map(|name| {
            std::fs::read_to_string(tiny().join(format!("fixtures/findings/{name}.ttl")))
                .expect("the committed oracle")
        })
        .collect();
    assert_eq!(regenerated(), committed);
    assert_eq!(regenerated(), committed);

    let run = cascade_bridge(&[
        "test",
        &elsewhere.to_string_lossy(),
        "--vocabularies",
        &vocabularies().to_string_lossy(),
    ]);
    let stdout = String::from_utf8(run.stdout).expect("utf-8");
    assert!(
        stdout.contains("4 passed, 2 failed, 1 cantTell, 1 untested"),
        "the oracles a run regenerated are the oracles that run judges: {stdout}"
    );
}
