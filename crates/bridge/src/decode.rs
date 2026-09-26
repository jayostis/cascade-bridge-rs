use crate::error::{Error, Result};
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};
use std::borrow::Cow;

/// The XML declaration is the first thing in an entity.
const WINDOW: usize = 256;

pub(crate) fn decode(bytes: &[u8]) -> Result<Cow<'_, str>> {
    let (encoding, rest) = sniff(bytes)?;
    if encoding == UTF_8 {
        return Ok(Cow::Borrowed(std::str::from_utf8(rest)?));
    }
    let (text, _, malformed) = encoding.decode(rest);
    if malformed {
        return Err(Error::msg(format!(
            "the document is not well-formed {}",
            encoding.name()
        )));
    }
    Ok(Cow::Owned(text.into_owned()))
}

fn sniff(bytes: &[u8]) -> Result<(&'static Encoding, &[u8])> {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return Ok((UTF_8, rest));
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return Ok((UTF_16LE, rest));
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return Ok((UTF_16BE, rest));
    }
    // Without a mark, a sixteen-bit encoding still shows in the first two
    // characters of "<?xml", which are ASCII in every encoding XML allows.
    if bytes.starts_with(&[0x00, b'<', 0x00, b'?']) {
        return Ok((UTF_16BE, bytes));
    }
    if bytes.starts_with(&[b'<', 0x00, b'?', 0x00]) {
        return Ok((UTF_16LE, bytes));
    }
    match declared(bytes) {
        Some(label) => Encoding::for_label(label.as_bytes())
            .map(|e| (e, bytes))
            .ok_or_else(|| {
                Error::msg(format!(
                    "the document declares an unknown encoding: {label}"
                ))
            }),
        None => Ok((UTF_8, bytes)),
    }
}

fn declared(bytes: &[u8]) -> Option<String> {
    let head = &bytes[..bytes.len().min(WINDOW)];
    let head = String::from_utf8_lossy(head);
    let decl = head.strip_prefix("<?xml")?;
    let decl = &decl[..decl.find("?>")?];
    let after = decl.split_once("encoding")?.1;
    let after = after.trim_start().strip_prefix('=')?.trim_start();
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value = &after[1..];
    Some(value[..value.find(quote)?].to_owned())
}

pub(crate) const XML_SPACE: [char; 4] = [' ', '\t', '\r', '\n'];

pub(crate) fn is_xml_space(s: &str) -> bool {
    s.chars().all(|c| XML_SPACE.contains(&c))
}

pub(crate) fn normalise_line_endings(s: &str) -> Cow<'_, str> {
    if !s.contains('\r') {
        return Cow::Borrowed(s);
    }
    Cow::Owned(s.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Runs on the raw value, before any character reference is resolved, which
/// keeps "&#10;" a line feed.
pub(crate) fn normalise_attribute_value(raw: &str) -> Cow<'_, str> {
    if !raw.contains(['\t', '\r', '\n']) {
        return Cow::Borrowed(raw);
    }
    Cow::Owned(normalise_line_endings(raw).replace(['\t', '\n'], " "))
}
