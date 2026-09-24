// Validation reports; it never refuses. A record that fails its schema is a
// finding with the record's own position as its selector, and its graph is
// produced all the same.
mod common;

use cascade_bridge::{load_adapter, prepare, DirectoryResolver, Error, Resolver, Result};
use common::{address, annotations, converted, says, tiny, Variant, OA, SH};
use oxrdf::{NamedOrBlankNode, Quad};

/// Every finding as its body, its severity and where it is addressed, joined
/// per finding so a severity is read off the finding whose address it sits
/// beside.
fn rows(findings: &[Quad]) -> Vec<(String, String, String, String)> {
    let mut rows: Vec<(String, String, String, String)> = annotations(findings)
        .iter()
        .map(|annotation| {
            let (record, within) = address(findings, annotation);
            (
                says(findings, annotation, &format!("{OA}hasBody")),
                says(findings, annotation, &format!("{SH}resultSeverity")),
                record,
                within,
            )
        })
        .collect();
    rows.sort();
    rows
}

/// Where each violation is addressed, whatever its body.
fn violations(findings: &[Quad]) -> Vec<(String, String)> {
    let mut addressed: Vec<(String, String)> = common::violations(findings)
        .into_iter()
        .map(|(_, record, within)| (record, within))
        .collect();
    addressed.sort();
    addressed
}

#[test]
fn converts_a_record_that_fails_its_schema_and_reports_it_at_that_record_s_position() {
    let conversion = converted(&tiny(), "invalid.xml");
    assert_eq!(conversion.units, 2);
    assert!(
        conversion.quads.iter().any(
            |q| matches!(&q.subject, NamedOrBlankNode::NamedNode(n) if n.as_str() == "urn:example:item:1")
        ),
        "the record that passed still produced its graph"
    );

    let records: Vec<String> = violations(&conversion.findings)
        .into_iter()
        .map(|(record, _)| record)
        .collect();
    assert!(
        records.contains(&"/catalog/item[2]".to_owned()),
        "{records:?}"
    );
    assert!(
        records.contains(&"/catalog".to_owned()),
        "the document fails its envelope's schema too: {records:?}"
    );
}

#[test]
fn reports_nothing_about_a_document_both_its_schemas_accept() {
    let conversion = converted(&tiny(), "two.xml");
    assert_eq!(
        violations(&conversion.findings),
        Vec::<(String, String)>::new(),
        "{:?}",
        rows(&conversion.findings)
    );
}

#[test]
fn refuses_a_schema_that_includes_a_file_outside_the_adapter() {
    let outside = Variant::of(tiny()).replacing(
        "schema/catalog.xsd",
        "\"item.xsd\"",
        "\"../../../harness.rs\"",
    );
    let adapter = load_adapter(&outside).expect("adapter");
    let Err(error) = prepare(&adapter, &outside) else {
        panic!("a schema outside the adapter was read");
    };
    assert!(
        error.to_string().contains("not inside the adapter"),
        "{error}"
    );
}

/// A comment and a processing instruction are not content, and the record's
/// own type is element-only, so a validator handed the record with both in it
/// draws what it drew without them: nothing.
#[test]
fn reports_nothing_about_a_record_carrying_a_comment_and_an_instruction() {
    let plain = converted(&tiny(), "two.xml");
    let aside = converted(
        &Variant::of(tiny()).replacing(
            "fixtures/in/two.xml",
            "<item id=\"1\">",
            "<item id=\"1\"><!-- said of the first --><?say it again?>",
        ),
        "two.xml",
    );
    assert_eq!(rows(&aside.findings), rows(&plain.findings));
    assert_eq!(
        violations(&aside.findings),
        Vec::<(String, String)>::new(),
        "{:?}",
        rows(&aside.findings)
    );
}

struct Rehomed {
    root: String,
    directory: DirectoryResolver,
}

impl Resolver for Rehomed {
    fn root(&self) -> &str {
        &self.root
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        let Some(rest) = iri.strip_prefix(self.root.as_str()) else {
            return Err(Error::msg(format!("not under {}: {iri}", self.root)));
        };
        self.directory
            .read(&format!("{}{rest}", self.directory.root()))
    }
}

#[test]
fn applies_an_included_schema_under_a_root_that_is_not_a_file() {
    let rehomed = Rehomed {
        root: "s3://b/a/".to_owned(),
        directory: tiny(),
    };
    let records: Vec<String> = violations(&converted(&rehomed, "invalid.xml").findings)
        .into_iter()
        .map(|(record, _)| record)
        .collect();
    assert!(
        records.contains(&"/catalog/item[2]".to_owned()),
        "the item without the id its included schema requires: {records:?}"
    );
}

const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// The tiny adapter with its item schema importing the XML namespace, located
/// where `location` says, and its first item carrying `xml:lang`.
fn importing_the_xml_namespace(location: &str) -> Variant {
    Variant::of(tiny())
        .replacing(
            "schema/item.xsd",
            "<xs:element name=\"item\"",
            format!("<xs:import namespace=\"{XML_NAMESPACE}\"{location}/>\n  <xs:element name=\"item\""),
        )
        .replacing(
            "schema/item.xsd",
            "<xs:attribute name=\"internal\" type=\"xs:string\"/>",
            "<xs:attribute name=\"internal\" type=\"xs:string\"/>\n    <xs:attribute ref=\"xml:lang\"/>",
        )
        .replacing(
            "fixtures/in/two.xml",
            "<item id=\"1\">",
            "<item id=\"1\" xml:lang=\"en\">",
        )
}

#[test]
fn refuses_a_schema_importing_a_namespace_the_adapter_ships_no_schema_for() {
    let unlocated = importing_the_xml_namespace("");
    let adapter = load_adapter(&unlocated).expect("adapter");
    let Err(error) = prepare(&adapter, &unlocated) else {
        panic!("a schema the adapter does not ship was answered");
    };
    let refused = error.to_string();
    assert!(refused.contains("schema/item.xsd"), "{refused}");
    assert!(refused.contains(XML_NAMESPACE), "{refused}");
    assert!(refused.contains("schemaLocation"), "{refused}");
}

#[test]
fn reports_nothing_about_a_record_whose_schema_imports_the_xml_namespace_from_the_adapter() {
    let shipped = importing_the_xml_namespace(" schemaLocation=\"xml.xsd\"").with(
        "schema/xml.xsd",
        format!(
            "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" targetNamespace=\"{XML_NAMESPACE}\">\n  <xs:attribute name=\"lang\" type=\"xs:language\"/>\n</xs:schema>\n"
        ),
    );
    let conversion = converted(&shipped, "two.xml");
    assert_eq!(
        violations(&conversion.findings),
        Vec::<(String, String)>::new(),
        "{:?}",
        rows(&conversion.findings)
    );
}

fn declaring_a_simple_type(name: &str) -> String {
    format!(
        "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\n  <xs:simpleType name=\"{name}\">\n    <xs:restriction base=\"xs:string\"/>\n  </xs:simpleType>\n</xs:schema>\n"
    )
}

#[test]
fn applies_two_included_schemas_whose_paths_differ_only_before_a_colon_segment() {
    let apart = Variant::of(tiny())
        .replacing(
            "schema/item.xsd",
            "<xs:element name=\"item\"",
            "<xs:include schemaLocation=\"v1/ab:/t.xsd\"/>\n  <xs:include schemaLocation=\"v2/ab:/t.xsd\"/>\n  <xs:element name=\"item\"",
        )
        .replacing(
            "schema/item.xsd",
            "<xs:attribute name=\"internal\" type=\"xs:string\"/>",
            "<xs:attribute name=\"internal\" type=\"second\"/>\n    <xs:attribute name=\"external\" type=\"first\"/>",
        )
        .with("schema/v1/ab:/t.xsd", declaring_a_simple_type("first"))
        .with("schema/v2/ab:/t.xsd", declaring_a_simple_type("second"));
    let conversion = converted(&apart, "two.xml");
    assert_eq!(
        violations(&conversion.findings),
        Vec::<(String, String)>::new(),
        "{:?}",
        rows(&conversion.findings)
    );
}
