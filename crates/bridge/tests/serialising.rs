// What a produced graph must survive being written down: a triple constructed
// for every record is written once, and two records' blank nodes stay two.
// The test harness sees neither, because it canonicalises before it compares.
use cascade_bridge::{
    convert, load_adapter, prepare, serialise, Conversion, DirectoryResolver, GraphFormat,
    Resolver, Source,
};
use std::path::PathBuf;

/// The tiny adapter with its one mapping replaced, so a mapping that emits
/// what this file is about can be run without a second adapter, and with a
/// lookup table beside it, whose blank node carries one label into every
/// record's store.
struct Mapping {
    directory: DirectoryResolver,
    text: String,
    table: bool,
}

const TABLE: &str = "_:shared <urn:example:catalog#about> \"the whole catalog\" .";

impl Resolver for Mapping {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        if iri.ends_with("mapping/item.rq") {
            return Ok(self.text.as_bytes().to_vec());
        }
        if !self.table {
            return self.directory.read(iri);
        }
        if iri.ends_with("table/catalog.ttl") {
            return Ok(TABLE.as_bytes().to_vec());
        }
        let text = String::from_utf8(self.directory.read(iri)?).expect("utf-8");
        if !iri.ends_with("ro-crate-metadata.json") {
            return Ok(text.into_bytes());
        }
        Ok(text
            .replace(
                "\"bridge:mapping\": \"bridge:mapping\"",
                "\"bridge:mapping\": \"bridge:mapping\",\n      \"bridge:table\": \"bridge:table\"",
            )
            .replace(
                "\"bridge:mapping\": { \"@id\": \"mapping/item.rq\" },",
                "\"bridge:mapping\": { \"@id\": \"mapping/item.rq\" },\n      \"bridge:table\": { \"@id\": \"table/catalog.ttl\" },",
            )
            .replace(
                "{\n      \"@id\": \"#envelope-catalog\",",
                "{\n      \"@id\": \"table/catalog.ttl\",\n      \"@type\": \"File\",\n      \"encodingFormat\": \"text/turtle\"\n    },\n    {\n      \"@id\": \"#envelope-catalog\",",
            )
            .into_bytes())
    }
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

/// The tiny adapter's two-record input, converted through the given mapping
/// and written out.
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
    let resolver = Mapping {
        directory: DirectoryResolver::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"),
        )
        .expect("resolver"),
        text: format!("{PROLOGUE}CONSTRUCT {{ {construct} }}{matching}"),
        table,
    };
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

#[test]
fn writes_a_triple_every_record_constructs_once() {
    let written = graph(
        "?s a ex:Item . <urn:example:catalog> a ex:Catalog .",
        GraphFormat::NTriples,
    );
    assert_eq!(written.matches("urn:example:catalog#Catalog").count(), 1);
    assert_eq!(written.matches("urn:example:catalog#Item").count(), 2);
}

/// The labels of the blank nodes a written graph names, each once.
fn labels(written: &str) -> Vec<&str> {
    let mut labels: Vec<&str> = written
        .split("_:")
        .skip(1)
        .map(|rest| rest.split([' ', '\n']).next().unwrap_or_default())
        .collect();
    labels.sort_unstable();
    labels.dedup();
    labels
}

#[test]
fn keeps_two_records_blank_nodes_apart() {
    let written = graph("?s ex:note [ ex:about ?id ] .", GraphFormat::NTriples);
    assert_eq!(labels(&written).len(), 2, "{written}");
}

/// A table is parsed once and loaded beside every record, so a mapping that
/// puts the table's own node in its graph hands every record the same label.
/// Merging two graphs is standardising their blank nodes apart, not taking a
/// shared label for a shared node, which is what a query engine free to label
/// two executions alike would otherwise cost.
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
    assert_eq!(written.matches("urn:example:catalog#note").count(), 2);
    assert_eq!(labels(&written).len(), 2, "{written}");
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
    assert_eq!(conversion.triples(), written.lines().count());
}

#[test]
fn declares_no_prefix_that_is_only_a_path_of_an_iri_the_graph_names() {
    let written = graph("?s a v1:Item .", GraphFormat::Turtle);
    assert!(
        written.contains("@prefix v1: <https://ns.example.org/v1#>"),
        "{written}"
    );
    assert!(!written.contains("@prefix ns:"), "{written}");
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
}
