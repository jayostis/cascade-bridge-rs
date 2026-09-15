// An XML document says what its bytes mean. Two documents that say the same
// thing in different encodings are the same document, and must lift to the
// same graph; decoding as UTF-8 whatever the bytes say turns one of them into
// mojibake, or into nothing at all.
use cascade_bridge::lift_slice;
use oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use oxrdf::Dataset;
use std::collections::BTreeSet;

const DOCUMENT: &str =
    r#"<?xml version="1.0" encoding="UTF-16"?><note lang="en">café — three children</note>"#;

fn lift(bytes: &[u8]) -> BTreeSet<String> {
    let store = lift_slice(bytes, None)
        .expect("lift")
        .into_skeleton()
        .expect("skeleton");
    let mut dataset = Dataset::new();
    for quad in store.iter() {
        dataset.insert(&quad.expect("quad"));
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
        hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
    });
    dataset.iter().map(|q| q.to_string()).collect()
}

fn utf16le_with_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

fn utf16be_with_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    bytes
}

#[test]
fn a_utf16_document_lifts_to_the_graph_of_its_utf8_twin() {
    let utf8 = DOCUMENT.replace("UTF-16", "UTF-8");
    let expected = lift(utf8.as_bytes());
    assert!(expected
        .iter()
        .any(|line| line.contains("caf\\u00E9") || line.contains("café")));

    assert_eq!(lift(&utf16le_with_bom(DOCUMENT)), expected);
    assert_eq!(lift(&utf16be_with_bom(DOCUMENT)), expected);
}

#[test]
fn a_utf8_byte_order_mark_is_not_a_character_of_the_document() {
    let utf8 = DOCUMENT.replace("UTF-16", "UTF-8");
    let mut with_mark = vec![0xEF, 0xBB, 0xBF];
    with_mark.extend_from_slice(utf8.as_bytes());
    assert_eq!(lift(&with_mark), lift(utf8.as_bytes()));
}
