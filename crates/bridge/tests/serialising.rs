// What a produced graph must survive being written down: a triple constructed
// for every record is written once, and two records' blank nodes stay two.
// The test harness sees neither, because it canonicalises before it compares.
use cascade_bridge::{
    convert, load_adapter, prepare, serialise, DirectoryResolver, GraphFormat, Resolver,
};
use std::path::PathBuf;

/// The tiny adapter with its one mapping replaced, so a mapping that emits
/// what this file is about can be run without a second adapter.
struct Mapping {
    directory: DirectoryResolver,
    text: String,
}

impl Resolver for Mapping {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        if iri.ends_with("mapping/item.rq") {
            return Ok(self.text.as_bytes().to_vec());
        }
        self.directory.read(iri)
    }
}

const PROLOGUE: &str = "
PREFIX fx:  <http://sparql.xyz/facade-x/ns/>
PREFIX xyz: <http://sparql.xyz/facade-x/data/>
PREFIX ex:  <urn:example:catalog#>
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
    let resolver = Mapping {
        directory: DirectoryResolver::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"),
        )
        .expect("resolver"),
        text: format!("{PROLOGUE}CONSTRUCT {{ {construct} }}{WHERE}"),
    };
    let adapter = load_adapter(&resolver).expect("adapter");
    let prepared = prepare(&adapter, &resolver).expect("prepared");
    let xml = resolver
        .read(&format!("{}fixtures/in/two.xml", resolver.root()))
        .expect("input");
    let conversion = convert(&prepared, &xml).expect("conversion");
    assert_eq!(conversion.units, 2);
    serialise(&conversion.quads, format, &prepared.prefixes).expect("graph")
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

#[test]
fn keeps_two_records_blank_nodes_apart() {
    let written = graph("?s ex:note [ ex:about ?id ] .", GraphFormat::NTriples);
    let mut labels: Vec<&str> = written
        .split("_:")
        .skip(1)
        .map(|rest| rest.split([' ', '\n']).next().unwrap_or_default())
        .collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), 2, "{written}");
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
