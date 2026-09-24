use super::common;

use common::{converted, tiny, written, Variant, CATALOG, RDF_VALUE};
use oxrdf::Quad;

/// The tiny adapter with its input put in a namespace its schemas do not declare.
fn namespaced() -> Variant {
    Variant::of(tiny()).replacing(
        "fixtures/in/two.xml",
        "<catalog>",
        format!("<catalog xmlns=\"{CATALOG}\">"),
    )
}

fn values(findings: &[Quad]) -> Vec<String> {
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_VALUE)
        .map(|q| written(q.object.clone()))
        .collect()
}

#[test]
fn names_a_namespaced_document_element_of_a_finding_about_the_document_itself() {
    let values = values(&converted(&namespaced(), "two.xml").findings);
    let document = format!("/*[local-name()='catalog' and namespace-uri()='{CATALOG}']");
    assert!(values.contains(&document), "{values:?}");
    let record = format!(
        "/*[local-name()='catalog' and namespace-uri()='{CATALOG}']\
         /*[local-name()='item' and namespace-uri()='{CATALOG}'][1]"
    );
    assert!(
        values.contains(&record),
        "a record and its document are written alike: {values:?}"
    );
}
