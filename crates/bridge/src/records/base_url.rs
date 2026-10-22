use super::normalised_base_url;
use crate::error::ErrorKind;
use crate::fixtures::fixture;

fn unescaped(url: &str) -> String {
    let mut out = String::new();
    let mut rest = url;
    while let Some(character) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("\\x") {
            let code = u8::from_str_radix(&after[..2], 16).expect("two hex digits");
            out.push(char::from(code));
            rest = &after[2..];
        } else if let Some(after) = rest.strip_prefix("\\\\") {
            out.push('\\');
            rest = after;
        } else {
            out.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    out
}

#[test]
fn normalises_or_refuses_every_edge_case_of_the_list() {
    let list = String::from_utf8(fixture("tests/base-url-edge-cases.txt")).expect("UTF-8");
    let mut checked = 0;
    for line in list
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let (given, expected) = line.split_once('\t').expect(line);
        let given = unescaped(given);
        let produced = normalised_base_url(&given);
        if expected == "REFUSED" {
            let refusal = produced.expect_err(line);
            assert_eq!(refusal.kind(), &ErrorKind::Facts, "{line}");
        } else {
            assert_eq!(produced.expect(line), expected, "{line}");
        }
        checked += 1;
    }
    assert!(checked >= 70, "{checked}");
}

#[test]
fn says_which_rule_refused_a_base_url_and_never_the_url() {
    for url in [
        "https://secret@h.example/r4",
        "https://h.example/r4?token=secret",
        "https://h.example/r4#secret",
        "https://h.example:secret/r4",
        "https://h\u{e9}secret.example/r4",
        "secret-no-scheme",
        "https://h.example/secret space",
        "https://[secret/r4",
        "https://secret%zz.example/r4",
        "https://[secret]/r4",
        "https://h.example:99999999secret/r4",
    ] {
        let said = normalised_base_url(url).expect_err(url).to_string();
        assert!(!said.contains("secret"), "{said}");
    }
}
