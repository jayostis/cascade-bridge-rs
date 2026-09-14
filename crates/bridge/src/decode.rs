// An XML document says what its bytes mean, in a byte-order mark or in its
// declaration. Assuming UTF-8 silently corrupts every other encoding, so the
// bytes are read before the parser sees them.
use crate::error::{Error, Result};
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};
use std::borrow::Cow;

/// How far into the document a declaration may still be found. The XML
/// declaration is the first thing in an entity, so a short window is enough
/// and a document without one costs nothing.
const WINDOW: usize = 256;

/// The document's characters. A UTF-8 document borrows its bytes; any other
/// encoding is transcoded, which is the one case that holds the whole
/// document at once.
pub fn decode(bytes: &[u8]) -> Result<Cow<'_, str>> {
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

/// The encoding, and the bytes after any byte-order mark. A mark wins over a
/// declaration, as XML requires; a declaration wins over the default.
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

/// The `encoding` pseudo-attribute of the XML declaration, if the document
/// opens with one.
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

/// XML's S production, and nothing else. A text child made only of these is
/// dropped; a no-break space is a character and stays.
pub fn is_xml_space(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
}

/// XML normalises every line ending to a single line feed before a parser
/// sees a character, so a document checked out with CRLF lifts to the same
/// graph as one checked out with LF.
pub fn normalise_line_endings(s: &str) -> Cow<'_, str> {
    if !s.contains('\r') {
        return Cow::Borrowed(s);
    }
    Cow::Owned(s.replace("\r\n", "\n").replace('\r', "\n"))
}
