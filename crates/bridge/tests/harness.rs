// The engine's own test subject: a synthetic adapter built so that each
// outcome is reached by the smallest input that can reach it, and two entries
// must fail.
use cascade_bridge::{
    earl_report_at, load_adapter, run_manifest, DirectoryResolver, ReportSubject, RunOptions,
};
use oxrdf::{Graph, NamedNode, Triple};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeMap;
use std::path::PathBuf;

const EARL: &str = "http://www.w3.org/ns/earl#";

fn tiny() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter")
}

fn run() -> Vec<cascade_bridge::EntryResult> {
    let resolver = DirectoryResolver::new(tiny()).expect("resolver");
    let adapter = load_adapter(&resolver).expect("adapter");
    run_manifest(&adapter, &resolver, RunOptions::default()).expect("manifest")
}

#[test]
fn reaches_every_outcome_and_fails_exactly_the_entries_built_to_fail() {
    let results = run();
    let outcomes: BTreeMap<&str, &str> = results
        .iter()
        .map(|r| (r.name.as_str(), r.outcome.as_str()))
        .collect();
    assert_eq!(
        outcomes,
        BTreeMap::from([
            ("pass", "passed"),
            ("graph-fail", "failed"),
            ("findings-fail", "failed"),
            ("multiset-order", "passed"),
            ("input-only", "cantTell"),
            ("dataset", "untested"),
        ])
    );
}

#[test]
fn says_what_differed_in_the_words_of_the_comparison() {
    let results = run();
    let described = |name: &str| {
        results
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.description.clone())
            .unwrap_or_default()
    };
    assert!(described("graph-fail").contains("graph differs: 1 missing, 1 extra"));
    assert!(described("findings-fail").contains("findings differ: 0 missing, 1 extra"));
    assert!(!described("pass").contains("detect query is false"));
    assert!(described("pass").contains("findings equal as a multiset (1)"));
}

#[test]
fn runs_nothing_when_the_adapter_requires_a_profile_this_bridge_does_not_offer() {
    let resolver = DirectoryResolver::new(tiny()).expect("resolver");
    let mut adapter = load_adapter(&resolver).expect("adapter");
    adapter
        .profiles_required
        .push("https://ns.cascadeprotocol.org/bridge/v1-draft#xslt-3".to_owned());
    let results = run_manifest(&adapter, &resolver, RunOptions::default()).expect("manifest");
    assert!(results.iter().all(|r| r.outcome.as_str() == "inapplicable"));
}

#[test]
fn reports_one_earl_assertion_per_entry_its_outcome_on_the_test_result() {
    let results = run();
    let subject = ReportSubject {
        iri: "urn:example:bridge".to_owned(),
        name: "test".to_owned(),
        version: "0".to_owned(),
    };
    let turtle = earl_report_at(&results, &subject, "2026-01-01T00:00:00Z").expect("earl");

    let mut graph = Graph::new();
    for quad in RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri("urn:example:report")
        .expect("base")
        .for_slice(turtle.as_bytes())
    {
        let quad = quad.expect("turtle");
        graph.insert(&Triple::new(quad.subject, quad.predicate, quad.object));
    }

    let type_of = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").expect("iri");
    let assertion = NamedNode::new(format!("{EARL}Assertion")).expect("iri");
    let result_of = NamedNode::new(format!("{EARL}result")).expect("iri");
    let outcome_of = NamedNode::new(format!("{EARL}outcome")).expect("iri");

    let assertions: Vec<_> = graph
        .subjects_for_predicate_object(type_of.as_ref(), assertion.as_ref())
        .collect();
    assert_eq!(assertions.len(), results.len());
    for a in assertions {
        assert_eq!(
            graph
                .objects_for_subject_predicate(a, outcome_of.as_ref())
                .count(),
            0,
            "the outcome belongs to the TestResult, not to the Assertion"
        );
        let result = graph
            .objects_for_subject_predicate(a, result_of.as_ref())
            .next()
            .expect("a result");
        let result = match result {
            oxrdf::TermRef::BlankNode(b) => oxrdf::NamedOrBlankNodeRef::BlankNode(b),
            oxrdf::TermRef::NamedNode(n) => oxrdf::NamedOrBlankNodeRef::NamedNode(n),
            other => panic!("unexpected result node {other}"),
        };
        assert_eq!(
            graph
                .objects_for_subject_predicate(result, outcome_of.as_ref())
                .count(),
            1
        );
    }
}
