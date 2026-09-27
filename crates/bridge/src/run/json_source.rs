use crate::fixtures;
use crate::fixtures::{converted, objects, tiny_json, Variant, OA, RDF_TYPE, RDF_VALUE};
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
            facts: None,
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
            facts: None,
        },
    )
    .err()
    .expect("refused")
    .to_string();
    assert!(refused.contains("not JSON"), "{refused}");
}

#[test]
fn follows_no_address_this_bridge_built_from_its_own_walk() {
    let resolver = Variant::of(tiny_json()).replacing(
        "fixtures/in/item.json",
        "\"title\": \"nine\"",
        "\"title\": 5, \"title\": 6",
    );
    let findings = converted(&resolver, "item.json").findings;
    let type_ = "<https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-01#section-6.25>";
    let not_one = "<https://ns.cascadeprotocol.org/bridge/v1-draft#addressNotOneNode>";
    assert!(
        findings.iter().any(|q| q.object.to_string() == type_),
        "{findings:?}"
    );
    assert!(
        !findings.iter().any(|q| q.object.to_string() == not_one),
        "{findings:?}"
    );
}

/// The tiny JSON adapter with its mapping reading the selector fact and a
/// document table every record of the document constructs.
fn tabled() -> Variant {
    Variant::of(tiny_json())
        .with(
            "mapping/item.rq",
            "PREFIX fx: <http://sparql.xyz/facade-x/ns/>
PREFIX xyz: <http://sparql.xyz/facade-x/data/>
PREFIX bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#>
PREFIX ex: <urn:example:list#>
CONSTRUCT { ?s ex:at ?at ; ex:seen ?other . }
WHERE {
  ?item a fx:root ; xyz:id ?id .
  BIND(IRI(CONCAT(\"urn:example:item:\", ?id)) AS ?s)
  bridge:thisRecord bridge:selector ?at .
  ?holder ex:holds ?other .
}",
        )
        .with(
            "mapping/table.rq",
            "PREFIX fx: <http://sparql.xyz/facade-x/ns/>
PREFIX xyz: <http://sparql.xyz/facade-x/data/>
PREFIX ex: <urn:example:list#>
CONSTRUCT { [] ex:holds ?id } WHERE { ?item a fx:root ; xyz:id ?id }",
        )
        .replacing(
            "ro-crate-metadata.json",
            "\"bridge:detectQuery\": \"bridge:detectQuery\",",
            "\"bridge:detectQuery\": \"bridge:detectQuery\",\n      \"bridge:documentTableQuery\": \"bridge:documentTableQuery\",",
        )
        .replacing(
            "ro-crate-metadata.json",
            "\"bridge:detectQuery\": { \"@id\": \"mapping/detect.rq\" },",
            "\"bridge:detectQuery\": { \"@id\": \"mapping/detect.rq\" },\n      \"bridge:documentTableQuery\": { \"@id\": \"mapping/table.rq\" },",
        )
}

fn said(quads: &[Quad], subject: &str, predicate: &str) -> Vec<String> {
    let mut said: Vec<String> = objects(quads, subject, predicate)
        .iter()
        .map(ToString::to_string)
        .collect();
    said.sort();
    said
}

#[test]
fn gives_each_json_record_its_pointer_and_the_table_every_record_of_its_document_constructs() {
    let quads = converted(&tabled(), "list.json").quads;
    for (item, at) in [("1", "\"/items/0\""), ("2", "\"/items/1\"")] {
        let item = format!("<urn:example:item:{item}>");
        assert_eq!(said(&quads, &item, "urn:example:list#at"), [at]);
        assert_eq!(
            said(&quads, &item, "urn:example:list#seen"),
            ["\"1\"", "\"2\""],
            "{item} reads what every record of its document holds"
        );
    }
}

#[test]
fn gives_a_json_document_s_own_value_the_empty_pointer() {
    let quads = read_in(&tabled(), "item.json", "envelope-item").quads;
    assert_eq!(
        said(&quads, "<urn:example:item:9>", "urn:example:list#at"),
        ["\"\""]
    );
}

/// The tiny JSON adapter whose list envelope names one definition of a schema whose
/// whole the adapter names, the whole requiring a member the definition does not,
/// and the definition one the whole does not.
fn entry_schema() -> Variant {
    Variant::of(tiny_json())
        .with(
            "schema/entries.schema.json",
            r##"{
  "$schema": "http://json-schema.org/draft-06/schema#",
  "type": "object",
  "required": ["kind"],
  "properties": {"kind": {"type": "string"}},
  "definitions": {
    "entry": {
      "type": "object",
      "required": ["id"],
      "properties": {"title": {"$ref": "#/definitions/title"}}
    },
    "title": {"type": "string"}
  }
}"##,
        )
        .replacing(
            "ro-crate-metadata.json",
            "\"bridge:sourceSchema\": { \"@id\": \"schema/item.schema.json\" },",
            "\"bridge:sourceSchema\": { \"@id\": \"schema/entries.schema.json\" },",
        )
        .replacing(
            "ro-crate-metadata.json",
            "\"bridge:documentSchema\": { \"@id\": \"schema/list.schema.json\" }",
            "\"bridge:documentSchema\": { \"@id\": \"schema/list.schema.json\" },\n      \"bridge:sourceSchema\": { \"@id\": \"schema/entries.schema.json#/definitions/entry\" }",
        )
        .replacing(
            "fixtures/in/list.json",
            "\"extra\": {\"a\": \"x\"}}]",
            "\"extra\": {\"a\": \"x\"}}, {\"kind\": \"entry\", \"title\": \"three\"}]",
        )
}

#[test]
fn validates_a_record_against_the_subschema_its_envelope_s_schema_fragment_names_resolving_refs_in_the_whole_document(
) {
    let violations = fixtures::violations(&converted(&entry_schema(), "list.json").findings);
    let section = |number: &str| {
        format!("https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-01#section-6.{number}")
    };
    assert_eq!(
        violations,
        [
            (section("17"), "/items/2".to_owned(), String::new()),
            (section("25"), "/items/1".to_owned(), "/title".to_owned()),
        ]
    );
}

#[test]
fn validates_a_record_read_in_an_envelope_naming_no_schema_against_the_adapter_s() {
    let violations = fixtures::violations(
        &converted(
            &entry_schema().replacing("fixtures/in/item.json", "\"kind\": \"item\"", "\"kind\": 3"),
            "item.json",
        )
        .findings,
    );
    let type_ =
        "https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-01#section-6.25";
    assert_eq!(
        violations,
        [(type_.to_owned(), String::new(), "/kind".to_owned())]
    );
}

#[test]
fn refuses_a_source_schema_whose_fragment_names_no_subschema() {
    let resolver = entry_schema().replacing(
        "ro-crate-metadata.json",
        "#/definitions/entry",
        "#/definitions/absent",
    );
    let adapter = load_adapter(&resolver).expect("the adapter");
    let refused = prepare(&adapter, &resolver)
        .err()
        .expect("refused")
        .to_string();
    assert!(refused.contains("#/definitions/absent"), "{refused}");
}
