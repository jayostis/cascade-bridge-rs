mod common;

use cascade_bridge::{
    earl_report_at, load_adapter, run_manifest, EntryResult, ReportSubject, Resolver, RunOptions,
};
use common::{tiny, Variant, BRIDGE, RDF_TYPE};
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
fn names_each_entry_s_type_and_time_and_the_profiles_this_bridge_offers() {
    let results = run();
    let typed = |name: &str| {
        results
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.type_iri.as_str())
    };
    assert_eq!(
        typed("input-only"),
        Some(format!("{BRIDGE}InputOnlyTest").as_str())
    );
    assert_eq!(
        typed("pass"),
        Some(format!("{BRIDGE}IsomorphicConversionTest").as_str())
    );
    assert!(results.iter().any(|r| r.elapsed > Duration::ZERO));
    assert_eq!(
        cascade_bridge::OFFERED_PROFILES,
        [format!("{BRIDGE}sparql-1.1").as_str()]
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
    // A finding carries no sentence, so the comparison names it by its body and address.
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

/// A property the specification gives at most one value, written twice in the
/// tiny adapter: `written` is the one value the adapter carries, found in
/// `file` `times` times, and `doubled` adds a second.
struct Second {
    file: &'static str,
    written: &'static str,
    doubled: &'static str,
    times: usize,
    /// The property and both values, as the refusal writes them.
    named: [&'static str; 3],
}

const SECONDS: [Second; 16] = [
    Second {
        file: common::CRATE,
        written: r#""about": { "@id": "./" }"#,
        doubled: r##""about": [{ "@id": "./" }, { "@id": "#other" }]"##,
        times: 1,
        named: ["schema.org/about", "tiny-adapter/>", "#other>"],
    },
    Second {
        file: common::CRATE,
        written: r#""bridge:testManifest": { "@id": "fixtures/manifest.ttl" }"#,
        doubled: r#""bridge:testManifest": [{ "@id": "fixtures/manifest.ttl" }, { "@id": "fixtures/other.ttl" }]"#,
        times: 1,
        named: ["#testManifest", "/manifest.ttl>", "/other.ttl>"],
    },
    Second {
        file: common::CRATE,
        written: r#""identifier": "catalog","#,
        doubled: r#""identifier": ["catalog", "other"],"#,
        times: 1,
        named: ["schema.org/identifier", r#""catalog""#, r#""other""#],
    },
    Second {
        file: common::CRATE,
        written: r#""bridge:elementNameOfEachRecord": "item""#,
        doubled: r#""bridge:elementNameOfEachRecord": ["item", "record"]"#,
        times: 1,
        named: ["#elementNameOfEachRecord", r#""item""#, r#""record""#],
    },
    Second {
        file: common::CRATE,
        written: r#""bridge:sourceSchema": { "@id": "schema/item.xsd" }"#,
        doubled: r#""bridge:sourceSchema": [{ "@id": "schema/item.xsd" }, { "@id": "schema/other.xsd" }]"#,
        times: 1,
        named: ["#sourceSchema", "/item.xsd>", "/other.xsd>"],
    },
    Second {
        file: common::CRATE,
        written: r#""bridge:gapScheme": { "@id": "vocab/catalog-gaps.ttl" }"#,
        doubled: r#""bridge:gapScheme": [{ "@id": "vocab/catalog-gaps.ttl" }, { "@id": "vocab/other-gaps.ttl" }]"#,
        times: 1,
        named: ["#gapScheme", "/catalog-gaps.ttl>", "/other-gaps.ttl>"],
    },
    Second {
        file: common::CRATE,
        written: r#""bridge:sourceAccounting": { "@id": "vocab/catalog-accounting.ttl" }"#,
        doubled: r#""bridge:sourceAccounting": [{ "@id": "vocab/catalog-accounting.ttl" }, { "@id": "vocab/other-accounting.ttl" }]"#,
        times: 1,
        named: [
            "#sourceAccounting",
            "/catalog-accounting.ttl>",
            "/other-accounting.ttl>",
        ],
    },
    Second {
        file: common::CRATE,
        written: r#""bridge:detectQuery": { "@id": "mapping/detect.rq" }"#,
        doubled: r#""bridge:detectQuery": [{ "@id": "mapping/detect.rq" }, { "@id": "mapping/other.rq" }]"#,
        times: 1,
        named: ["#detectQuery", "/detect.rq>", "/other.rq>"],
    },
    Second {
        file: common::CRATE,
        written: r#""name": "catalog","#,
        doubled: r#""name": ["catalog", "other"],"#,
        times: 1,
        named: ["schema.org/name", r#""catalog""#, r#""other""#],
    },
    Second {
        file: common::CRATE,
        written: r#""bridge:docRootElementName": "catalog""#,
        doubled: r#""bridge:docRootElementName": ["catalog", "zcatalog"]"#,
        times: 1,
        named: ["#docRootElementName", r#""catalog""#, r#""zcatalog""#],
    },
    Second {
        file: common::CRATE,
        written: r#""bridge:documentSchema": { "@id": "schema/catalog.xsd" }"#,
        doubled: r#""bridge:documentSchema": [{ "@id": "schema/catalog.xsd" }, { "@id": "schema/other.xsd" }]"#,
        times: 1,
        named: ["#documentSchema", "/catalog.xsd>", "/other.xsd>"],
    },
    Second {
        file: MANIFEST,
        written: r#"mf:name "pass" ;"#,
        doubled: r#"mf:name "pass", "other" ;"#,
        times: 1,
        named: ["test-manifest#name", r#""pass""#, r#""other""#],
    },
    Second {
        file: MANIFEST,
        written: "bridge:input <in/two.xml> ;",
        doubled: "bridge:input <in/two.xml>, <in/order.xml> ;",
        times: 3,
        named: ["#input", "/two.xml>", "/order.xml>"],
    },
    Second {
        file: MANIFEST,
        written: "bridge:envelope <../ro-crate-metadata.json#envelope-catalog> ]",
        doubled: "bridge:envelope <../ro-crate-metadata.json#envelope-catalog>, <../ro-crate-metadata.json#envelope-other> ]",
        times: 8,
        named: ["#envelope ", "#envelope-catalog>", "#envelope-other>"],
    },
    Second {
        file: MANIFEST,
        written: "bridge:expectedGraph <expected/two.ttl> ;",
        doubled: "bridge:expectedGraph <expected/two.ttl>, <expected/order.ttl> ;",
        times: 1,
        named: ["#expectedGraph", "expected/two.ttl>", "expected/order.ttl>"],
    },
    Second {
        file: MANIFEST,
        written: "bridge:expectedFindings <findings/two.ttl> ]",
        doubled: "bridge:expectedFindings <findings/two.ttl>, <findings/order.ttl> ]",
        times: 2,
        named: ["#expectedFindings", "findings/two.ttl>", "findings/order.ttl>"],
    },
];

/// What the Bridge says of the second value: a crate's is a refusal to load
/// it, and a manifest's is the verdict on the entry `pass` that carries it.
fn said_of(second: &Second) -> std::result::Result<String, String> {
    let resolver = Variant::of(tiny()).replacing_exactly(
        second.file,
        second.written,
        second.doubled,
        second.times,
    );
    let adapter = match load_adapter(&resolver) {
        Err(refusal) if second.file == common::CRATE => return Ok(refusal.to_string()),
        Err(refusal) => return Err(format!("the crate was refused: {refusal}")),
        Ok(_) if second.file == common::CRATE => return Err("the crate loaded".to_owned()),
        Ok(adapter) => adapter,
    };
    let results = run_manifest(&adapter, &resolver, RunOptions::default())
        .map_err(|refusal| format!("the run was refused: {refusal}"))?;
    let pass = results
        .iter()
        .find(|r| r.entry.to_string().ends_with("manifest.ttl#pass>"))
        .ok_or("no entry pass was reported")?;
    match pass.outcome.as_str() {
        "failed" => Ok(pass.description.clone()),
        other => Err(format!("pass was {other}: {}", pass.description)),
    }
}

#[test]
fn refuses_a_second_value_of_each_single_valued_property_with_a_sentence_naming_both() {
    let mut wrong = Vec::new();
    for second in &SECONDS {
        match said_of(second) {
            Ok(said) => {
                let unnamed: Vec<_> = second
                    .named
                    .iter()
                    .filter(|named| !said.contains(*named))
                    .collect();
                if !unnamed.is_empty() {
                    wrong.push(format!("{}: {unnamed:?} not in: {said}", second.doubled));
                }
            }
            Err(said) => wrong.push(format!("{}: {said}", second.doubled)),
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

const MANIFEST: &str = "fixtures/manifest.ttl";

/// `list` replaces what `mf:entries` names; `appended` holds triples for a list
/// Turtle's collection syntax cannot write.
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
