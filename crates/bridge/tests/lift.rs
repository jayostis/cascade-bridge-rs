// The lift vectors of the Cascade Bridge Specification, judged by the
// bridge:LiftTest and bridge:SkeletonTest rules its vocabulary states.
//
// The files in tests/lift/ are copies of fixtures/lift/ in
// jayostis/cascade-bridge-spec, which is the authority: a disagreement is this
// Bridge's to fix, and a change there is copied here rather than argued with.
use cascade_bridge::{canonical_lines, lift_slice};
use oxigraph::store::Store;
use oxrdf::Quad;
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Each skeleton vector with the bridge:elementNameOfEachRecord its manifest
/// entry names. Every other vector in the directory is a lift vector, so one
/// copied in is run whatever it is, and a skeleton vector missing here fails as
/// a lift vector rather than going unrun.
const SKELETON_VECTORS: [(&str, &str); 2] =
    [("skeleton", "Unit"), ("skeleton-record-root", "Unit")];

fn vectors_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/lift")
}

fn vector(name: &str, extension: &str) -> Vec<u8> {
    let path = vectors_directory().join(format!("{name}.{extension}"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn vector_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(vectors_directory())
        .expect("the vectors")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "xml"))
        .map(|path| {
            path.file_stem()
                .expect("a name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn canonical(quads: Vec<Quad>) -> BTreeSet<String> {
    canonical_lines(quads).expect("canonical")
}

fn lines(store: &Store) -> BTreeSet<String> {
    canonical(store.iter().map(|q| q.expect("quad")).collect())
}

fn parsed(ntriples: &[u8]) -> BTreeSet<String> {
    canonical(
        RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(ntriples)
            .map(|q| q.expect("expected N-Triples"))
            .collect(),
    )
}

/// Without a unit the whole document is the skeleton, which is the lift of the
/// document with its document element as the lift root.
fn lift_whole(xml: &[u8]) -> BTreeSet<String> {
    lines(
        &lift_slice(xml, None)
            .expect("lift")
            .into_skeleton()
            .expect("skeleton"),
    )
}

fn assert_graph(what: &str, produced: BTreeSet<String>, expected: BTreeSet<String>) {
    assert_eq!(
        produced,
        expected,
        "{what}: missing {:?}, extra {:?}",
        expected.difference(&produced).collect::<Vec<_>>(),
        produced.difference(&expected).collect::<Vec<_>>()
    );
}

fn assert_vector(name: &str, produced: BTreeSet<String>) {
    assert_graph(
        &format!("the {name} vector"),
        produced,
        parsed(&vector(name, "nt")),
    );
}

#[test]
fn reproduces_every_lift_vector_of_the_specification() {
    let lifts: Vec<String> = vector_names()
        .into_iter()
        .filter(|name| {
            !SKELETON_VECTORS
                .iter()
                .any(|(skeleton, _)| skeleton == name)
        })
        .collect();
    assert!(lifts.len() >= 8, "{lifts:?}");
    for name in lifts {
        assert_vector(&name, lift_whole(&vector(&name, "xml")));
    }
}

#[test]
fn reproduces_every_skeleton_vector_of_the_specification() {
    let names = vector_names();
    for (name, record) in SKELETON_VECTORS {
        assert!(names.iter().any(|n| n == name), "no {name} vector");
        let store = lift_slice(&vector(name, "xml"), Some(record))
            .expect("lift")
            .into_skeleton()
            .expect("skeleton");
        assert_vector(name, lines(&store));
    }
}

const FIRST_UNIT: &str = r#"
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/ns/root> .
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/item> .
_:item <http://sparql.xyz/facade-x/data/id> "1" .
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#_1> _:t .
_:t <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/t> .
_:t <http://www.w3.org/1999/02/22-rdf-syntax-ns#_1> "x" .
"#;

const SECOND_UNIT: &str = r#"
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/ns/root> .
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/item> .
_:item <http://sparql.xyz/facade-x/data/id> "2" .
"#;

/// Each unit keeps its type triple and its slot, and nothing else.
const SKELETON: &str = r#"
_:set <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/ns/root> .
_:set <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/set> .
_:set <http://sparql.xyz/facade-x/data/n> "2" .
_:set <http://www.w3.org/1999/02/22-rdf-syntax-ns#_1> _:first .
_:set <http://www.w3.org/1999/02/22-rdf-syntax-ns#_2> _:second .
_:first <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/item> .
_:second <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/item> .
"#;

#[test]
fn lifts_each_unit_with_the_unit_as_root_and_empties_it_in_the_skeleton() {
    let xml = br#"<set n="2"><item id="1"><t>x</t></item><item id="2"/></set>"#;
    let mut lift = lift_slice(xml, Some("item")).expect("lift");
    let mut units = Vec::new();
    while let Some(unit) = lift.next_unit().expect("unit") {
        units.push(lines(&unit.store));
    }
    assert_eq!(units.len(), 2);
    assert_graph(
        "the first unit",
        units.remove(0),
        parsed(FIRST_UNIT.as_bytes()),
    );
    assert_graph(
        "the second unit",
        units.remove(0),
        parsed(SECOND_UNIT.as_bytes()),
    );
    assert_graph(
        "the skeleton",
        lines(&lift.into_skeleton().expect("skeleton")),
        parsed(SKELETON.as_bytes()),
    );
}

#[test]
fn treats_a_document_whose_element_is_the_unit_as_one_unit() {
    let mut lift = lift_slice(br#"<item id="9"/>"#, Some("item")).expect("lift");
    let unit = lift.next_unit().expect("unit").expect("one unit");
    assert_graph(
        "the one unit",
        lines(&unit.store),
        parsed(
            br#"
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/ns/root> .
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/item> .
_:item <http://sparql.xyz/facade-x/data/id> "9" .
"#,
        ),
    );
    assert!(lift.next_unit().expect("end").is_none());
}

/// The values an attribute of this local name lifts to.
fn attribute(xml: &[u8], local: &str) -> Vec<String> {
    let predicate = format!("http://sparql.xyz/facade-x/data/{local}");
    lift_slice(xml, None)
        .expect("lift")
        .into_skeleton()
        .expect("skeleton")
        .iter()
        .map(|q| q.expect("quad"))
        .filter(|q| q.predicate.as_str() == predicate)
        .map(|q| match q.object {
            oxrdf::Term::Literal(l) => l.value().to_owned(),
            other => panic!("{local} lifted to {other}"),
        })
        .collect()
}

#[test]
fn normalises_an_attribute_value_as_xml_does_whatever_the_checkout() {
    // A line break, however the checkout wrote it, and a tab are each a space;
    // a character reference is the character it names, whitespace or not.
    for xml in [
        &b"<item note=\"a\r\nb\tc&#10;d&#9;e\"/>"[..],
        b"<item note=\"a\nb\tc&#10;d&#9;e\"/>",
        b"<item note=\"a\rb\tc&#10;d&#9;e\"/>",
    ] {
        assert_eq!(
            attribute(xml, "note"),
            ["a b c\nd\te"],
            "{}",
            String::from_utf8_lossy(xml).escape_debug()
        );
    }
}

/// The record the lift writes out again is what a validator is handed and what
/// an address is followed through, and XPath counts a comment and a processing
/// instruction, so each stands in it where the document wrote it. Neither is a
/// member of the graph: the characters either side of one are still a single
/// text child there.
#[test]
fn writes_a_record_out_with_the_comments_and_instructions_it_held() {
    let document =
        b"<catalog><!-- before --><item>before<!-- inside --><?say it?>after<e/></item></catalog>";
    let unit = lift_slice(document, Some("item"))
        .expect("lift")
        .next()
        .expect("a unit")
        .expect("a unit");
    assert_eq!(
        unit.xml,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><item>before<!-- inside --><?say it?>after<e></e></item>"
    );
    assert_graph(
        "the unit",
        lines(&unit.store),
        parsed(
            br#"
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/ns/root> .
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/item> .
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#_1> "beforeafter" .
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#_2> _:e .
_:e <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/e> .
"#,
        ),
    );
}
