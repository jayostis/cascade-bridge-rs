// tests/lift/ is a copy of fixtures/lift/ in jayostis/cascade-bridge-spec, the authority.
use super::{lift, lifted};
use crate::fixtures::{fixture, fixture_names};
use crate::lift::{Admission, Lift, Paths, Reading};
use crate::rdf::canonical_lines;
use oxrdf::Quad;
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;

/// Each skeleton vector, the files it reads and the record path its manifest entry names.
const SKELETON_VECTORS: [(&str, &str, &str); 3] = [
    ("skeleton-json", "skeleton-json", "$.entry[*]"),
    ("skeleton-json-record-root", "skeleton-json-record-root", "$"),
    (
        "skeleton-json-bracket-path",
        "skeleton-json",
        "$['entry'][*]",
    ),
];

const VECTORS: &str = "tests/lift";

fn vector(name: &str, extension: &str) -> Vec<u8> {
    fixture(&format!("{VECTORS}/{name}.{extension}"))
}

fn text(name: &str) -> String {
    String::from_utf8(vector(name, "json")).expect("utf-8")
}

fn canonical(quads: Vec<Quad>) -> BTreeSet<String> {
    canonical_lines(quads).expect("canonical")
}

fn expected(name: &str) -> BTreeSet<String> {
    canonical(
        RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(&vector(name, "nt"))
            .map(|q| q.expect("expected N-Triples"))
            .collect(),
    )
}

pub(super) fn skeleton(text: &str, path: &str) -> BTreeSet<String> {
    let admission = Admission {
        records: Some(path.to_owned()),
        ..Admission::default()
    };
    let reading = Reading {
        element: None,
        breaking: &[],
        envelopes: vec![&admission],
        named: Some(0),
    };
    let lift: Box<dyn Lift + '_> =
        Box::new(lift(text, &reading, Paths::Dropped).expect("the lift"));
    canonical(
        lift.into_skeleton()
            .expect("the skeleton")
            .iter()
            .map(|q| q.expect("quad"))
            .collect(),
    )
}

mod tests {
    use super::*;

    /// Reading a JSON document as records · Loaded::convert(), Document::envelope
    mod reading_a_json_document_as_records {
        use super::*;

        /// given the specification's JSON lift vectors
        mod given_the_specification_s_json_lift_vectors {
            use super::*;

            #[test]
            fn reproduces_every_json_lift_vector_of_the_specification() {
                let names: Vec<String> = fixture_names(VECTORS)
                    .into_iter()
                    .filter_map(|name| name.strip_suffix(".json").map(str::to_owned))
                    .filter(|name| {
                        !SKELETON_VECTORS
                            .iter()
                            .any(|(_, files, _)| files == name)
                    })
                    .collect();
                assert!(names.len() >= 7, "{names:?}");
                for name in names {
                    let produced = canonical(lifted(&text(&name)).expect("the lift"));
                    let expected = expected(&name);
                    assert_eq!(
                        produced,
                        expected,
                        "the {name} vector: missing {:?}, extra {:?}",
                        expected.difference(&produced).collect::<Vec<_>>(),
                        produced.difference(&expected).collect::<Vec<_>>()
                    );
                }
            }

            #[test]
            fn reproduces_every_json_skeleton_vector_of_the_specification() {
                for (name, files, path) in SKELETON_VECTORS {
                    assert_eq!(
                        skeleton(&text(files), path),
                        expected(files),
                        "the {name} vector"
                    );
                }
            }
        }
    }
}
