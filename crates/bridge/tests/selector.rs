// A record's selector is the XPath from the document element to the record:
// a step for the document element, then one for each element down to and
// including the record, every step below the document element carrying its
// place among its own siblings of that name. A finding about the document
// rather than about a record stops at the document element's own step.
use cascade_bridge::{
    convert, lift_slice, load_adapter, prepare, DirectoryResolver, Resolver, Source,
};
use oxrdf::Quad;
use std::path::PathBuf;

const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";
const CATALOG: &str = "urn:example:catalog";

fn selectors(xml: &[u8], record: &str) -> Vec<String> {
    lift_slice(xml, Some(record))
        .expect("lift")
        .map(|unit| unit.expect("unit").selector())
        .collect()
}

#[test]
fn writes_a_step_for_every_element_down_to_a_record_below_the_document_element() {
    assert_eq!(
        selectors(b"<set><group><item/><item/></group></set>", "item"),
        ["/set/group[1]/item[1]", "/set/group[1]/item[2]"]
    );
}

#[test]
fn numbers_a_record_among_its_own_parent_s_children_of_that_name() {
    assert_eq!(
        selectors(
            b"<set><group><item/></group><group><item/></group></set>",
            "item"
        ),
        ["/set/group[1]/item[1]", "/set/group[2]/item[1]"]
    );
}

#[test]
fn counts_a_sibling_of_another_name_towards_neither() {
    assert_eq!(
        selectors(
            b"<set><other/><group/><other/><group><item/></group></set>",
            "item"
        ),
        ["/set/group[2]/item[1]"]
    );
}

#[test]
fn names_an_element_in_a_namespace_where_no_prefix_can_be_bound() {
    assert_eq!(
        selectors(
            br#"<s:set xmlns:s="urn:example:set"><s:item/><s:item/></s:set>"#,
            "item"
        ),
        [
            "/*[local-name()='set' and namespace-uri()='urn:example:set']\
             /*[local-name()='item' and namespace-uri()='urn:example:set'][1]",
            "/*[local-name()='set' and namespace-uri()='urn:example:set']\
             /*[local-name()='item' and namespace-uri()='urn:example:set'][2]"
        ]
    );
}

#[test]
fn names_an_element_in_no_namespace_under_one_in_a_namespace_by_its_name() {
    assert_eq!(
        selectors(
            br#"<set xmlns="urn:example:set"><item xmlns=""/></set>"#,
            "item"
        ),
        ["/*[local-name()='set' and namespace-uri()='urn:example:set']/item[1]"]
    );
}

#[test]
fn writes_a_record_that_is_the_document_element_as_one_step_with_no_index() {
    assert_eq!(selectors(br#"<item id="9"/>"#, "item"), ["/item"]);
}

/// The tiny adapter with its input put in a namespace, which its schemas do
/// not declare, so every element is namespaced and the document fails the
/// document schema its envelope names.
struct Namespaced {
    directory: DirectoryResolver,
}

impl Resolver for Namespaced {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("fixtures/in/two.xml") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(
            text.contains("<catalog>"),
            "the input has a document element"
        );
        Ok(text
            .replace("<catalog>", &format!("<catalog xmlns=\"{CATALOG}\">"))
            .into_bytes())
    }
}

/// Every literal a finding carries, as N-Triples writes it.
fn values(findings: &[Quad]) -> Vec<String> {
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_VALUE)
        .map(|q| q.object.to_string())
        .collect()
}

#[test]
fn names_a_namespaced_document_element_of_a_finding_about_the_document_itself() {
    let resolver = Namespaced {
        directory: DirectoryResolver::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"),
        )
        .expect("resolver"),
    };
    let adapter = load_adapter(&resolver).expect("adapter");
    let prepared = prepare(&adapter, &resolver).expect("prepared");
    let iri = format!("{}fixtures/in/two.xml", resolver.root());
    let xml = resolver.read(&iri).expect("input");
    let conversion = convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            xml: &xml,
        },
    )
    .expect("conversion");

    let values = values(&conversion.findings);
    let document = format!("\"/*[local-name()='catalog' and namespace-uri()='{CATALOG}']\"");
    assert!(values.contains(&document), "{values:?}");
    let record = format!(
        "\"/*[local-name()='catalog' and namespace-uri()='{CATALOG}']\
         /*[local-name()='item' and namespace-uri()='{CATALOG}'][1]\""
    );
    assert!(
        values.contains(&record),
        "a record and its document are written alike: {values:?}"
    );
}
