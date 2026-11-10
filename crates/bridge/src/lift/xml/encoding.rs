use super::lift_slice;
use crate::rdf::canonical_lines;
use oxrdf::{Quad, Term};
use std::collections::BTreeSet;

const DOCUMENT: &str =
    r#"<?xml version="1.0" encoding="UTF-16"?><note lang="en">café — three children</note>"#;

fn skeleton(bytes: &[u8]) -> Vec<Quad> {
    lift_slice(bytes, None)
        .expect("lift")
        .into_skeleton()
        .expect("skeleton")
        .iter()
        .map(|quad| quad.expect("quad"))
        .collect()
}

fn lift(bytes: &[u8]) -> BTreeSet<String> {
    canonical_lines(skeleton(bytes)).expect("canonical")
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

mod tests {
    use super::*;

    /// Reading an XML document as records · Loaded::convert()
    mod reading_an_xml_document_as_records {
        use super::*;

        /// given a document's encoding
        mod given_a_document_s_encoding {
            use super::*;

            /// lifts a UTF-16 document to the graph of its UTF-8 twin
            #[test]
            fn lifts_a_utf_16_document_to_the_graph_of_its_utf_8_twin() {
                let utf8 = DOCUMENT.replace("UTF-16", "UTF-8");
                let expected = lift(utf8.as_bytes());
                let text: Vec<String> = skeleton(utf8.as_bytes())
                    .into_iter()
                    .filter_map(|quad| match quad.object {
                        Term::Literal(literal) => Some(literal.value().to_owned()),
                        _ => None,
                    })
                    .collect();
                assert!(
                    text.iter().any(|value| value == "café — three children"),
                    "{text:?}"
                );

                assert_eq!(lift(&utf16le_with_bom(DOCUMENT)), expected);
                assert_eq!(lift(&utf16be_with_bom(DOCUMENT)), expected);
            }

            /// reads a UTF-8 byte order mark as no character of the document
            #[test]
            fn reads_a_utf_8_byte_order_mark_as_no_character_of_the_document() {
                let utf8 = DOCUMENT.replace("UTF-16", "UTF-8");
                let mut with_mark = vec![0xEF, 0xBB, 0xBF];
                with_mark.extend_from_slice(utf8.as_bytes());
                assert_eq!(lift(&with_mark), lift(utf8.as_bytes()));
            }
        }
    }
}
