// A findings query is a CONSTRUCT whose annotations name the record by
// bridge:thisRecord. The Bridge puts the document in that name's place and
// moves the query's selector under the record's own position, so an adapter
// says what inside a record a finding is about and the Bridge says which
// record that was.
use cascade_bridge::{convert, load_adapter, prepare, DirectoryResolver, Resolver, Source};
use oxrdf::{Quad, Term};
use std::collections::BTreeSet;
use std::path::PathBuf;

const OA: &str = "http://www.w3.org/ns/oa#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";

fn tiny() -> DirectoryResolver {
    DirectoryResolver::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"))
        .expect("resolver")
}

/// The tiny adapter's findings for one of its committed inputs.
fn findings_for(input: &str) -> Vec<Quad> {
    findings_through(&tiny(), input)
}

fn findings_through(resolver: &dyn Resolver, input: &str) -> Vec<Quad> {
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
    .findings
}

/// Every object of a predicate, written as N-Triples writes it.
fn objects(quads: &[Quad], predicate: &str) -> Vec<String> {
    let mut written: Vec<String> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == predicate)
        .map(|q| q.object.to_string())
        .collect();
    written.sort();
    written
}

/// The XPath every selector node holds, sorted.
fn selector_values(quads: &[Quad]) -> Vec<String> {
    let selector = format!("{OA}XPathSelector");
    let selectors: Vec<String> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == selector))
        .map(|q| q.subject.to_string())
        .collect();
    let mut values: Vec<String> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_VALUE)
        .filter(|q| selectors.contains(&q.subject.to_string()))
        .map(|q| q.object.to_string())
        .collect();
    values.sort();
    values
}

fn annotations(quads: &[Quad]) -> usize {
    let annotation = format!("{OA}Annotation");
    quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == annotation))
        .count()
}

#[test]
fn names_the_document_the_record_was_read_from_where_the_query_named_this_record() {
    let findings = findings_for("two.xml");
    let sources = objects(&findings, &format!("{OA}hasSource"));
    assert_eq!(sources.len(), 2);
    for source in sources {
        assert!(source.ends_with("fixtures/in/two.xml>"), "{source}");
    }
}

#[test]
fn moves_the_query_s_selector_under_the_record_s_own_position() {
    let findings = findings_for("two.xml");
    assert_eq!(
        objects(&findings, &format!("{OA}refinedBy")).len(),
        1,
        "one finding of two names a node inside its record"
    );

    assert_eq!(
        selector_values(&findings),
        ["\"/catalog/item[1]\"", "\"/catalog/item[2]\"", "\"note\""]
    );
}

#[test]
fn gives_every_annotation_a_record_selector_of_its_own() {
    let findings = findings_for("order.xml");
    let selectors = objects(&findings, &format!("{OA}hasSelector"));
    assert_eq!(annotations(&findings), 4);
    assert_eq!(objects(&findings, &format!("{OA}hasTarget")).len(), 4);
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        4,
        "two annotations share a selector node: {selectors:?}"
    );
}

#[test]
fn counts_a_finding_a_record_produced_twice_twice() {
    let notes = selector_values(&findings_for("order.xml"))
        .into_iter()
        .filter(|value| value == "\"note\"")
        .count();
    assert_eq!(notes, 3, "two of one record's, one of the other's");
}

/// The tiny adapter with the findings query that names a node inside the
/// record rewritten to name none.
struct WholeRecord {
    directory: DirectoryResolver,
}

const SELECTOR: &str = " ;\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value \"note\" ]";

impl Resolver for WholeRecord {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("mapping/item-note-findings.rq") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(text.contains(SELECTOR), "the query names a selector");
        Ok(text.replace(SELECTOR, "").into_bytes())
    }
}

#[test]
fn selects_the_record_itself_for_a_query_that_writes_no_selector() {
    let findings = findings_through(&WholeRecord { directory: tiny() }, "two.xml");
    assert_eq!(annotations(&findings), 2);
    assert!(objects(&findings, &format!("{OA}refinedBy")).is_empty());
    assert_eq!(objects(&findings, &format!("{OA}hasSelector")).len(), 2);
}

/// The tiny adapter with the findings query that builds a target of its own
/// rewritten to name the record itself, which is the same node for every
/// annotation of every record.
struct RecordItself {
    directory: DirectoryResolver,
}

const TARGET: &str = "[\n      oa:hasSource bridge:thisRecord ;\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value \"note\" ]\n    ]";

impl Resolver for RecordItself {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("mapping/item-note-findings.rq") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(
            text.contains(TARGET),
            "the query builds a target of its own"
        );
        Ok(text.replace(TARGET, "bridge:thisRecord").into_bytes())
    }
}

#[test]
fn gives_an_annotation_targeting_the_record_itself_a_record_selector_of_its_own() {
    let findings = findings_through(&RecordItself { directory: tiny() }, "order.xml");
    assert_eq!(annotations(&findings), 4);

    let targets = objects(&findings, &format!("{OA}hasTarget"));
    assert_eq!(
        targets.iter().collect::<BTreeSet<_>>().len(),
        4,
        "annotations share a target node: {targets:?}"
    );

    let selectors = objects(&findings, &format!("{OA}hasSelector"));
    assert_eq!(selectors.len(), 4);
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        4,
        "annotations share a record selector: {selectors:?}"
    );

    let sources = objects(&findings, &format!("{OA}hasSource"));
    assert_eq!(sources.len(), 4);
    for source in sources {
        assert!(source.ends_with("fixtures/in/order.xml>"), "{source}");
    }

    assert_eq!(
        selector_values(&findings),
        [
            "\"/catalog/item[1]\"",
            "\"/catalog/item[1]\"",
            "\"/catalog/item[1]\"",
            "\"/catalog/item[2]\""
        ]
    );
}
