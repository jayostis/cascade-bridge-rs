// tests/lift/ is a copy of fixtures/lift/ in jayostis/cascade-bridge-spec, the authority.
use super::{lift_slice, lift_slice_breaking};
use crate::fixtures::{fixture, fixture_names};
use crate::rdf::canonical_lines;
use oxigraph::store::Store;
use oxrdf::Quad;
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;

/// Every other vector in the directory is run as a lift vector.
const SKELETON_VECTORS: [(&str, &str); 2] =
    [("skeleton", "Unit"), ("skeleton-record-root", "Unit")];

/// Each vector whose manifest entry names elements breaking text.
const BREAKING_VECTORS: [&str; 3] = [
    "text-with-element-breaks-inline",
    "text-with-element-breaks-paragraphs",
    "text-with-element-breaks-table",
];
const BREAKING: [&str; 5] = ["br", "paragraph", "td", "th", "tr"];

const VECTORS: &str = "tests/lift";

fn vector(name: &str, extension: &str) -> Vec<u8> {
    fixture(&format!("{VECTORS}/{name}.{extension}"))
}

fn vector_names() -> Vec<String> {
    let mut names: Vec<String> = fixture_names(VECTORS)
        .into_iter()
        .filter_map(|name| name.strip_suffix(".xml").map(str::to_owned))
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

fn document_element(xml: &[u8]) -> String {
    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer).expect("an event") {
            quick_xml::events::Event::Start(start) | quick_xml::events::Event::Empty(start) => {
                return String::from_utf8(start.local_name().as_ref().to_vec()).expect("utf-8")
            }
            quick_xml::events::Event::Eof => panic!("no document element"),
            _ => buffer.clear(),
        }
    }
}

fn lift_whole(xml: &[u8], breaking: &[String]) -> BTreeSet<String> {
    let mut lift = lift_slice_breaking(xml, Some(&document_element(xml)), breaking).expect("lift");
    let unit = lift
        .next_unit()
        .expect("unit")
        .expect("the document element");
    lines(&unit.store)
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
        let breaking: Vec<String> = match BREAKING_VECTORS.contains(&name.as_str()) {
            true => BREAKING.map(str::to_owned).to_vec(),
            false => Vec::new(),
        };
        assert_vector(&name, lift_whole(&vector(&name, "xml"), &breaking));
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
_:item <https://dom.spec.whatwg.org/#dom-node-textcontent> "x" .
_:t <https://dom.spec.whatwg.org/#dom-node-textcontent> "x" .
"#;

const SECOND_UNIT: &str = r#"
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/ns/root> .
_:item <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/item> .
_:item <http://sparql.xyz/facade-x/data/id> "2" .
"#;

/// Each unit keeps its type triple, its slot and its attributes; a child holding text is left out.
const SKELETON: &str = r#"
_:set <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/ns/root> .
_:set <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/set> .
_:set <http://sparql.xyz/facade-x/data/n> "2" .
_:set <http://www.w3.org/1999/02/22-rdf-syntax-ns#_1> _:first .
_:set <http://www.w3.org/1999/02/22-rdf-syntax-ns#_2> _:second .
_:first <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/item> .
_:first <http://sparql.xyz/facade-x/data/id> "1" .
_:second <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://sparql.xyz/facade-x/data/item> .
_:second <http://sparql.xyz/facade-x/data/id> "2" .
"#;

#[test]
fn lifts_each_unit_with_the_unit_as_root_and_keeps_only_its_attributes_and_childless_children_in_the_skeleton(
) {
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
fn refuses_a_document_ending_with_an_element_still_open_before_its_skeleton_is_read() {
    let refused = lift_slice(b"<set><item/>", Some("item"))
        .and_then(|lift| lift.into_skeleton())
        .err()
        .expect("refused");
    assert!(
        refused.message().contains("still open"),
        "{}",
        refused.message()
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
        unit.text,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><item>before<!-- inside --><?say it?>after<e></e></item>"
    );
}
