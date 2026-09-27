use super::{parse, path, pointer, records, selected, tokens, Segment, Value};

fn refusal(text: &str) -> String {
    parse(text).expect_err(text).to_string()
}

#[test]
fn keeps_every_member_of_a_repeated_name_in_document_order() {
    let node = parse(r#"{"a": 1, "b": 2, "a": 3}"#).expect("parsed");
    let names: Vec<String> = node.children().into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["a", "b", "a"]);
}

#[test]
fn keeps_a_number_as_the_document_writes_it() {
    let node = parse("[1.50, -0, 1E+2, 123456789012345678901234567890]").expect("parsed");
    let written: Vec<&str> = node
        .children()
        .into_iter()
        .filter_map(|(_, item)| item.scalar())
        .collect();
    assert_eq!(
        written,
        ["1.50", "-0", "1E+2", "123456789012345678901234567890"]
    );
}

#[test]
fn refuses_a_lone_surrogate_and_what_rfc_8259_does_not_allow() {
    assert!(refusal(r#"["\ud834"]"#).contains("lone surrogate"));
    assert!(refusal(r#"["\udd1e\ud834"]"#).contains("lone surrogate"));
    for text in [
        "[01]",
        "[1.]",
        "{'a': 1}",
        "[1,]",
        "[\"a\tb\"]",
        "[] []",
        "[NaN]",
    ] {
        refusal(text);
    }
}

#[test]
fn refuses_a_byte_order_mark() {
    assert!(super::decode(b"\xEF\xBB\xBF[]").is_err());
}

#[test]
fn records_where_each_value_stands_in_the_text() {
    let text = r#"{"records": [ {"a": 1} ]}"#;
    let node = parse(text).expect("parsed");
    let record = node.at(&[0, 0]).expect("the record");
    assert_eq!(&text[record.span.clone()], r#"{"a": 1}"#);
}

#[test]
fn writes_a_pointer_in_its_fragment_representation_and_reads_it_back() {
    let written: Vec<String> = ["a/b", "m~n", "c d", "é", "100%", "q?:@"]
        .iter()
        .map(|token| (*token).to_owned())
        .collect();
    let spelled = pointer(&written);
    assert_eq!(spelled, "/a~1b/m~0n/c%20d/%C3%A9/100%25/q?:@");
    assert_eq!(tokens(&spelled), Some(written));
    assert_eq!(tokens(""), Some(Vec::new()));
    assert_eq!(tokens("no-slash"), None);
    assert_eq!(tokens("/bad~2"), None);
}

#[test]
fn a_pointer_through_a_repeated_name_selects_each_node_and_one_to_null_the_null() {
    let node = parse(r#"{"a": {"b": 1}, "a": {"b": 2}, "n": null}"#).expect("parsed");
    assert_eq!(selected(&node, "/a/b").len(), 2);
    assert_eq!(selected(&node, "/n").len(), 1);
    assert_eq!(selected(&node, "/%FF").len(), 0);
    assert_eq!(selected(&node, "").len(), 1);
}

#[test]
fn reads_a_record_path_in_the_subset_and_refuses_one_outside_it() {
    assert_eq!(path("$").expect("root"), Vec::<Segment>::new());
    assert_eq!(
        path("$.entry[*]['full name']").expect("path"),
        [
            Segment::Name("entry".to_owned()),
            Segment::Wildcard,
            Segment::Name("full name".to_owned())
        ]
    );
    for outside in [
        "entry",
        "$..entry",
        "$.entry[0]",
        "$.1a",
        "$['a'",
        "$[?@.a]",
    ] {
        assert!(path(outside).is_err(), "{outside}");
    }
}

#[test]
fn selects_as_records_only_the_objects_a_path_reaches() {
    let node = parse(r#"{"entry": [{"a": 1}, "text", {"b": 2}, null]}"#).expect("parsed");
    let segments = path("$.entry[*]").expect("path");
    assert_eq!(records(&node, &segments), [vec![0, 0], vec![0, 2]]);
    assert_eq!(records(&node, &[]), [Vec::<usize>::new()]);
    assert!(matches!(
        node.at(&[0, 1]).map(|n| &n.value),
        Some(Value::String(_))
    ));
}

#[test]
fn reads_a_percent_escape_only_as_two_hex_digits() {
    assert_eq!(tokens("/%41"), Some(vec!["A".to_owned()]));
    assert_eq!(tokens("/%+41"), None);
    assert_eq!(tokens("/%-1"), None);
    assert_eq!(tokens("/%4"), None);
}
