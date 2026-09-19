// Validation reports; it never refuses. A record that fails its schema is a
// finding with the record's own position as its selector, and its graph is
// produced all the same.
use cascade_bridge::{convert, load_adapter, prepare, DirectoryResolver, Resolver, Source};
use oxrdf::{Quad, Term};
use std::path::PathBuf;

const OA: &str = "http://www.w3.org/ns/oa#";
const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";

fn tiny() -> DirectoryResolver {
    DirectoryResolver::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"))
        .expect("resolver")
}

fn objects(quads: &[Quad], predicate: &str) -> Vec<String> {
    let mut written: Vec<String> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == predicate)
        .map(|q| q.object.to_string())
        .collect();
    written.sort();
    written
}

/// Whether a blank node is the subject of a triple naming this type.
fn typed(quads: &[Quad], node: &str, type_iri: &str) -> bool {
    quads.iter().any(|q| {
        q.subject.to_string() == node
            && q.predicate.as_str() == RDF_TYPE
            && matches!(&q.object, Term::NamedNode(n) if n.as_str() == type_iri)
    })
}

fn run(resolver: &dyn Resolver, input: &str) -> cascade_bridge::Conversion {
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

#[test]
fn converts_a_record_that_fails_its_schema_and_reports_it_at_that_record_s_position() {
    let conversion = run(&tiny(), "invalid.xml");
    assert_eq!(conversion.units, 2);
    assert!(
        conversion
            .quads
            .iter()
            .any(|q| q.subject.to_string() == "<urn:example:item:1>"),
        "the record that passed still produced its graph"
    );

    let findings = &conversion.findings;
    let violations: Vec<&Quad> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{SH}resultSeverity"))
        .filter(
            |q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == format!("{SH}Violation")),
        )
        .collect();
    assert!(!violations.is_empty(), "{findings:?}");
    for violation in &violations {
        assert!(typed(
            findings,
            &violation.subject.to_string(),
            &format!("{OA}Annotation")
        ));
    }

    let positions = objects(findings, RDF_VALUE);
    assert!(
        positions.contains(&"\"/catalog/item[2]\"".to_owned()),
        "{positions:?}"
    );
    assert!(
        positions.contains(&"\"/catalog\"".to_owned()),
        "the document fails its envelope's schema too: {positions:?}"
    );
}

#[test]
fn reports_nothing_about_a_document_both_its_schemas_accept() {
    let conversion = run(&tiny(), "two.xml");
    let severities = objects(&conversion.findings, &format!("{SH}resultSeverity"));
    assert!(
        !severities.contains(&format!("<{SH}Violation>")),
        "{severities:?}"
    );
}

/// The tiny adapter with its document schema including a file outside the
/// crate.
struct Outside {
    directory: DirectoryResolver,
}

impl Resolver for Outside {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("schema/catalog.xsd") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        Ok(text
            .replace("\"item.xsd\"", "\"../../../harness.rs\"")
            .into_bytes())
    }
}

#[test]
fn refuses_a_schema_that_includes_a_file_outside_the_adapter() {
    let resolver = Outside { directory: tiny() };
    let adapter = load_adapter(&resolver).expect("adapter");
    let Err(error) = prepare(&adapter, &resolver) else {
        panic!("a schema outside the adapter was read");
    };
    assert!(
        error.to_string().contains("not inside the adapter"),
        "{error}"
    );
}
