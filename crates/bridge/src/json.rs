// A hand-written reader: a JSON library's map keeps one member of a repeated name,
// and the lift keeps every one, in document order, with each number's text.
use crate::error::{Error, Result};
use std::ops::Range;

pub(crate) mod addressing;

/// Past this, a document is refused rather than read into a stack it would exhaust.
const DEPTH: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Object(Vec<(String, Node)>),
    Array(Vec<Node>),
    String(String),
    /// As the document writes it.
    Number(String),
    Bool(bool),
    Null,
}

/// `span` is where the value stands in the text it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    pub(crate) value: Value,
    pub(crate) span: Range<usize>,
}

impl Node {
    /// What the lift writes for a string, a number, `true` or `false`.
    pub(crate) fn scalar(&self) -> Option<&str> {
        match &self.value {
            Value::String(text) | Value::Number(text) => Some(text),
            Value::Bool(true) => Some("true"),
            Value::Bool(false) => Some("false"),
            _ => None,
        }
    }

    pub(crate) fn is_container(&self) -> bool {
        matches!(self.value, Value::Object(_) | Value::Array(_))
    }

    /// Each child with its reference token, in document order.
    pub(crate) fn children(&self) -> Vec<(String, &Node)> {
        match &self.value {
            Value::Object(members) => members
                .iter()
                .map(|(name, member)| (name.clone(), member))
                .collect(),
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(index, item)| (index.to_string(), item))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The node reached by child positions, a member's among its object's members.
    pub(crate) fn at(&self, positions: &[usize]) -> Option<&Node> {
        let mut node = self;
        for &position in positions {
            node = match &node.value {
                Value::Object(members) => &members.get(position)?.1,
                Value::Array(items) => items.get(position)?,
                _ => return None,
            };
        }
        Some(node)
    }
}

/// UTF-8 with no byte order mark, as RFC 8259 has a JSON text exchanged.
pub(crate) fn decode(bytes: &[u8]) -> Result<&str> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(Error::document(
            "the document begins with a byte order mark, which a JSON text does not",
        ));
    }
    Ok(std::str::from_utf8(bytes)?)
}

pub(crate) fn parse(text: &str) -> Result<Node> {
    let mut reader = Reader {
        text,
        bytes: text.as_bytes(),
        at: 0,
    };
    reader.space();
    let node = reader.value(0)?;
    reader.space();
    if reader.at != reader.bytes.len() {
        return Err(reader.error("more after the document's value"));
    }
    Ok(node)
}

struct Reader<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn error(&self, what: &str) -> Error {
        Error::document(format!(
            "the document is not JSON at byte {}: {what}",
            self.at
        ))
    }

    fn space(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.bytes.get(self.at) {
            self.at += 1;
        }
    }

    fn take(&mut self, expected: u8) -> Result<()> {
        if self.bytes.get(self.at) != Some(&expected) {
            return Err(self.error(&format!("expected '{}'", expected as char)));
        }
        self.at += 1;
        Ok(())
    }

    fn value(&mut self, depth: usize) -> Result<Node> {
        if depth > DEPTH {
            return Err(self.error("nested deeper than this Bridge reads"));
        }
        let start = self.at;
        let value = match self.bytes.get(self.at) {
            Some(b'{') => self.object(depth)?,
            Some(b'[') => self.array(depth)?,
            Some(b'"') => Value::String(self.string()?),
            Some(b'-' | b'0'..=b'9') => Value::Number(self.number()?),
            Some(b't') => self.literal("true", Value::Bool(true))?,
            Some(b'f') => self.literal("false", Value::Bool(false))?,
            Some(b'n') => self.literal("null", Value::Null)?,
            _ => return Err(self.error("expected a value")),
        };
        Ok(Node {
            value,
            span: start..self.at,
        })
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value> {
        if !self.bytes[self.at..].starts_with(word.as_bytes()) {
            return Err(self.error("expected a value"));
        }
        self.at += word.len();
        Ok(value)
    }

    fn object(&mut self, depth: usize) -> Result<Value> {
        self.take(b'{')?;
        let mut members = Vec::new();
        self.space();
        if self.bytes.get(self.at) == Some(&b'}') {
            self.at += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.space();
            if self.bytes.get(self.at) != Some(&b'"') {
                return Err(self.error("expected a member's name"));
            }
            let name = self.string()?;
            self.space();
            self.take(b':')?;
            self.space();
            members.push((name, self.value(depth + 1)?));
            self.space();
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value> {
        self.take(b'[')?;
        let mut items = Vec::new();
        self.space();
        if self.bytes.get(self.at) == Some(&b']') {
            self.at += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.space();
            items.push(self.value(depth + 1)?);
            self.space();
            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn number(&mut self) -> Result<String> {
        let start = self.at;
        if self.bytes.get(self.at) == Some(&b'-') {
            self.at += 1;
        }
        match self.bytes.get(self.at) {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(self.error("expected a digit")),
        }
        if self.bytes.get(self.at) == Some(&b'.') {
            self.at += 1;
            self.some_digits()?;
        }
        if let Some(b'e' | b'E') = self.bytes.get(self.at) {
            self.at += 1;
            if let Some(b'+' | b'-') = self.bytes.get(self.at) {
                self.at += 1;
            }
            self.some_digits()?;
        }
        Ok(self.text[start..self.at].to_owned())
    }

    fn digits(&mut self) {
        while let Some(b'0'..=b'9') = self.bytes.get(self.at) {
            self.at += 1;
        }
    }

    fn some_digits(&mut self) -> Result<()> {
        let start = self.at;
        self.digits();
        if self.at == start {
            return Err(self.error("expected a digit"));
        }
        Ok(())
    }

    fn string(&mut self) -> Result<String> {
        self.take(b'"')?;
        let mut decoded = String::new();
        loop {
            let run = self.at;
            while let Some(&byte) = self.bytes.get(self.at) {
                if byte == b'"' || byte == b'\\' || byte < 0x20 {
                    break;
                }
                self.at += 1;
            }
            decoded.push_str(&self.text[run..self.at]);
            match self.bytes.get(self.at) {
                Some(b'"') => {
                    self.at += 1;
                    return Ok(decoded);
                }
                Some(b'\\') => {
                    self.at += 1;
                    decoded.push(self.escape()?);
                }
                Some(_) => return Err(self.error("a control character unescaped in a string")),
                None => return Err(self.error("a string not closed")),
            }
        }
    }

    fn escape(&mut self) -> Result<char> {
        let Some(&byte) = self.bytes.get(self.at) else {
            return Err(self.error("a string not closed"));
        };
        self.at += 1;
        Ok(match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                let high = self.hex()?;
                if !(0xD800..0xE000).contains(&high) {
                    return char::from_u32(high).ok_or_else(|| self.error("an escape"));
                }
                if high >= 0xDC00 || !self.bytes[self.at..].starts_with(b"\\u") {
                    return Err(self.error("a lone surrogate"));
                }
                self.at += 2;
                let low = self.hex()?;
                if !(0xDC00..0xE000).contains(&low) {
                    return Err(self.error("a lone surrogate"));
                }
                char::from_u32(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00))
                    .ok_or_else(|| self.error("an escape"))?
            }
            _ => return Err(self.error("an unknown escape")),
        })
    }

    fn hex(&mut self) -> Result<u32> {
        let digits = self
            .text
            .get(self.at..self.at + 4)
            .filter(|digits| digits.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| self.error("an escape without four hexadecimal digits"))?;
        self.at += 4;
        Ok(u32::from_str_radix(digits, 16).expect("four hexadecimal digits"))
    }
}

/// RFC 3986's fragment characters other than the unreserved.
const FRAGMENT_SAFE: &str = "/?:@!$&'()*+,;=-._~";

/// A JSON Pointer in its URI fragment identifier representation, without the `#`.
pub(crate) fn pointer(tokens: &[String]) -> String {
    let mut written = String::new();
    for token in tokens {
        written.push('/');
        for character in token.replace('~', "~0").replace('/', "~1").chars() {
            if character.is_ascii_alphanumeric() || FRAGMENT_SAFE.contains(character) {
                written.push(character);
                continue;
            }
            let mut octets = [0; 4];
            for octet in character.encode_utf8(&mut octets).as_bytes() {
                written.push_str(&format!("%{octet:02X}"));
            }
        }
    }
    written
}

/// The reference tokens of a pointer in its fragment representation; none where it is not one.
pub(crate) fn tokens(pointer: &str) -> Option<Vec<String>> {
    if pointer.is_empty() {
        return Some(Vec::new());
    }
    let unescaped = percent_decoded(pointer.strip_prefix('/')?)?;
    unescaped
        .split('/')
        .map(|escaped| {
            let mut token = String::new();
            let mut characters = escaped.chars();
            while let Some(character) = characters.next() {
                match character {
                    '~' => match characters.next()? {
                        '0' => token.push('~'),
                        '1' => token.push('/'),
                        _ => return None,
                    },
                    other => token.push(other),
                }
            }
            Some(token)
        })
        .collect()
}

fn percent_decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut octets = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let hex = text.get(at + 1..at + 3)?;
            if !hex.bytes().all(|digit| digit.is_ascii_hexdigit()) {
                return None;
            }
            octets.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            octets.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(octets).ok()
}

/// Every node a pointer selects, with its child positions: more than one through a
/// name its object repeats.
pub(crate) fn selected<'a>(root: &'a Node, pointer: &str) -> Vec<(Vec<usize>, &'a Node)> {
    let Some(tokens) = tokens(pointer) else {
        return Vec::new();
    };
    let mut reached = vec![(Vec::new(), root)];
    for token in &tokens {
        let mut next = Vec::new();
        for (positions, node) in reached {
            for (position, (name, child)) in node.children().into_iter().enumerate() {
                if name == *token {
                    let mut walked = positions.clone();
                    walked.push(position);
                    next.push((walked, child));
                }
            }
        }
        reached = next;
    }
    reached
}

/// The tokens of the node at child positions below `root`.
pub(crate) fn tokens_at(root: &Node, positions: &[usize]) -> Vec<String> {
    let mut tokens = Vec::with_capacity(positions.len());
    let mut node = root;
    for &position in positions {
        let (token, child) = node
            .children()
            .into_iter()
            .nth(position)
            .expect("a position below the root");
        tokens.push(token);
        node = child;
    }
    tokens
}

/// A segment of a record path: a member's name, or every child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Segment {
    Name(String),
    Wildcard,
}

/// `$` followed by `.name`, `['name']` and `[*]`, the subset of RFC 9535 a record path is written in.
pub(crate) fn path(written: &str) -> Result<Vec<Segment>> {
    let outside = || {
        Error::adapter(format!(
            "the record path {written} is outside the subset of RFC 9535 JSONPath a record path \
             is written in: $ followed by .name, ['name'] and [*]"
        ))
    };
    let mut rest = written.strip_prefix('$').ok_or_else(outside)?;
    let mut segments = Vec::new();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("[*]") {
            segments.push(Segment::Wildcard);
            rest = after;
        } else if let Some(after) = rest.strip_prefix("['") {
            let (name, after) = quoted(after).ok_or_else(outside)?;
            segments.push(Segment::Name(name));
            rest = after;
        } else if let Some(after) = rest.strip_prefix('.') {
            let end = after
                .char_indices()
                .find(|(at, character)| !shorthand(*character, *at == 0))
                .map_or(after.len(), |(at, _)| at);
            if end == 0 {
                return Err(outside());
            }
            segments.push(Segment::Name(after[..end].to_owned()));
            rest = &after[end..];
        } else {
            return Err(outside());
        }
    }
    Ok(segments)
}

/// RFC 9535's `name-first` and `name-char`.
fn shorthand(character: char, first: bool) -> bool {
    character.is_ascii_alphabetic()
        || character == '_'
        || character >= '\u{80}'
        || (!first && character.is_ascii_digit())
}

/// A single-quoted name up to its closing `']`, its escapes decoded.
fn quoted(text: &str) -> Option<(String, &str)> {
    let mut name = String::new();
    let mut characters = text.char_indices();
    while let Some((at, character)) = characters.next() {
        match character {
            '\'' => return Some((name, text[at + 1..].strip_prefix(']')?)),
            '\\' => {
                let (_, escaped) = characters.next()?;
                name.push(match escaped {
                    '\'' => '\'',
                    '"' => '"',
                    '\\' => '\\',
                    '/' => '/',
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'u' => {
                        let hex: String = (0..4)
                            .filter_map(|_| characters.next())
                            .map(|(_, c)| c)
                            .collect();
                        char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?
                    }
                    _ => return None,
                });
            }
            other => name.push(other),
        }
    }
    None
}

/// Each object the path selects, by its child positions, in document order.
pub(crate) fn records(root: &Node, segments: &[Segment]) -> Vec<Vec<usize>> {
    let mut reached: Vec<(Vec<usize>, &Node)> = vec![(Vec::new(), root)];
    for segment in segments {
        let mut next = Vec::new();
        for (positions, node) in reached {
            let children: Vec<(usize, &Node)> = match (&node.value, segment) {
                (Value::Object(members), Segment::Name(name)) => members
                    .iter()
                    .enumerate()
                    .filter(|(_, (named, _))| named == name)
                    .map(|(position, (_, member))| (position, member))
                    .collect(),
                (Value::Object(members), Segment::Wildcard) => members
                    .iter()
                    .enumerate()
                    .map(|(position, (_, member))| (position, member))
                    .collect(),
                (Value::Array(items), Segment::Wildcard) => items.iter().enumerate().collect(),
                _ => Vec::new(),
            };
            for (position, child) in children {
                let mut walked = positions.clone();
                walked.push(position);
                next.push((walked, child));
            }
        }
        reached = next;
    }
    reached
        .into_iter()
        .filter(|(_, node)| matches!(node.value, Value::Object(_)))
        .map(|(positions, _)| positions)
        .collect()
}

#[cfg(test)]
mod tests;
