// A finding's body is a code and its address is the node it is about: a
// Bridge's own finding carries W3C's rule for the schema rule that was broken,
// and selects the element it was broken on rather than the record it stood in.
// No finding, from an adapter or from a Bridge, carries a sentence.
use cascade_bridge::{
    convert, load_adapter, prepare, Conversion, DirectoryResolver, Resolver, Source,
};
use oxrdf::{Quad, Term};
use std::path::PathBuf;

const OA: &str = "http://www.w3.org/ns/oa#";
const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";

/// The anchor of a rule the XML Schema Recommendation defines, by Part.
const PART_1: &str = "https://www.w3.org/TR/xmlschema-1/#";
const PART_2: &str = "https://www.w3.org/TR/xmlschema-2/#";
/// What a schema failure W3C names no rule for carries instead.
const UNNAMED: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#schemaRuleUnnamed";

/// The namespace the namespaced fixture and its schemas are written in.
const CATALOG: &str = "urn:example:catalog";

fn tiny() -> DirectoryResolver {
    DirectoryResolver::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"))
        .expect("resolver")
}

fn run(resolver: &dyn Resolver, input: &str) -> Conversion {
    let adapter = load_adapter(resolver).expect("adapter");
    let prepared = prepare(&adapter, resolver).expect("prepared");
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    let xml = resolver.read(&iri).expect("input");
    convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            xml: &xml,
        },
    )
    .expect("conversion")
}

/// A term as an address or a body is read: an IRI or a literal by what it
/// says, anything else by how N-Triples writes it.
fn written(term: Term) -> String {
    match term {
        Term::NamedNode(node) => node.as_str().to_owned(),
        Term::Literal(literal) => literal.value().to_owned(),
        other => other.to_string(),
    }
}

/// The one object this subject carries for this predicate, where it carries
/// exactly one.
fn one(quads: &[Quad], subject: &str, predicate: &str) -> Option<Term> {
    let mut objects = quads
        .iter()
        .filter(|q| q.subject.to_string() == subject && q.predicate.as_str() == predicate)
        .map(|q| q.object.clone());
    let first = objects.next()?;
    match objects.next() {
        None => Some(first),
        Some(_) => None,
    }
}

fn node(quads: &[Quad], subject: &str, predicate: &str) -> String {
    one(quads, subject, predicate)
        .map(|term| term.to_string())
        .unwrap_or_default()
}

fn says(quads: &[Quad], subject: &str, predicate: &str) -> String {
    one(quads, subject, predicate)
        .map(written)
        .unwrap_or_default()
}

/// Every finding a Bridge stage made itself, as its body and the address it
/// carries: the selector of the record or document it is about, and the step
/// or steps below that which its `oa:refinedBy` names. A finding an adapter's
/// query made is another severity and is left out.
fn violations(findings: &[Quad]) -> Vec<(String, String, String)> {
    let annotation = format!("{OA}Annotation");
    let violation = format!("{SH}Violation");
    let mut rows: Vec<(String, String, String)> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{SH}resultSeverity"))
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == violation))
        .map(|q| q.subject.to_string())
        .filter(|subject| {
            findings.iter().any(|q| {
                q.subject.to_string() == *subject
                    && q.predicate.as_str() == RDF_TYPE
                    && matches!(&q.object, Term::NamedNode(n) if n.as_str() == annotation)
            })
        })
        .map(|subject| {
            let body = says(findings, &subject, &format!("{OA}hasBody"));
            let target = node(findings, &subject, &format!("{OA}hasTarget"));
            let selector = node(findings, &target, &format!("{OA}hasSelector"));
            let refinement = node(findings, &selector, &format!("{OA}refinedBy"));
            (
                body,
                says(findings, &selector, RDF_VALUE),
                says(findings, &refinement, RDF_VALUE),
            )
        })
        .collect();
    rows.sort();
    rows
}

#[test]
fn names_w3c_s_rule_and_the_child_the_parent_s_content_model_refuses() {
    assert_eq!(
        violations(&run(&tiny(), "unexpected-child.xml").findings),
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
        violations(&run(&tiny(), "second-of-its-name.xml").findings),
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
        violations(&run(&tiny(), "three-schema-location-tokens.xml").findings),
        [(UNNAMED.to_owned(), "/catalog".to_owned(), String::new())]
    );
}

/// The tiny adapter with the type of an item's title narrowed, so a value its
/// simple type refuses can be written where the schemas on disk accept every
/// string.
struct Typed {
    directory: DirectoryResolver,
}

const STRING_TITLE: &str = r#"<xs:element name="title" type="xs:string" minOccurs="0"/>"#;
const INT_TITLE: &str = r#"<xs:element name="title" type="xs:int" minOccurs="0"/>"#;

impl Resolver for Typed {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("schema/item.xsd") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(text.contains(STRING_TITLE), "the schema types a title");
        Ok(text.replace(STRING_TITLE, INT_TITLE).into_bytes())
    }
}

#[test]
fn names_a_rule_of_part_two_for_a_value_its_simple_type_refuses() {
    let findings = run(&Typed { directory: tiny() }, "refused-value.xml").findings;
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
        violations(&run(&tiny(), "element-under-the-document-element.xml").findings),
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
/// The validator's own `element_path` is element names alone, so an address
/// taken from it cannot write a namespaced step at all.
struct Namespaced {
    directory: DirectoryResolver,
}

impl Resolver for Namespaced {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        for (plain, namespaced) in [
            ("schema/item.xsd", "schema/namespaced-item.xsd"),
            ("schema/catalog.xsd", "schema/namespaced-catalog.xsd"),
        ] {
            if iri.ends_with(plain) {
                return self.directory.read(&iri.replace(plain, namespaced));
            }
        }
        self.directory.read(iri)
    }
}

fn step(local: &str) -> String {
    format!("*[local-name()='{local}' and namespace-uri()='{CATALOG}']")
}

#[test]
fn writes_a_namespaced_offending_element_as_a_step_no_prefix_is_needed_for() {
    let findings = run(&Namespaced { directory: tiny() }, "namespaced.xml").findings;
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

#[test]
fn gives_every_finding_it_writes_a_body_that_is_an_iri() {
    for input in [
        "two.xml",
        "order.xml",
        "invalid.xml",
        "unexpected-child.xml",
    ] {
        let findings = run(&tiny(), input).findings;
        let bodies: Vec<String> = findings
            .iter()
            .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
            .map(|q| q.object.to_string())
            .collect();
        assert!(!bodies.is_empty(), "{input} draws no finding");
        for body in &bodies {
            assert!(
                body.starts_with('<'),
                "{input}: a finding carries a body that is not an IRI: {body}"
            );
        }
    }
}

#[test]
fn names_the_sentence_body_nowhere_in_the_crates() {
    // Written in parts, so this test is not itself what it looks for.
    let sentence = concat!("Textual", "Body");
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crates directory")
        .to_owned();
    let mut naming = Vec::new();
    let mut stack = vec![crates.clone()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).expect("read a directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "target") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            let bytes = std::fs::read(&path).expect("read a file");
            if String::from_utf8_lossy(&bytes).contains(sentence) {
                naming.push(path.display().to_string());
            }
        }
    }
    naming.sort();
    assert!(
        naming.is_empty(),
        "{sentence} is named in {}: {naming:?}",
        crates.display()
    );
}

/// The tiny adapter with one findings query replaced by a query of this
/// test's own, so what an adapter writes is read back from the output whatever
/// the adapter on disk has grown into.
struct Substituted {
    directory: DirectoryResolver,
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

impl Resolver for Substituted {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        if iri.ends_with("mapping/item-note-findings.rq") {
            return Ok(QUERY.as_bytes().to_vec());
        }
        self.directory.read(iri)
    }
}

#[test]
fn passes_a_findings_query_s_body_motivation_and_value_through_unchanged() {
    let findings = run(&Substituted { directory: tiny() }, "two.xml").findings;
    let bodies: Vec<String> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
        .map(|q| q.object.to_string())
        .collect();
    assert_eq!(
        bodies
            .iter()
            .filter(|body| *body == &format!("<{GAP}>"))
            .count(),
        1,
        "the query's body: {bodies:?}"
    );
    let annotation = findings
        .iter()
        .find(|q| {
            q.predicate.as_str() == format!("{OA}hasBody")
                && matches!(&q.object, Term::NamedNode(n) if n.as_str() == GAP)
        })
        .map(|q| q.subject.to_string())
        .expect("the finding the query's body names");
    assert_eq!(
        says(&findings, &annotation, &format!("{OA}motivatedBy")),
        format!("{OA}classifying")
    );
    assert_eq!(says(&findings, &annotation, &format!("{SH}value")), NOTE);

    for body in &bodies {
        assert!(
            body.starts_with('<'),
            "a finding carries a body that is not an IRI: {bodies:?}"
        );
    }
}
