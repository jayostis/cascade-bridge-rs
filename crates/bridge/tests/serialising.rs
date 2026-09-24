// The harness canonicalises before it compares, so it sees none of what this file holds.
mod common;

use cascade_bridge::{
    canonical_lines, convert, load_adapter, prepare, serialise, serialise_at, Conversion,
    GraphFormat, Resolver, Source,
};
use common::{committed, conversion, tiny, Variant, CRATE, RDF_TYPE};
use oxrdf::{NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;

const TABLE: &str = "_:shared <urn:example:catalog#about> \"the whole catalog\" .";

/// With `table`, a lookup table whose blank node carries one label into every record's store.
fn mapping(text: String, table: bool) -> Variant {
    let replaced = Variant::of(tiny()).with("mapping/item.rq", text);
    if !table {
        return replaced;
    }
    replaced
        .with("table/catalog.ttl", TABLE)
        .replacing(
            CRATE,
            "\"bridge:mapping\": \"bridge:mapping\"",
            "\"bridge:mapping\": \"bridge:mapping\",\n      \"bridge:table\": \"bridge:table\"",
        )
        .replacing(
            CRATE,
            "\"bridge:detectQuery\": { \"@id\": \"mapping/detect.rq\" },",
            "\"bridge:table\": { \"@id\": \"table/catalog.ttl\" },\n      \"bridge:detectQuery\": { \"@id\": \"mapping/detect.rq\" },",
        )
        .replacing(
            CRATE,
            "{\n      \"@id\": \"#envelope-catalog\",",
            "{\n      \"@id\": \"table/catalog.ttl\",\n      \"@type\": \"File\",\n      \"encodingFormat\": \"text/turtle\"\n    },\n    {\n      \"@id\": \"#envelope-catalog\",",
        )
}

const PROLOGUE: &str = "
PREFIX fx:  <http://sparql.xyz/facade-x/ns/>
PREFIX xyz: <http://sparql.xyz/facade-x/data/>
PREFIX ex:  <urn:example:catalog#>
PREFIX ns:  <https://ns.example.org/>
PREFIX v1:  <https://ns.example.org/v1#>
";

const WHERE: &str = "
WHERE {
  ?item a fx:root, xyz:item ; xyz:id ?id .
  BIND(IRI(CONCAT(\"urn:example:item:\", ?id)) AS ?s)
}
";

fn graph(construct: &str, format: GraphFormat) -> String {
    through(construct, WHERE, format, false)
}

fn through(construct: &str, matching: &str, format: GraphFormat, table: bool) -> String {
    converted(construct, matching, format, table).1
}

/// The conversion and the text it was written as.
fn converted(
    construct: &str,
    matching: &str,
    format: GraphFormat,
    table: bool,
) -> (Conversion, String) {
    let resolver = mapping(
        format!("{PROLOGUE}CONSTRUCT {{ {construct} }}{matching}"),
        table,
    );
    let adapter = load_adapter(&resolver).expect("adapter");
    let prepared = prepare(&adapter, &resolver).expect("prepared");
    let xml = resolver
        .read(&format!("{}fixtures/in/two.xml", resolver.root()))
        .expect("input");
    let conversion = convert(
        &prepared,
        Source {
            iri: "urn:example:document",
            envelope: None,
            xml: &xml,
        },
    )
    .expect("conversion");
    assert_eq!(conversion.units, 2);
    let written = serialise(&conversion.quads, format, &prepared.prefixes).expect("graph");
    (conversion, written)
}

/// Every IRI a case writes is absolute, so no base is needed to read it back.
fn read_back(written: &str, format: RdfFormat) -> Vec<Quad> {
    RdfParser::from_format(format)
        .for_slice(written.as_bytes())
        .map(|quad| quad.expect("what was written parses"))
        .collect()
}

fn typed(quads: &[Quad], class: &str) -> usize {
    quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == class))
        .count()
}

fn blank_nodes(quads: &[Quad]) -> BTreeSet<String> {
    let mut named = BTreeSet::new();
    for quad in quads {
        if let NamedOrBlankNode::BlankNode(node) = &quad.subject {
            named.insert(node.as_str().to_owned());
        }
        if let Term::BlankNode(node) = &quad.object {
            named.insert(node.as_str().to_owned());
        }
    }
    named
}

#[test]
fn writes_a_triple_every_record_constructs_once() {
    let written = read_back(
        &graph(
            "?s a ex:Item . <urn:example:catalog> a ex:Catalog .",
            GraphFormat::NTriples,
        ),
        RdfFormat::NTriples,
    );
    assert_eq!(typed(&written, "urn:example:catalog#Catalog"), 1);
    assert_eq!(typed(&written, "urn:example:catalog#Item"), 2);
}

#[test]
fn keeps_two_records_blank_nodes_apart() {
    let written = graph("?s ex:note [ ex:about ?id ] .", GraphFormat::NTriples);
    assert_eq!(
        blank_nodes(&read_back(&written, RdfFormat::NTriples)).len(),
        2,
        "{written}"
    );
}

#[test]
fn keeps_two_records_apart_when_they_were_handed_one_label() {
    let written = through(
        "?s ex:note ?shared .",
        "
WHERE {
  ?item a fx:root, xyz:item ; xyz:id ?id .
  BIND(IRI(CONCAT(\"urn:example:item:\", ?id)) AS ?s)
  ?shared ex:about \"the whole catalog\" .
}
",
        GraphFormat::NTriples,
        true,
    );
    let quads = read_back(&written, RdfFormat::NTriples);
    assert_eq!(
        quads
            .iter()
            .filter(|q| q.predicate.as_str() == "urn:example:catalog#note")
            .count(),
        2,
        "{written}"
    );
    assert_eq!(blank_nodes(&quads).len(), 2, "{written}");
}

#[test]
fn counts_the_triples_the_written_graph_holds() {
    let (conversion, written) = converted(
        "?s a ex:Item . <urn:example:catalog> a ex:Catalog .",
        WHERE,
        GraphFormat::NTriples,
        false,
    );
    assert_eq!(
        conversion.quads.len(),
        4,
        "the raw union: both records construct the catalog's type"
    );
    assert_eq!(
        conversion.triples(),
        3,
        "the catalog's type once, and each record's own"
    );
    assert_eq!(
        read_back(&written, RdfFormat::NTriples).len(),
        conversion.triples()
    );
}

/// A prefix declaration is a property of the text, so the text is read as written.
#[test]
fn declares_no_prefix_that_is_only_a_path_of_an_iri_the_graph_names() {
    let written = graph("?s a v1:Item .", GraphFormat::Turtle);
    assert!(
        written.contains("@prefix v1: <https://ns.example.org/v1#>"),
        "{written}"
    );
    assert!(!written.contains("@prefix ns:"), "{written}");
    assert_eq!(
        typed(
            &read_back(&written, RdfFormat::Turtle),
            "https://ns.example.org/v1#Item"
        ),
        2
    );
}

#[test]
fn writes_turtle_with_the_names_the_mapping_gave_the_namespaces_it_used() {
    let written = graph("?s a ex:Item .", GraphFormat::Turtle);
    assert!(
        written.contains("@prefix ex: <urn:example:catalog#>"),
        "{written}"
    );
    assert!(written.contains("ex:Item"), "{written}");
    // The lift's namespaces are in the mapping's prologue and not in its
    // output, so they are not declared over a graph that never names them.
    assert!(!written.contains("facade-x"), "{written}");
    assert_eq!(
        typed(
            &read_back(&written, RdfFormat::Turtle),
            "urn:example:catalog#Item"
        ),
        2
    );
}

#[test]
fn writes_the_same_findings_graph_as_turtle_as_it_does_as_n_triples() {
    let resolver = tiny();
    let adapter = load_adapter(&resolver).expect("adapter");
    let prepared = prepare(&adapter, &resolver).expect("prepared");
    let mut compared = 0;
    for input in committed() {
        let findings = conversion(&resolver, &input).expect("conversion").findings;
        if findings.is_empty() {
            continue;
        }
        let at = format!("{}fixtures/findings/{input}.ttl", resolver.root());
        let turtle = serialise_at(
            &findings,
            GraphFormat::Turtle,
            &prepared.findings_prefixes,
            Some(&at),
        )
        .expect("turtle");
        let ntriples = serialise_at(
            &findings,
            GraphFormat::NTriples,
            &prepared.findings_prefixes,
            Some(&at),
        )
        .expect("n-triples");
        let as_turtle: Vec<Quad> = RdfParser::from_format(RdfFormat::Turtle)
            .with_base_iri(&at)
            .expect("the file's IRI")
            .for_slice(turtle.as_bytes())
            .map(|quad| quad.expect("the Turtle parses"))
            .collect();
        assert_eq!(
            canonical_lines(as_turtle).expect("canonical"),
            canonical_lines(read_back(&ntriples, RdfFormat::NTriples)).expect("canonical"),
            "{input}:\n{turtle}"
        );
        compared += 1;
    }
    assert!(
        compared > 3,
        "only {compared} documents had findings to compare"
    );
}

#[test]
#[ignore = "Oxigraph 0.5.11 returns derived XSD integer types such as `xsd:positiveInteger` as `xsd:integer`, which SPARQL 1.1 does not allow, so an expected graph that uses them cannot pass on this Bridge."]
fn keeps_the_derived_integer_type_a_mapping_gave_a_literal() {
    const POSITIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#positiveInteger";
    let (conversion, written) = converted(
        "?s ex:copies ?copies .",
        &WHERE.replace(
            "AS ?s)",
            &format!("AS ?s)\n  BIND(\"3\"^^<{POSITIVE_INTEGER}> AS ?copies)"),
        ),
        GraphFormat::NTriples,
        false,
    );
    let copies: Vec<&str> = conversion
        .quads
        .iter()
        .filter_map(|q| match &q.object {
            Term::Literal(literal) => Some(literal.datatype().as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(copies, [POSITIVE_INTEGER; 2], "{written}");
}
