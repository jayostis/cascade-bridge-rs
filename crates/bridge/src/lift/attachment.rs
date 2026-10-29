use crate::decode::XML_SPACE;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

pub(crate) const TEXT_CONTENT: &str = "https://dom.spec.whatwg.org/#dom-node-textcontent";
pub(crate) const TEXT_DECODED_FROM_BASE64: &str =
    "https://ns.cascadeprotocol.org/bridge/v1-draft#textDecodedFromBase64";
pub(crate) const TEXT_WITH_ELEMENT_BREAKS: &str =
    "https://ns.cascadeprotocol.org/bridge/v1-draft#textWithElementBreaks";

fn is_utf_8(charset: &str) -> bool {
    charset
        .trim_matches(XML_SPACE)
        .trim_matches('"')
        .eq_ignore_ascii_case("utf-8")
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
    String::from_utf8(STANDARD.decode(base64).ok()?).ok()
}
