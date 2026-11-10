use super::lift;
use crate::lift::{admitting, Admission, Lift, Paths, Reading, Unit};
use std::collections::HashSet;

fn envelope(root: &str, value: Option<&str>, records: &str) -> Admission {
    Admission {
        root: Some(root.to_owned()),
        value: value.map(str::to_owned),
        records: Some(records.to_owned()),
    }
}

fn records(text: &str, reading: &Reading<'_>, paths: Paths) -> Vec<Unit> {
    let mut lift = lift(text, reading, paths).expect("the lift");
    let mut records = Vec::new();
    while let Some(record) = Lift::next_unit(&mut lift).expect("a record") {
        records.push(record);
    }
    records
}

/// Reading a JSON document as records · Loaded::convert(), Document::envelope
mod reading_a_json_document_as_records {
    use super::*;

    /// given JSON text
    mod given_json_text {
        use super::*;

        #[test]
        fn refuses_a_document_whose_value_is_neither_an_object_nor_an_array() {
            let reading = Reading {
                element: None,
                breaking: &[],
                envelopes: Vec::new(),
                named: None,
            };
            for text in ["\"text\"", "1", "null"] {
                assert!(lift(text, &reading, Paths::Dropped).is_err(), "{text}");
            }
        }
    }

    /// given an envelope's record path
    mod given_an_envelope_s_record_path {
        use super::*;

        /// counts an array's items at its path and addresses the first
        #[test]
        fn counts_an_array_s_items_at_its_path_and_addresses_the_first() {
            let whole = envelope("id", None, "$");
            let reading = Reading {
                element: None,
                breaking: &[],
                envelopes: vec![&whole],
                named: None,
            };
            let paths = Paths::Kept {
                valued: HashSet::from(["/status".to_owned()]),
            };
            let text = r#"{"id": "r", "status": ["a", "B", "B"], "ext": [{"u": 1}, {"u": 2}], "gone": null, "a/b": []}"#;
            let records = records(text, &reading, paths);
            let occurrences: Vec<(&str, Option<&str>, usize)> = records[0]
                .occurrences()
                .iter()
                .map(|o| (o.path.as_str(), o.within.as_deref(), o.count))
                .collect();
            assert_eq!(
                occurrences,
                [
                    ("/id", Some("/id"), 1),
                    ("/status", Some("/status/0"), 3),
                    ("/ext", Some("/ext/0"), 2),
                    ("/ext/u", Some("/ext/0/u"), 2),
                ]
            );
            let values: Vec<(&str, Option<&str>, usize)> = records[0]
                .values()
                .iter()
                .map(|v| (v.value.as_str(), v.within.as_deref(), v.count))
                .collect();
            assert_eq!(
                values,
                [("a", Some("/status/0"), 1), ("B", Some("/status/1"), 2)]
            );
        }
    }

    mod given_more_than_one_envelope {
        use super::*;

        /// reads a document in an envelope naming its member's value before one naming none
        #[test]
        fn reads_a_document_in_an_envelope_naming_its_member_s_value_before_one_naming_none() {
            let any = envelope("kind", None, "$");
            let set = envelope("kind", Some("set"), "$.records[*]");
            let other = envelope("kind", Some("other"), "$.other[*]");
            let reading = Reading {
                element: None,
                breaking: &[],
                envelopes: vec![&any, &other, &set],
                named: None,
            };
            let text = r#"{"kind": "set", "records": [{"a": "1"}, {"a": "2"}]}"#;
            let lift = lift(text, &reading, Paths::Dropped).expect("the lift");
            assert_eq!(admitting(&lift, &reading), Some(2));
            let selectors: Vec<String> = records(text, &reading, Paths::Dropped)
                .iter()
                .map(Unit::selector)
                .collect();
            assert_eq!(selectors, ["/records/0", "/records/1"]);
        }

        #[test]
        fn reads_a_document_in_the_envelope_named_whether_or_not_it_admits_it() {
            let set = envelope("kind", Some("set"), "$.records[*]");
            let whole = envelope("absent", None, "$");
            let reading = Reading {
                element: None,
                breaking: &[],
                envelopes: vec![&set, &whole],
                named: Some(1),
            };
            let text = r#"{"kind": "set", "records": [{"a": "1"}]}"#;
            let records = records(text, &reading, Paths::Dropped);
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].selector(), "");
            assert_eq!(records[0].text, text);
        }

        #[test]
        fn splits_nothing_where_no_envelope_admits_the_document() {
            let set = envelope("kind", Some("set"), "$.records[*]");
            let reading = Reading {
                element: None,
                breaking: &[],
                envelopes: vec![&set],
                named: None,
            };
            assert!(records(
                r#"{"kind": "list", "records": [{}]}"#,
                &reading,
                Paths::Dropped
            )
            .is_empty());
        }
    }
}
