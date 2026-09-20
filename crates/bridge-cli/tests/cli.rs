use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;
use std::path::PathBuf;
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
        stdout.contains("3 passed, 2 failed, 1 cantTell, 1 untested"),
        "{stdout}"
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
