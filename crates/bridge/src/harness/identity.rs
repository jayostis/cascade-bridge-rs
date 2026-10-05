use crate::fixtures::{tiny, versioned, Variant};
use crate::harness::{run_manifest, EntryResult, RunOptions};
use crate::load::load_adapter;

const MANIFEST: &str = "fixtures/manifest.ttl";

fn conversion(input: &str, selector: Option<&str>) -> String {
    let at = selector.map_or_else(String::new, |at| format!(" ; bridge:selector \"{at}\""));
    format!("[ bridge:input <in/{input}> ; bridge:facts <facts/catalog.ttl>{at} ]")
}

fn relation(one: &str, two: &str, same: bool) -> String {
    format!(
        "@prefix mf:     <http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#> .
@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#> .
<> a mf:Manifest ; bridge:adapter <../> ; mf:entries ( <#relation> ) .
<#relation> a bridge:IdentityRelationTest ;
  mf:name \"relation\" ;
  mf:action [ bridge:conversion {one}, {two} ] ;
  mf:result [ bridge:sameRecord {same} ] ."
    )
}

fn judged(adapter: Variant, manifest: String) -> EntryResult {
    let resolver = adapter.with(MANIFEST, manifest);
    let adapter = load_adapter(&resolver).expect("adapter");
    let mut results = run_manifest(&adapter, &resolver, RunOptions::default()).expect("run");
    assert_eq!(results.len(), 1);
    results.remove(0)
}

fn judged_as(adapter: Variant, manifest: String) -> (String, String) {
    let result = judged(adapter, manifest);
    (result.outcome.as_str().to_owned(), result.description)
}

#[test]
fn passes_where_two_documents_name_one_record_and_the_result_says_they_are_one() {
    let (outcome, said) = judged_as(
        versioned(),
        relation(
            &conversion("two.xml", Some("/catalog/item[1]")),
            &conversion("output-fails-a-shape.xml", None),
            true,
        ),
    );
    assert_eq!(outcome, "passed", "{said}");
    assert!(said.contains("urn:example:item:1"), "{said}");
}

#[test]
fn passes_where_two_records_of_one_document_are_two_and_the_result_says_so() {
    let (outcome, said) = judged_as(
        versioned(),
        relation(
            &conversion("two.xml", Some("/catalog/item[1]")),
            &conversion("two.xml", Some("/catalog/item[2]")),
            false,
        ),
    );
    assert_eq!(outcome, "passed", "{said}");
}

#[test]
fn fails_where_the_names_break_the_relation_the_result_states() {
    let (outcome, said) = judged_as(
        versioned(),
        relation(
            &conversion("two.xml", Some("/catalog/item[1]")),
            &conversion("two.xml", Some("/catalog/item[2]")),
            true,
        ),
    );
    assert_eq!(outcome, "failed", "{said}");
    assert!(said.contains("must be one record"), "{said}");
}

#[test]
fn fails_a_conversion_that_names_other_than_one_record() {
    let (outcome, said) = judged_as(
        versioned(),
        relation(
            &conversion("two.xml", None),
            &conversion("output-fails-a-shape.xml", None),
            true,
        ),
    );
    assert_eq!(outcome, "failed", "{said}");
    assert!(said.contains("names 2 record(s)"), "{said}");

    let (outcome, said) = judged_as(
        Variant::of(tiny()),
        relation(
            &conversion("output-fails-a-shape.xml", None),
            &conversion("output-fails-a-shape.xml", None),
            true,
        ),
    );
    assert_eq!(
        outcome, "failed",
        "a record no version arrived for is named by nothing"
    );
    assert!(said.contains("names 0 record(s)"), "{said}");
}
