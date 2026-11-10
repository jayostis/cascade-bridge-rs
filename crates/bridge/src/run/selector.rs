use crate::fixtures::{converted, tiny, written, Variant, CATALOG, RDF_VALUE};
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

mod tests {
    use super::*;

    /// Findings about a document · Conversion::findings()
    mod findings_about_a_document {
        use super::*;

        mod given_a_findings_query {
            use super::*;

            /// writes a namespaced document and its record by local name and namespace in a finding's selector
            #[test]
            fn writes_a_namespaced_document_and_its_record_by_local_name_and_namespace_in_a_finding_s_selector(
            ) {
                let values = values(&converted(&namespaced(), "two.xml").findings);
                let document =
                    format!("/*[local-name()='catalog' and namespace-uri()='{CATALOG}']");
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
        }
    }
}
