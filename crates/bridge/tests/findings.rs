// A findings query is a CONSTRUCT whose annotations name the record by
// bridge:thisRecord. The Bridge puts the document in that name's place and
// moves the query's selector under the record's own position, so an adapter
// says what inside a record a finding is about and the Bridge says which
// record that was.
use cascade_bridge::{
    convert, load_adapter, prepare, Conversion, DirectoryResolver, Resolver, Source,
};
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
    conversion(resolver, input).expect("conversion").findings
}

fn conversion(resolver: &dyn Resolver, input: &str) -> cascade_bridge::Result<Conversion> {
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

const TARGET: &str = "[\n      oa:hasSource bridge:thisRecord ;\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value \"note\" ]\n    ]";

/// The tiny adapter with the findings query that builds a target of its own
/// rewritten to name the record itself, which the specification forbids: one
/// name is one node for every finding the query produces.
struct NamedTarget {
    directory: DirectoryResolver,
}

impl Resolver for NamedTarget {
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
fn refuses_a_findings_query_whose_target_is_a_name() {
    let Err(refusal) = conversion(&NamedTarget { directory: tiny() }, "two.xml") else {
        panic!("the named form is accepted");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("item-note-findings.rq"), "{refusal}");
    assert!(refusal.contains("oa:hasTarget"), "{refusal}");
}

#[test]
fn gives_an_annotation_targeting_the_record_itself_a_record_selector_of_its_own() {
    let findings = findings_through(&WholeRecord { directory: tiny() }, "order.xml");
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

/// The tiny adapter with its findings query rewritten, each replacement
/// asserted to have something to replace so a query that moved on cannot leave
/// a test passing on the query it no longer has.
struct Rewritten {
    directory: DirectoryResolver,
    replacements: Vec<(String, String)>,
}

impl Rewritten {
    fn new(replacements: &[(&str, &str)]) -> Self {
        Self {
            directory: tiny(),
            replacements: replacements
                .iter()
                .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
                .collect(),
        }
    }
}

impl Resolver for Rewritten {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("mapping/item-note-findings.rq") {
            return Ok(bytes);
        }
        let mut text = String::from_utf8(bytes).expect("utf-8");
        for (from, to) in &self.replacements {
            assert!(text.contains(from), "the query holds {from:?}");
            text = text.replace(from, to);
        }
        Ok(text.into_bytes())
    }
}

const SOURCELESS: &str =
    "[\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value \"note\" ]\n    ]";

#[test]
fn refuses_a_findings_query_whose_target_names_no_document() {
    let Err(refusal) = conversion(&Rewritten::new(&[(TARGET, SOURCELESS)]), "two.xml") else {
        panic!("a finding about no document is accepted");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("item-note-findings.rq"), "{refusal}");
    assert!(refusal.contains("oa:hasSource"), "{refusal}");
}

#[test]
fn refuses_a_findings_query_whose_annotation_has_no_target() {
    let targeting = format!("oa:hasTarget {TARGET} ;\n    ");
    let Err(refusal) = conversion(&Rewritten::new(&[(&targeting, "")]), "two.xml") else {
        panic!("an annotation about no document is accepted");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("item-note-findings.rq"), "{refusal}");
    assert!(refusal.contains("oa:hasTarget"), "{refusal}");
}

const NOTE: &str = "urn:example:note";

/// The selector inside the target, named from the annotation as well.
const ALSO_NAMED: &str = "[
      oa:hasSource bridge:thisRecord ;
      oa:hasSelector _:sel
    ] ;
    <urn:example:note> _:sel";

#[test]
fn keeps_the_description_of_a_node_in_a_target_the_query_names_from_outside_it() {
    let findings = findings_through(
        &Rewritten::new(&[
            (TARGET, ALSO_NAMED),
            (
                "sh:resultSeverity sh:Info .\n}",
                "sh:resultSeverity sh:Info .\n\n  _:sel a oa:XPathSelector ; rdf:value \"note\" .\n}",
            ),
        ]),
        "two.xml",
    );

    let named = objects(&findings, NOTE);
    assert_eq!(named.len(), 1);
    let described: Vec<String> = findings
        .iter()
        .filter(|q| named.contains(&q.subject.to_string()))
        .map(|q| q.predicate.as_str().to_owned())
        .collect();
    assert!(
        described.contains(&RDF_VALUE.to_owned()),
        "the graph names a node with nothing on it: {described:?}"
    );
}

/// The tiny adapter with the findings query rewritten to construct two
/// annotations about the one target node, which it mints once per solution.
struct SharedTarget {
    directory: DirectoryResolver,
}

const BOTH: &str = "_:note ;
    oa:hasBody [ a oa:TextualBody ; rdf:value \"no term for a free-text note\" ] ;
    sh:resultSeverity sh:Info .

  [] a oa:Annotation ;
    oa:hasTarget _:note ;
    oa:hasBody [ a oa:TextualBody ; rdf:value \"a note is not a title\" ] ;
    sh:resultSeverity sh:Info .

  _:note
    oa:hasSource bridge:thisRecord ;
    oa:hasSelector [ a oa:XPathSelector ; rdf:value \"note\" ] .";

impl Resolver for SharedTarget {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("mapping/item-note-findings.rq") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        let one = format!("{TARGET} ;\n    oa:hasBody [ a oa:TextualBody ; rdf:value \"no term for a free-text note\" ] ;\n    sh:resultSeverity sh:Info .");
        assert!(text.contains(&one), "the query constructs one annotation");
        Ok(text.replace(&one, BOTH).into_bytes())
    }
}

#[test]
fn gives_two_annotations_the_query_pointed_at_one_target_a_record_selector_each() {
    let findings = findings_through(&SharedTarget { directory: tiny() }, "two.xml");
    assert_eq!(
        annotations(&findings),
        3,
        "two about the one note, one about the record with no title"
    );

    let targets = objects(&findings, &format!("{OA}hasTarget"));
    assert_eq!(
        targets.iter().collect::<BTreeSet<_>>().len(),
        3,
        "annotations share a target node: {targets:?}"
    );

    let selectors = objects(&findings, &format!("{OA}hasSelector"));
    assert_eq!(selectors.len(), 3);
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        3,
        "annotations share a record selector: {selectors:?}"
    );

    assert_eq!(objects(&findings, &format!("{OA}refinedBy")).len(), 2);
    assert_eq!(
        selector_values(&findings),
        [
            "\"/catalog/item[1]\"",
            "\"/catalog/item[1]\"",
            "\"/catalog/item[2]\"",
            "\"note\"",
            "\"note\""
        ]
    );

    let sources = objects(&findings, &format!("{OA}hasSource"));
    assert_eq!(sources.len(), 3);
    for source in sources {
        assert!(source.ends_with("fixtures/in/two.xml>"), "{source}");
    }
}

const THIS_RECORD: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#thisRecord";

#[test]
fn retargets_an_annotation_the_query_named_by_this_record() {
    let findings = findings_through(
        &Rewritten::new(&[("[] a oa:Annotation", "bridge:thisRecord a oa:Annotation")]),
        "two.xml",
    );

    assert!(
        !findings.iter().any(|q| q.to_string().contains(THIS_RECORD)),
        "the vocabulary's own name is in the findings graph: {findings:?}"
    );

    let targets = objects(&findings, &format!("{OA}hasTarget"));
    assert_eq!(
        targets.len(),
        2,
        "one target per annotation, the query's own replaced: {targets:?}"
    );
    assert_eq!(
        selector_values(&findings),
        ["\"/catalog/item[1]\"", "\"/catalog/item[2]\"", "\"note\""]
    );
}
