use super::lift_slice;

fn selectors(xml: &[u8], record: &str) -> Vec<String> {
    lift_slice(xml, Some(record))
        .expect("lift")
        .map(|unit| unit.expect("unit").selector())
        .collect()
}

#[test]
fn writes_a_step_for_every_element_down_to_a_record_below_the_document_element() {
    assert_eq!(
        selectors(b"<set><group><item/><item/></group></set>", "item"),
        ["/set/group[1]/item[1]", "/set/group[1]/item[2]"]
    );
}

#[test]
fn numbers_a_record_among_its_own_parent_s_children_of_that_name() {
    assert_eq!(
        selectors(
            b"<set><group><item/></group><group><item/></group></set>",
            "item"
        ),
        ["/set/group[1]/item[1]", "/set/group[2]/item[1]"]
    );
}

#[test]
fn counts_a_sibling_of_another_name_towards_neither() {
    assert_eq!(
        selectors(
            b"<set><other/><group/><other/><group><item/></group></set>",
            "item"
        ),
        ["/set/group[2]/item[1]"]
    );
}

#[test]
fn names_an_element_in_a_namespace_where_no_prefix_can_be_bound() {
    assert_eq!(
        selectors(
            br#"<s:set xmlns:s="urn:example:set"><s:item/><s:item/></s:set>"#,
            "item"
        ),
        [
            "/*[local-name()='set' and namespace-uri()='urn:example:set']\
             /*[local-name()='item' and namespace-uri()='urn:example:set'][1]",
            "/*[local-name()='set' and namespace-uri()='urn:example:set']\
             /*[local-name()='item' and namespace-uri()='urn:example:set'][2]"
        ]
    );
}

#[test]
fn names_an_element_in_no_namespace_under_one_in_a_namespace_by_its_name() {
    assert_eq!(
        selectors(
            br#"<set xmlns="urn:example:set"><item xmlns=""/></set>"#,
            "item"
        ),
        ["/*[local-name()='set' and namespace-uri()='urn:example:set']/item[1]"]
    );
}

#[test]
fn writes_a_record_that_is_the_document_element_as_one_step_with_no_index() {
    assert_eq!(selectors(br#"<item id="9"/>"#, "item"), ["/item"]);
}
