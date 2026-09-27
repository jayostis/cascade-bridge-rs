use crate::fixtures::{converted, tiny_json, Variant, OA, RDF_TYPE, RDF_VALUE};
use crate::harness::{run_manifest, RunOptions};
use crate::load::load_adapter;
use crate::run::{convert, prepare, Conversion, Source};
use crate::Resolver;
use oxrdf::Quad;

fn read_in(resolver: &dyn Resolver, input: &str, envelope: &str) -> Conversion {
    let adapter = load_adapter(resolver).expect("the adapter");
    let prepared = prepare(&adapter, resolver).expect("prepared");
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    let named = format!("{}ro-crate-metadata.json#{envelope}", resolver.root());
    let bytes = resolver.read(&iri).expect("the input");
    convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: Some(&named),
            bytes: &bytes,
        },
    )
    .expect("the conversion")
}

fn selectors(findings: &[Quad]) -> Vec<String> {
    let fragment = format!("{OA}FragmentSelector");
    let typed: Vec<String> = findings
        .iter()
        .filter(|q| {
            q.predicate.as_str() == RDF_TYPE && q.object.to_string() == format!("<{fragment}>")
        })
        .map(|q| q.subject.to_string())
        .collect();
    let mut values: Vec<String> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_VALUE && typed.contains(&q.subject.to_string()))
        .map(|q| q.object.to_string())
        .collect();
    values.sort();
    values.dedup();
    values
}

#[test]
fn passes_every_entry_of_the_tiny_json_adapter() {
    let resolver = tiny_json();
    let adapter = load_adapter(&resolver).expect("the adapter");
    let results = run_manifest(&adapter, &resolver, RunOptions::default()).expect("the run");
    assert_eq!(results.len(), 2);
    for result in results {
        assert_eq!(
            result.outcome.as_str(),
            "passed",
            "{}: {}",
            result.name,
            result.description
        );
    }
}

#[test]
fn reads_a_json_document_in_the_envelope_naming_its_member_s_value_before_one_naming_none() {
    let conversion = converted(&tiny_json(), "list.json");
    assert_eq!(conversion.units, 2);
    assert_eq!(conversion.detected, Some(true));
    assert_eq!(
        selectors(&conversion.findings),
        ["\"/extra\"", "\"/extra/a\"", "\"/items/1\"", "\"/title\""]
    );
}

#[test]
fn reads_a_json_document_in_the_envelope_named_whether_or_not_another_admits_it() {
    let conversion = read_in(&tiny_json(), "list.json", "envelope-item");
    assert_eq!(conversion.units, 1);
    let selectors = selectors(&conversion.findings);
    assert!(selectors.contains(&"\"\"".to_owned()), "{selectors:?}");
    assert!(
        selectors.contains(&"\"/items/1/extra\"".to_owned()),
        "{selectors:?}"
    );
}

#[test]
fn validates_a_json_document_against_its_envelope_s_schema_at_the_document_s_value() {
    let resolver = Variant::of(tiny_json()).replacing(
        "fixtures/in/list.json",
        "{\"kind\": \"list\",",
        "{\"kind\": \"list\", \"trailer\": 1,",
    );
    let findings = converted(&resolver, "list.json").findings;
    let additional = "<https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-01#section-6.20>";
    assert!(
        findings.iter().any(|q| q.object.to_string() == additional),
        "{findings:?}"
    );
}

#[test]
fn refuses_a_json_document_that_is_not_json() {
    let resolver = Variant::of(tiny_json()).replacing("fixtures/in/item.json", "\"nine\"", "nine");
    let adapter = load_adapter(&resolver).expect("the adapter");
    let prepared = prepare(&adapter, &resolver).expect("prepared");
    let iri = format!("{}fixtures/in/item.json", resolver.root());
    let bytes = resolver.read(&iri).expect("the input");
    let refused = convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            bytes: &bytes,
        },
    )
    .err()
    .expect("refused")
    .to_string();
    assert!(refused.contains("not JSON"), "{refused}");
}
