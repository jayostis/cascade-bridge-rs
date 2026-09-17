// The lift vectors of the Cascade Bridge Specification, judged by the
// bridge:LiftTest and bridge:SkeletonTest rules its vocabulary states.
//
// The files in tests/lift/ are copies of fixtures/lift/ in
// jayostis/cascade-bridge-spec, which is the authority: a disagreement is this
// Bridge's to fix, and a change there is copied here rather than argued with.
use cascade_bridge::lift_slice;
use oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use oxrdf::{Dataset, Quad};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;
use std::path::PathBuf;

const VECTORS: [&str; 7] = [
    "attributes",
    "mixed-content",
    "whitespace",
    "cdata",
    "dropped",
    "namespaces",
    "no-break-space",
];

/// Each skeleton vector with the bridge:elementNameOfEachRecord its manifest
/// entry names.
const SKELETON_VECTORS: [(&str, &str); 2] =
    [("skeleton", "Unit"), ("skeleton-record-root", "Unit")];

fn vector(name: &str, extension: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/lift")
        .join(format!("{name}.{extension}"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn canonical(quads: Vec<Quad>) -> BTreeSet<String> {
    let mut dataset = Dataset::new();
    for quad in &quads {
        dataset.insert(quad);
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
        hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
    });
    dataset.iter().map(|q| q.to_string()).collect()
}

/// Without a unit the whole document is the skeleton, which is the lift of the
/// document with its document element as the lift root.
fn lift_whole(xml: &[u8]) -> BTreeSet<String> {
    let store = lift_slice(xml, None)
        .expect("lift")
        .into_skeleton()
        .expect("skeleton");
    canonical(store.iter().map(|q| q.expect("quad")).collect())
}

fn expected(name: &str) -> BTreeSet<String> {
    canonical(
        RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(&vector(name, "nt"))
            .map(|q| q.expect("expected N-Triples"))
            .collect(),
    )
}

fn assert_vector(name: &str, produced: BTreeSet<String>) {
    let expected = expected(name);
    assert_eq!(
        produced,
        expected,
        "the {name} vector: missing {:?}, extra {:?}",
        expected.difference(&produced).collect::<Vec<_>>(),
        produced.difference(&expected).collect::<Vec<_>>()
    );
}

#[test]
fn reproduces_every_lift_vector_of_the_specification() {
    for name in VECTORS {
        assert_vector(name, lift_whole(&vector(name, "xml")));
    }
}

#[test]
fn reproduces_every_skeleton_vector_of_the_specification() {
    for (name, record) in SKELETON_VECTORS {
        let store = lift_slice(&vector(name, "xml"), Some(record))
            .expect("lift")
            .into_skeleton()
            .expect("skeleton");
        assert_vector(
            name,
            canonical(store.iter().map(|q| q.expect("quad")).collect()),
        );
    }
}

#[test]
fn lifts_each_unit_with_the_unit_as_root_and_empties_it_in_the_skeleton() {
    let xml = br#"<set n="2"><item id="1"><t>x</t></item><item id="2"/></set>"#;
    let mut lift = lift_slice(xml, Some("item")).expect("lift");
    let mut units = Vec::new();
    while let Some(store) = lift.next_unit().expect("unit") {
        units.push(store);
    }
    assert_eq!(units.len(), 2);

    let first = canonical(units[0].iter().map(|q| q.expect("quad")).collect());
    assert!(first.iter().any(|line| line.contains("facade-x/ns/root")));
    assert!(first.iter().any(|line| line.contains("facade-x/data/item")));
    assert!(!first.iter().any(|line| line.contains("facade-x/data/set")));

    let skeleton = lift.into_skeleton().expect("skeleton");
    let lines = canonical(skeleton.iter().map(|q| q.expect("quad")).collect());
    // Each unit keeps its type triple and its slot, and nothing else: two
    // types, two slots, and the set's own root type, name and attribute.
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.contains("facade-x/data/item"))
            .count(),
        2
    );
    assert!(!lines.iter().any(|line| line.contains("facade-x/data/t")));
    assert!(lines.iter().any(|line| line.contains("facade-x/data/n>")));
}

#[test]
fn treats_a_document_whose_element_is_the_unit_as_one_unit() {
    let mut lift = lift_slice(br#"<item id="9"/>"#, Some("item")).expect("lift");
    let unit = lift.next_unit().expect("unit").expect("one unit");
    let lines = canonical(unit.iter().map(|q| q.expect("quad")).collect());
    assert!(lines.iter().any(|line| line.contains("\"9\"")));
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
