mod common;

use cascade_bridge::command;
use common::{tiny_directory, tiny_with_vocabularies, vocabularies_directory};
use oxrdf::Term;
use oxrdfio::{RdfFormat, RdfParser};

const DOAP_REVISION: &str = "http://usefulinc.com/ns/doap#revision";

fn arguments(earl: Option<&str>) -> command::Test {
    command::Test {
        directory: tiny_directory().to_string_lossy().into_owned(),
        vocabularies: Some(vocabularies_directory().to_string_lossy().into_owned()),
        earl: earl.map(str::to_owned),
        datasets: false,
    }
}

#[test]
fn test_says_the_adapter_this_bridge_at_its_own_version_and_the_tally() {
    let run = command::test(&arguments(None), &tiny_with_vocabularies()).expect("a run");
    let lines: Vec<&str> = run.summary.lines().collect();
    assert!(
        lines
            .first()
            .is_some_and(|l| l.starts_with("Adapter  catalog")),
        "{}",
        run.summary
    );
    let bridge = format!(
        "Bridge   Cascade Bridge for Rust {}, offers ",
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        lines.iter().any(|l| l.starts_with(&bridge)),
        "{}",
        run.summary
    );
    assert_eq!(
        lines.last(),
        Some(&"4 passed, 2 failed, 1 cantTell, 1 untested"),
        "{}",
        run.summary
    );
}

#[test]
fn test_reports_this_bridge_at_its_own_version() {
    let run =
        command::test(&arguments(Some("report.ttl")), &tiny_with_vocabularies()).expect("a run");
    let report = run.earl.expect("the report --earl asked for");
    let revisions: Vec<String> = RdfParser::from_format(RdfFormat::Turtle)
        .for_slice(report.as_bytes())
        .map(|quad| quad.expect("a parsed report"))
        .filter(|quad| quad.predicate.as_str() == DOAP_REVISION)
        .map(|quad| match quad.object {
            Term::Literal(revision) => revision.value().to_owned(),
            other => panic!("a revision that is not a literal: {other}"),
        })
        .collect();
    assert_eq!(revisions, [env!("CARGO_PKG_VERSION")]);
}

#[test]
fn test_writes_no_report_where_none_was_asked_for() {
    let run = command::test(&arguments(None), &tiny_with_vocabularies()).expect("a run");
    assert!(run.earl.is_none());
}
