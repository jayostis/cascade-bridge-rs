use crate::decode::XML_SPACE;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

pub(crate) const TEXT_CONTENT: &str = "https://dom.spec.whatwg.org/#dom-node-textcontent";
pub(crate) const TEXT_DECODED_FROM_BASE64: &str =
    "https://ns.cascadeprotocol.org/bridge/v1-draft#textDecodedFromBase64";
pub(crate) const TEXT_WITH_ELEMENT_BREAKS: &str =
    "https://ns.cascadeprotocol.org/bridge/v1-draft#textWithElementBreaks";

fn is_utf_8(charset: &str) -> bool {
    let value = charset.trim_matches(XML_SPACE);
    let unquoted = match value.strip_prefix('"') {
        Some(inside) => match inside.strip_suffix('"') {
            Some(unquoted) => unquoted,
            None => return false,
        },
        None => value,
    };
    unquoted.eq_ignore_ascii_case("utf-8")
}

/// A media type naming the type `text`, with no `charset` parameter other than UTF-8.
pub(crate) fn names_text(media_type: &str) -> bool {
    let mut parts = media_type.split(';');
    let essence = parts.next().unwrap_or_default().trim_matches(XML_SPACE);
    let named_text = essence
        .split_once('/')
        .is_some_and(|(kind, _)| kind.eq_ignore_ascii_case("text"));
    named_text
        && parts.all(|parameter| match parameter.split_once('=') {
            Some((name, value)) if name.trim_matches(XML_SPACE).eq_ignore_ascii_case("charset") => {
                is_utf_8(value)
            }
            _ => true,
        })
}

/// Canonical padded base64 of UTF-8 octets, whitespace removed, decoded.
pub(crate) fn decoded(written: &str) -> Option<String> {
    let base64: String = written.chars().filter(|c| !XML_SPACE.contains(c)).collect();
    if base64.is_empty() {
        return None;
    }
    String::from_utf8(STANDARD.decode(base64).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_charset_quoted_with_balanced_quotes_only() {
        for (media_type, text) in [
            ("text/plain;charset=utf-8", true),
            ("text/plain; charset=\"UTF-8\"", true),
            ("text/plain; charset=\"\"utf-8", false),
            ("text/plain; charset=utf-8\"", false),
            ("text/plain; charset=\"utf-8", false),
        ] {
            assert_eq!(names_text(media_type), text, "{media_type}");
        }
    }

    #[test]
    fn decodes_nothing_where_no_character_is_left_once_whitespace_is_removed() {
        assert_eq!(decoded(""), None);
        assert_eq!(decoded(" \n\t\r "), None);
        assert_eq!(decoded("aGk=").as_deref(), Some("hi"));
    }
}
