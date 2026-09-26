use crate::fixtures::{tiny_with_vocabularies, Variant};
use crate::load::load_adapter;
use crate::run::prepare;

const ENDPOINT: &str = "<https://example.invalid/sparql>";
const GRAPH: &str = "<https://example.invalid/g>";

/// Empty where the adapter prepares.
fn refusal(query: &str, from: &str, to: impl Into<String>) -> String {
    let resolver = Variant::of(tiny_with_vocabularies()).replacing(query, from, to);
    let adapter = load_adapter(&resolver).expect("adapter");
    prepare(&adapter, &resolver)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default()
}

fn assert_holds(refused: &str, query: &str, phrase: &str) {
    let said = format!("{query} holds {phrase};");
    assert!(
        refused.contains(&said),
        "{query} was prepared, or refused without saying {said:?}: {refused:?}"
    );
    assert!(
        !refused.contains('\n'),
        "a refusal is one line: {refused:?}"
    );
}

fn service_in_the_mapping(pattern: String) -> String {
    refusal(
        "mapping/item.rq",
        "WHERE {\n",
        format!("WHERE {{\n  {pattern}\n"),
    )
}

fn fetch() -> String {
    format!("SERVICE {ENDPOINT} {{ ?there ?far ?away }}")
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern() {
    assert_holds(
        &service_in_the_mapping(fetch()),
        "item.rq",
        "a SERVICE pattern",
    );
}

#[test]
fn refuses_a_findings_query_holding_a_service_pattern_inside_filter_exists() {
    let refused = refusal(
        "mapping/item-findings.rq",
        "WHERE {\n",
        format!("WHERE {{\n  FILTER EXISTS {{ {} }}\n", fetch()),
    );
    assert_holds(&refused, "item-findings.rq", "a SERVICE pattern");
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern_inside_a_subquery() {
    let pattern = format!("{{ SELECT * WHERE {{ {} }} }}", fetch());
    assert_holds(
        &service_in_the_mapping(pattern),
        "item.rq",
        "a SERVICE pattern",
    );
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern_inside_optional() {
    let pattern = format!("OPTIONAL {{ {} }}", fetch());
    assert_holds(
        &service_in_the_mapping(pattern),
        "item.rq",
        "a SERVICE pattern",
    );
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern_inside_union() {
    let pattern = format!("{{ ?item ?near ?by }} UNION {{ {} }}", fetch());
    assert_holds(
        &service_in_the_mapping(pattern),
        "item.rq",
        "a SERVICE pattern",
    );
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern_inside_minus() {
    let pattern = format!("MINUS {{ {} }}", fetch());
    assert_holds(
        &service_in_the_mapping(pattern),
        "item.rq",
        "a SERVICE pattern",
    );
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern_inside_graph() {
    let pattern = format!("GRAPH ?g {{ {} }}", fetch());
    assert_holds(
        &service_in_the_mapping(pattern),
        "item.rq",
        "a SERVICE pattern",
    );
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern_inside_a_bind_of_exists() {
    let pattern = format!("BIND(EXISTS {{ {} }} AS ?reached)", fetch());
    assert_holds(
        &service_in_the_mapping(pattern),
        "item.rq",
        "a SERVICE pattern",
    );
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern_inside_having() {
    let pattern = format!(
        "{{ SELECT ?item WHERE {{ ?item ?near ?by }} GROUP BY ?item HAVING (EXISTS {{ {} }}) }}",
        fetch()
    );
    assert_holds(
        &service_in_the_mapping(pattern),
        "item.rq",
        "a SERVICE pattern",
    );
}

#[test]
fn refuses_a_mapping_holding_a_service_pattern_inside_order_by() {
    let pattern = format!(
        "{{ SELECT ?item WHERE {{ ?item ?near ?by }} ORDER BY (EXISTS {{ {} }}) }}",
        fetch()
    );
    assert_holds(
        &service_in_the_mapping(pattern),
        "item.rq",
        "a SERVICE pattern",
    );
}

#[test]
fn refuses_a_detect_query_holding_a_service_pattern() {
    let refused = refusal("mapping/detect.rq", "ASK {", format!("ASK {{ {}", fetch()));
    assert_holds(&refused, "detect.rq", "a SERVICE pattern");
}

#[test]
fn refuses_a_mapping_holding_a_from_clause() {
    let refused = refusal(
        "mapping/item.rq",
        "WHERE {",
        format!("FROM {GRAPH}\nWHERE {{"),
    );
    assert_holds(&refused, "item.rq", "a FROM clause");
}

#[test]
fn refuses_a_mapping_holding_a_from_named_clause() {
    let refused = refusal(
        "mapping/item.rq",
        "WHERE {",
        format!("FROM NAMED {GRAPH}\nWHERE {{"),
    );
    assert_holds(&refused, "item.rq", "a FROM NAMED clause");
}

#[test]
fn refuses_a_detect_query_holding_a_from_clause() {
    let refused = refusal("mapping/detect.rq", "ASK {", format!("ASK FROM {GRAPH} {{"));
    assert_holds(&refused, "detect.rq", "a FROM clause");
}

#[test]
fn prepares_a_mapping_naming_service_and_from_only_in_a_comment() {
    let refused = refusal(
        "mapping/item.rq",
        "WHERE {\n",
        format!("# FROM {GRAPH}\nWHERE {{\n  # {}\n", fetch()),
    );
    assert_eq!(refused, "");
}

#[test]
fn prepares_a_mapping_naming_service_and_from_only_in_a_string_literal() {
    let refused = refusal(
        "mapping/item.rq",
        "WHERE {\n",
        "WHERE {\n  BIND(\"SERVICE <https://example.invalid/sparql>\" AS ?said)\n  BIND(\"FROM\" AS ?word)\n",
    );
    assert_eq!(refused, "");
}
