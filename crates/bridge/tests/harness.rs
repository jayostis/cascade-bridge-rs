// The engine's own test subject: a synthetic adapter built so that each
// outcome is reached by the smallest input that can reach it, and two entries
// must fail.
mod common;

use cascade_bridge::{
    earl_report_at, load_adapter, run_manifest, EntryResult, ReportSubject, Resolver, RunOptions,
};
use common::{tiny, Variant, RDF_TYPE};
use oxrdf::{Graph, NamedNode, NamedOrBlankNodeRef, TermRef, Triple};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeMap;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const EARL: &str = "http://www.w3.org/ns/earl#";

fn run() -> Vec<cascade_bridge::EntryResult> {
    let resolver = tiny();
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
            ("findings-repeated", "passed"),
            ("census", "passed"),
            // This resolver was given no checkout, so the vocabulary that
            // draws the entry's one expected finding is not read.
            ("shapes", "failed"),
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
    assert!(
        described("findings-fail").contains(
            "findings differ: 6 annotation(s) produced, 5 expected; 0 finding(s) missing, 1 extra"
        ),
        "{}",
        described("findings-fail")
    );
    // A finding carries no sentence, so what the comparison reports names it
    // by the two things it does carry: its body, an IRI, and its address.
    assert!(
        described("findings-fail")
            .contains("<http://www.w3.org/ns/oa#hasBody> <urn:example:catalog#noteHasNoTerm>"),
        "the finding that differs is not named by its body: {}",
        described("findings-fail")
    );
    assert!(
        described("findings-fail").contains("\"/catalog/item[1]\""),
        "the finding that differs is not placed by its address: {}",
        described("findings-fail")
    );
    assert!(!described("pass").contains("detect query is false"));
    assert!(
        described("pass").contains("findings isomorphic (3 annotation(s))"),
        "{}",
        described("pass")
    );
}

#[test]
fn runs_nothing_when_the_adapter_requires_a_profile_this_bridge_does_not_offer() {
    let resolver = tiny();
    let mut adapter = load_adapter(&resolver).expect("adapter");
    adapter
        .required_profiles
        .push("https://ns.cascadeprotocol.org/bridge/v1-draft#xslt-3".to_owned());
    let results = run_manifest(&adapter, &resolver, RunOptions::default()).expect("manifest");
    assert_eq!(results.len(), run().len(), "every entry is reported");
    for result in &results {
        assert_eq!(
            result.outcome.as_str(),
            "inapplicable",
            "{}: {}",
            result.name,
            result.description
        );
        assert!(
            result.description.contains("xslt-3"),
            "{}: {}",
            result.name,
            result.description
        );
    }
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

    let type_of = NamedNode::new(RDF_TYPE).expect("iri");
    let assertion = NamedNode::new(format!("{EARL}Assertion")).expect("iri");
    let test_of = NamedNode::new(format!("{EARL}test")).expect("iri");
    let result_of = NamedNode::new(format!("{EARL}result")).expect("iri");
    let outcome_of = NamedNode::new(format!("{EARL}outcome")).expect("iri");

    let assertions: Vec<_> = graph
        .subjects_for_predicate_object(type_of.as_ref(), assertion.as_ref())
        .collect();
    assert_eq!(assertions.len(), results.len());
    let mut reported: BTreeMap<String, String> = BTreeMap::new();
    for a in assertions {
        assert_eq!(
            graph
                .objects_for_subject_predicate(a, outcome_of.as_ref())
                .count(),
            0,
            "the outcome belongs to the TestResult, not to the Assertion"
        );
        let result = match graph
            .object_for_subject_predicate(a, result_of.as_ref())
            .expect("a result")
        {
            TermRef::BlankNode(b) => NamedOrBlankNodeRef::BlankNode(b),
            TermRef::NamedNode(n) => NamedOrBlankNodeRef::NamedNode(n),
            other => panic!("unexpected result node {other}"),
        };
        let outcomes: Vec<_> = graph
            .objects_for_subject_predicate(result, outcome_of.as_ref())
            .collect();
        let [TermRef::NamedNode(outcome)] = outcomes[..] else {
            panic!("a result carries one outcome IRI: {outcomes:?}");
        };
        let test = graph
            .object_for_subject_predicate(a, test_of.as_ref())
            .expect("a test");
        reported.insert(test.to_string(), outcome.as_str().to_owned());
    }
    let expected: BTreeMap<String, String> = results
        .iter()
        .map(|r| (r.entry.to_string(), format!("{EARL}{}", r.outcome.as_str())))
        .collect();
    assert_eq!(reported, expected, "each entry's outcome, on its own test");
}

#[test]
fn fails_every_entry_when_the_adapter_names_no_element_name_of_each_record() {
    let resolver = tiny();
    let mut adapter = load_adapter(&resolver).expect("adapter");
    adapter.element_name_of_each_record = None;
    let results = run_manifest(&adapter, &resolver, RunOptions::default()).expect("manifest");
    assert!(!results.is_empty());
    for result in &results {
        assert_eq!(
            result.outcome.as_str(),
            "failed",
            "{}: {}",
            result.name,
            result.description
        );
        assert!(
            result
                .description
                .contains("the adapter names no bridge:elementNameOfEachRecord"),
            "{}: {}",
            result.name,
            result.description
        );
    }
}

const MANIFEST: &str = "fixtures/manifest.ttl";

/// The tiny adapter with its manifest's entry list replaced, so an entry, or
/// the list itself, can be written in a form the adapter on disk does not use.
/// `list` is what `mf:entries` names in place of the list on disk; `appended`
/// holds triples for a list Turtle's collection syntax cannot write.
fn entries(list: &str, appended: &str) -> Variant {
    let directory = tiny();
    let iri = format!("{}{MANIFEST}", directory.root());
    let text = String::from_utf8(directory.read(&iri).expect("the manifest")).expect("utf-8");
    let (head, rest) = text.split_once("mf:entries (").expect("an entry list");
    let tail = &rest[rest.find(')').expect("the list's end") + 1..];
    Variant::of(directory).with(MANIFEST, format!("{head}mf:entries {list}{tail}{appended}"))
}

fn run_with(entries_written: &'static str) -> (Vec<EntryResult>, Graph) {
    let resolver = entries(&format!("( {entries_written} )"), "");
    let adapter = load_adapter(&resolver).expect("adapter");
    let results = run_manifest(&adapter, &resolver, RunOptions::default()).expect("manifest");
    let subject = ReportSubject {
        iri: "urn:example:bridge".to_owned(),
        name: "test".to_owned(),
        version: "0".to_owned(),
    };
    let turtle = earl_report_at(&results, &subject, "2026-01-01T00:00:00Z").expect("a report");
    let mut report = Graph::new();
    for quad in RdfParser::from_format(RdfFormat::Turtle).for_slice(turtle.as_bytes()) {
        let quad = quad.expect("turtle");
        report.insert(&Triple::new(quad.subject, quad.predicate, quad.object));
    }
    (results, report)
}

/// What each assertion's earl:test names: its IRI, or the title of a test that
/// has none.
fn tests_reported(report: &Graph) -> Vec<String> {
    let test = NamedNode::new(format!("{EARL}test")).expect("iri");
    let title = NamedNode::new("http://purl.org/dc/terms/title").expect("iri");
    let mut named: Vec<String> = report
        .triples_for_predicate(test.as_ref())
        .map(|t| match t.object {
            TermRef::NamedNode(n) => n.as_str().to_owned(),
            TermRef::BlankNode(b) => match report.object_for_subject_predicate(b, title.as_ref()) {
                Some(TermRef::Literal(l)) => l.value().to_owned(),
                other => panic!("an anonymous test titled {other:?}"),
            },
            other => panic!("earl:test is {other}"),
        })
        .collect();
    named.sort();
    named
}

#[test]
fn reports_an_entry_written_as_a_blank_node_by_its_name() {
    let (results, report) = run_with(
        r#"[ a bridge:IsomorphicConversionTest ; mf:name "anonymous" ;
             mf:action [ bridge:input <in/two.xml> ] ;
             mf:result [ bridge:expectedGraph <expected/two.ttl> ; bridge:expectedFindings <findings/two.ttl> ] ]"#,
    );
    let outcomes: Vec<_> = results
        .iter()
        .map(|r| (r.name.as_str(), r.outcome.as_str()))
        .collect();
    assert_eq!(outcomes, [("anonymous", "passed")]);
    assert_eq!(tests_reported(&report), ["anonymous"]);
}

#[test]
fn reports_a_literal_entry_instead_of_leaving_it_out() {
    let (results, report) = run_with(r#"<#pass> "not a test""#);
    let outcomes: Vec<_> = results
        .iter()
        .map(|r| (r.name.as_str(), r.outcome.as_str()))
        .collect();
    assert_eq!(
        outcomes,
        [("pass", "passed"), ("not a test", "inapplicable")]
    );
    assert!(
        results[1].description.contains("literal"),
        "{}",
        results[1].description
    );
    let reported = tests_reported(&report);
    assert_eq!(reported.len(), 2, "{reported:?}");
    assert!(
        reported[0].ends_with("fixtures/manifest.ttl#pass"),
        "{reported:?}"
    );
    assert_eq!(reported[1], "not a test");
}

#[test]
fn refuses_an_entry_list_that_loops_back_on_itself() {
    let resolver = entries(
        "_:cell",
        "_:cell <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:cell .\n",
    );
    let adapter = load_adapter(&resolver).expect("adapter");
    // Walked without a guard this list never ends, so the run is watched from
    // here: a hang would otherwise be the test's only way to fail.
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let run = run_manifest(&adapter, &resolver, RunOptions::default());
        let _ = sender.send(run.map(|results| results.len()).map_err(|e| e.to_string()));
    });
    let run = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("the run to end");
    let error = run.expect_err("a cyclic entry list refused");
    assert!(error.contains("loops back on itself"), "{error}");
}
