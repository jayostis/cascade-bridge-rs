// Validation reports; it never refuses. A record that fails its schema is a
// finding with the record's own position as its selector, and its graph is
// produced all the same.
mod common;

use cascade_bridge::{load_adapter, prepare};
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
