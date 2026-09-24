mod common;

use cascade_bridge::{load_adapter, prepare};
use common::{tiny_with_vocabularies, Variant};

const ENDPOINT: &str = "<https://example.invalid/sparql>";

/// What preparing the tiny adapter says with this query rewritten, which is
/// nothing where it prepares.
fn refusal(query: &str, from: &str, to: String) -> String {
    let resolver = Variant::of(tiny_with_vocabularies()).replacing(query, from, to);
    let adapter = load_adapter(&resolver).expect("adapter");
    prepare(&adapter, &resolver)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default()
}

fn assert_names(refused: &str, query: &str, clause: &str) {
    assert!(
        refused.contains(query) && refused.contains(clause),
        "{query} holding {clause} was prepared, or refused without naming both: {refused:?}"
    );
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern() {
    let refused = refusal(
        "mapping/item.rq",
        "WHERE {\n",
        format!("WHERE {{\n  SERVICE {ENDPOINT} {{ ?there ?p ?o }}\n"),
    );
    assert_names(&refused, "item.rq", "SERVICE");
}

#[test]
fn refuses_a_findings_query_holding_a_service_pattern_inside_filter_exists() {
    let refused = refusal(
        "mapping/item-findings.rq",
        "WHERE {\n",
        format!("WHERE {{\n  FILTER EXISTS {{ SERVICE {ENDPOINT} {{ ?there ?p ?o }} }}\n"),
    );
    assert_names(&refused, "item-findings.rq", "SERVICE");
}

#[test]
fn refuses_a_mapping_holding_a_from_clause() {
    let refused = refusal(
        "mapping/item.rq",
        "WHERE {",
        "FROM <https://example.invalid/g>\nWHERE {".to_owned(),
    );
    assert_names(&refused, "item.rq", "FROM");
}

#[test]
fn refuses_a_mapping_holding_a_from_named_clause() {
    let refused = refusal(
        "mapping/item.rq",
        "WHERE {",
        "FROM NAMED <https://example.invalid/g>\nWHERE {".to_owned(),
    );
    assert_names(&refused, "item.rq", "FROM NAMED");
}
