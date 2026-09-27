use super::compile;
use crate::error::{Error, Result};
use crate::resolver::Resolver;
use crate::validate::Schema;
use std::collections::HashMap;

const ROOT: &str = "file:///adapter/";
const SECTION: &str =
    "https://datatracker.ietf.org/doc/html/draft-wright-json-schema-validation-01#section-6.";

struct Files(HashMap<String, String>);

impl Resolver for Files {
    fn root(&self) -> &str {
        ROOT
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        self.0
            .get(iri)
            .map(|text| text.clone().into_bytes())
            .ok_or_else(|| Error::msg(format!("{iri}: not in the package")))
    }
}

fn files(schemas: &[(&str, &str)]) -> Files {
    Files(
        schemas
            .iter()
            .map(|(name, text)| (format!("{ROOT}{name}"), (*text).to_owned()))
            .collect(),
    )
}

/// Each finding as its keyword's section, or its body, and where it is refined to.
fn findings(schema: &str, instance: &str) -> Vec<(String, Option<String>)> {
    let files = files(&[("schema.json", schema)]);
    let compiled = compile(&format!("{ROOT}schema.json"), &files).expect("the schema");
    compiled
        .errors(instance)
        .expect("validated")
        .iter()
        .map(|finding| {
            (
                finding.body().replace(SECTION, "6."),
                finding.within().map(str::to_owned),
            )
        })
        .collect()
}

fn found(pairs: &[(&str, Option<&str>)]) -> Vec<(String, Option<String>)> {
    pairs
        .iter()
        .map(|(body, within)| ((*body).to_owned(), within.map(str::to_owned)))
        .collect()
}

#[test]
fn reports_each_failing_keyword_by_its_draft_06_section_at_the_node_it_fails_at() {
    let schema = r##"{
        "$schema": "http://json-schema.org/draft-06/schema#",
        "type": "object",
        "required": ["accession"],
        "properties": {
            "version": {"$ref": "#/definitions/positive"},
            "label": {"type": "string"},
            "tags": {"type": "array", "items": {"pattern": "^[a-z]+$"}, "minItems": 2}
        },
        "definitions": {"positive": {"type": "integer", "minimum": 1}}
    }"##;
    assert_eq!(
        findings(
            schema,
            r#"{"version": 0, "label": 5, "tags": ["ok", "Not"]}"#
        ),
        found(&[
            ("6.17", None),
            ("6.4", Some("/version")),
            ("6.25", Some("/label")),
            ("6.8", Some("/tags/1")),
        ])
    );
}

#[test]
fn reports_an_applicator_whose_subschema_is_false_where_the_applicator_stands() {
    let schema = r#"{"properties": {"inner": {"properties": {"a": {}}, "additionalProperties": false}}, "additionalProperties": false}"#;
    assert_eq!(
        findings(schema, r#"{"inner": {"a": 1, "b": 2}, "extra": 3}"#),
        found(&[("6.20", Some("/inner")), ("6.20", None)])
    );
}

#[test]
fn reports_one_of_any_of_not_and_contains_as_themselves() {
    let schema = r#"{"properties": {
        "one": {"oneOf": [{"type": "string"}, {"minLength": 1}]},
        "any": {"anyOf": [{"type": "integer"}, {"type": "boolean"}]},
        "not": {"not": {"type": "null"}},
        "has": {"contains": {"const": "x"}}
    }}"#;
    assert_eq!(
        findings(
            schema,
            r#"{"one": "ab", "any": "s", "not": null, "has": ["y"]}"#
        ),
        found(&[
            ("6.28", Some("/one")),
            ("6.27", Some("/any")),
            ("6.29", Some("/not")),
            ("6.14", Some("/has")),
        ])
    );
}

#[test]
fn compares_numbers_by_value_and_counts_one_point_zero_an_integer() {
    let schema = r#"{"items": {"type": "integer", "enum": [1, 20]}, "uniqueItems": true}"#;
    assert_eq!(findings(schema, "[1.0, 2E1]"), found(&[]));
    assert_eq!(findings(schema, "[1, 1.0]"), found(&[("6.13", None)]));
    assert_eq!(
        findings(schema, "[1.5]"),
        found(&[("6.25", Some("/0")), ("6.23", Some("/0"))])
    );
}

#[test]
fn asserts_no_format() {
    assert_eq!(
        findings(r#"{"format": "date-time"}"#, r#""not a date""#),
        found(&[])
    );
}

#[test]
fn follows_a_ref_to_another_file_of_the_package_and_refuses_one_outside_it() {
    let package = files(&[
        ("a.json", r##"{"$ref": "b.json#/definitions/text"}"##),
        ("b.json", r#"{"definitions": {"text": {"type": "string"}}}"#),
    ]);
    let compiled = compile(&format!("{ROOT}a.json"), &package).expect("the schema");
    assert_eq!(compiled.errors("1").expect("validated").len(), 1);
    let outside = files(&[("a.json", r#"{"$ref": "https://example.org/b.json"}"#)]);
    assert!(compile(&format!("{ROOT}a.json"), &outside).is_err());
}

#[test]
fn refuses_a_schema_written_in_another_draft() {
    let package = files(&[(
        "a.json",
        r#"{"$schema": "http://json-schema.org/draft-07/schema#"}"#,
    )]);
    let refusal = compile(&format!("{ROOT}a.json"), &package)
        .err()
        .expect("refused")
        .to_string();
    assert!(refusal.contains("draft-06"), "{refusal}");
}

#[test]
fn resolves_a_ref_against_the_id_of_the_subschema_it_stands_in() {
    let package = files(&[
        (
            "schema.json",
            r##"{
                "definitions": {
                    "a": {"$id": "sub/a.json", "properties": {"x": {"$ref": "b.json"}}},
                    "named": {"$id": "#addr", "type": "string"}
                },
                "properties": {
                    "a": {"$ref": "sub/a.json"},
                    "n": {"$ref": "#addr"},
                    "p": {"$ref": "#/definitions/a/properties/x"}
                }
            }"##,
        ),
        ("sub/b.json", r#"{"type": "integer"}"#),
    ]);
    let compiled = compile(&format!("{ROOT}schema.json"), &package).expect("the schema");
    let within: Vec<Option<String>> = compiled
        .errors(r#"{"a": {"x": "s"}, "n": 1, "p": "s"}"#)
        .expect("validated")
        .iter()
        .map(|finding| finding.within().map(str::to_owned))
        .collect();
    assert_eq!(
        within,
        [
            Some("/a/x".to_owned()),
            Some("/n".to_owned()),
            Some("/p".to_owned())
        ]
    );
}

#[test]
fn reads_a_pattern_as_ecma_262_does() {
    let schema = r#"{
        "properties": {"digits": {"pattern": "^\\d+$"}, "ahead": {"pattern": "^(?=a)\\w$"}},
        "patternProperties": {"^\\w$": false}
    }"#;
    assert_eq!(
        findings(schema, r#"{"digits": "١٢٣", "ahead": "a", "é": 1}"#),
        found(&[("6.8", Some("/digits"))])
    );
}

#[test]
fn compiles_the_pattern_keyword_alone_and_no_pattern_member_of_a_value() {
    let schema = r#"{
        "enum": [{"pattern": "("}],
        "const": {"pattern": "(", "patternProperties": {"(": 1}},
        "default": {"pattern": "("},
        "examples": [{"pattern": "("}],
        "properties": {"enum": {"pattern": "^a$"}, "pattern": {"enum": [{"pattern": "("}]}}
    }"#;
    assert_eq!(
        findings(schema, r#"{"enum": "b"}"#),
        found(&[("6.23", None), ("6.24", None), ("6.8", Some("/enum"))])
    );
}
