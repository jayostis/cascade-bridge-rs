use crate::fixtures::{
    annotations, converted, fixture, one, says, step, tiny, violations, Variant, OA, SH,
};
use oxrdf::{Quad, Term};

const PART_1: &str = "https://www.w3.org/TR/xmlschema-1/#";
const PART_2: &str = "https://www.w3.org/TR/xmlschema-2/#";
const UNNAMED: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#schemaRuleUnnamed";

#[test]
fn names_w3c_s_rule_and_the_child_the_parent_s_content_model_refuses() {
    assert_eq!(
        violations(&converted(&tiny(), "unexpected-child.xml").findings),
        [
            (
                format!("{PART_1}cvc-complex-type"),
                "/catalog".to_owned(),
                "item[1]/bogus[1]".to_owned()
            ),
            (
                format!("{PART_1}cvc-complex-type"),
                "/catalog/item[1]".to_owned(),
                "bogus[1]".to_owned()
            ),
            (
                format!("{PART_1}cvc-elt"),
                "/catalog".to_owned(),
                "item[1]/bogus[1]".to_owned()
            ),
            (
                format!("{PART_1}cvc-elt"),
                "/catalog/item[1]".to_owned(),
                "bogus[1]".to_owned()
            ),
        ]
    );
}

#[test]
fn indexes_an_offending_element_among_its_own_siblings_of_that_name() {
    assert_eq!(
        violations(&converted(&tiny(), "second-of-its-name.xml").findings),
        [
            (
                format!("{PART_1}cvc-complex-type"),
                "/catalog".to_owned(),
                "item[1]/title[2]".to_owned()
            ),
            (
                format!("{PART_1}cvc-complex-type"),
                "/catalog/item[1]".to_owned(),
                "title[2]".to_owned()
            ),
            (
                format!("{PART_1}cvc-elt"),
                "/catalog".to_owned(),
                "item[1]/title[2]".to_owned()
            ),
            (
                format!("{PART_1}cvc-elt"),
                "/catalog/item[1]".to_owned(),
                "title[2]".to_owned()
            ),
        ]
    );
}

#[test]
fn names_the_specification_s_concept_for_a_failure_w3c_names_no_rule_for() {
    assert_eq!(
        violations(&converted(&tiny(), "three-schema-location-tokens.xml").findings),
        [(UNNAMED.to_owned(), "/catalog".to_owned(), String::new())]
    );
}

const STRING_TITLE: &str = r#"<xs:element name="title" type="xs:string" minOccurs="0"/>"#;
const INT_TITLE: &str = r#"<xs:element name="title" type="xs:int" minOccurs="0"/>"#;

#[test]
fn names_a_rule_of_part_two_for_a_value_its_simple_type_refuses() {
    // The type of an item's title narrowed, so a value its simple type refuses
    // can be written where the schemas on disk accept every string.
    let typed = Variant::of(tiny()).replacing("schema/item.xsd", STRING_TITLE, INT_TITLE);
    let findings = converted(&typed, "refused-value.xml").findings;
    assert_eq!(
        violations(&findings),
        [
            (
                format!("{PART_2}cvc-datatype-valid"),
                "/catalog".to_owned(),
                "item[1]/title[1]".to_owned()
            ),
            (
                format!("{PART_2}cvc-datatype-valid"),
                "/catalog/item[1]".to_owned(),
                "title[1]".to_owned()
            ),
        ]
    );
}

#[test]
fn selects_the_offending_element_under_the_document_element() {
    assert_eq!(
        violations(&converted(&tiny(), "element-under-the-document-element.xml").findings),
        [
            (
                format!("{PART_1}cvc-complex-type"),
                "/catalog".to_owned(),
                "bogus[1]".to_owned()
            ),
            (
                format!("{PART_1}cvc-elt"),
                "/catalog".to_owned(),
                "bogus[1]".to_owned()
            ),
        ]
    );
}

/// The tiny adapter with schemas in a namespace, for an input written in one.
fn namespaced() -> Variant {
    let schema = |file: &str| {
        String::from_utf8(fixture(&format!("tests/tiny-adapter/{file}")))
            .expect("a committed schema")
    };
    Variant::of(tiny())
        .with("schema/item.xsd", schema("schema/namespaced-item.xsd"))
        .with(
            "schema/catalog.xsd",
            schema("schema/namespaced-catalog.xsd"),
        )
}

#[test]
fn writes_a_namespaced_offending_element_as_a_step_no_prefix_is_needed_for() {
    let findings = converted(&namespaced(), "namespaced.xml").findings;
    let document = format!("/{}", step("catalog"));
    let record = format!("{document}/{}[1]", step("item"));
    let bogus = format!("{}[1]", step("bogus"));
    assert_eq!(
        violations(&findings),
        [
            (
                format!("{PART_1}cvc-complex-type"),
                document.clone(),
                format!("{}[1]/{bogus}", step("item"))
            ),
            (
                format!("{PART_1}cvc-complex-type"),
                record.clone(),
                bogus.clone()
            ),
            (
                format!("{PART_1}cvc-elt"),
                document,
                format!("{}[1]/{bogus}", step("item"))
            ),
            (format!("{PART_1}cvc-elt"), record, bogus),
        ]
    );
}

/// Each annotation's one body; one with none, with two, or with no IRI fails the test.
fn bodies(findings: &[Quad], input: &str) -> Vec<String> {
    annotations(findings)
        .iter()
        .map(
            |annotation| match one(findings, annotation, &format!("{OA}hasBody")) {
                Some(Term::NamedNode(body)) => body.as_str().to_owned(),
                other => panic!("{input}: {annotation} carries no body that is an IRI: {other:?}"),
            },
        )
        .collect()
}

#[test]
fn gives_every_finding_it_writes_a_body_that_is_an_iri() {
    for input in [
        "two.xml",
        "order.xml",
        "invalid.xml",
        "unexpected-child.xml",
    ] {
        let findings = converted(&tiny(), input).findings;
        assert!(
            !bodies(&findings, input).is_empty(),
            "{input} draws no finding"
        );
    }
}

const GAP: &str = "urn:example:gaps#a-note-has-no-term";
const NOTE: &str = "a note the mapping has no term for";

const QUERY: &str = r#"PREFIX rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX sh:     <http://www.w3.org/ns/shacl#>
PREFIX oa:     <http://www.w3.org/ns/oa#>
PREFIX fx:     <http://sparql.xyz/facade-x/ns/>
PREFIX xyz:    <http://sparql.xyz/facade-x/data/>
PREFIX bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#>

CONSTRUCT {
  [] a oa:Annotation ;
    oa:hasTarget [
      oa:hasSource bridge:thisRecord ;
      oa:hasSelector [ a oa:XPathSelector ; rdf:value "note" ]
    ] ;
    oa:hasBody <urn:example:gaps#a-note-has-no-term> ;
    oa:motivatedBy oa:classifying ;
    sh:value ?text ;
    sh:resultSeverity sh:Info .
}
WHERE {
  ?item a fx:root, xyz:item ; ?slot ?note .
  ?note a xyz:note ; rdf:_1 ?text .
}
"#;

#[test]
fn passes_a_findings_query_s_body_motivation_and_value_through_unchanged() {
    // One findings query replaced by a query of this test's own, so what an
    // adapter writes is read back whatever the adapter on disk has grown into.
    let substituted = Variant::of(tiny()).with("mapping/item-note-findings.rq", QUERY);
    let findings = converted(&substituted, "two.xml").findings;
    let bodies = bodies(&findings, "two.xml");
    assert_eq!(
        bodies.iter().filter(|body| *body == GAP).count(),
        1,
        "the query's body: {bodies:?}"
    );
    let annotation = annotations(&findings)
        .into_iter()
        .find(|annotation| says(&findings, annotation, &format!("{OA}hasBody")) == GAP)
        .expect("the finding the query's body names");
    assert_eq!(
        says(&findings, &annotation, &format!("{OA}motivatedBy")),
        format!("{OA}classifying")
    );
    assert_eq!(says(&findings, &annotation, &format!("{SH}value")), NOTE);
}
