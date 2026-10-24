use super::expr::{binary, boolean, eval_fn, text, unary};
use super::{Eval, Expr, Fail};
use oxigraph::model::vocab::xsd;
use oxigraph::model::{BlankNode, Literal, NamedNode, Term};
use oxiri::Iri;
use regex::{Regex, RegexBuilder};
use sha2::{Digest, Sha256};
use spareval::ExpressionTerm as T;
use spargebra::algebra::{Expression, Function};

const REGEX_SIZE_LIMIT: usize = 1_000_000;

pub(super) struct Call<'a> {
    pub(super) arguments: std::vec::IntoIter<Expr>,
    pub(super) written: &'a [Expression],
    pub(super) base: Option<&'a Iri<String>>,
}

impl Call<'_> {
    fn take(&mut self) -> Expr {
        self.arguments.next().expect("the arity was checked")
    }
}

enum How {
    One(fn(T) -> Eval),
    Two(fn(T, T) -> Eval),
    Texts(fn(&str, &str) -> Eval),
    Special(fn(Call<'_>) -> Result<Expr, String>),
}

pub(super) struct Builtin {
    pub(super) name: &'static str,
    fewest: usize,
    most: usize,
    how: How,
}

impl Builtin {
    const fn one(name: &'static str, f: fn(T) -> Eval) -> Self {
        Self {
            name,
            fewest: 1,
            most: 1,
            how: How::One(f),
        }
    }

    const fn two(name: &'static str, f: fn(T, T) -> Eval) -> Self {
        Self {
            name,
            fewest: 2,
            most: 2,
            how: How::Two(f),
        }
    }

    const fn texts(name: &'static str, f: fn(&str, &str) -> Eval) -> Self {
        Self {
            name,
            fewest: 2,
            most: 2,
            how: How::Texts(f),
        }
    }

    const fn special(
        name: &'static str,
        fewest: usize,
        most: usize,
        f: fn(Call<'_>) -> Result<Expr, String>,
    ) -> Self {
        Self {
            name,
            fewest,
            most,
            how: How::Special(f),
        }
    }

    pub(super) fn accepts(&self, arguments: usize) -> bool {
        (self.fewest..=self.most).contains(&arguments)
    }

    pub(super) fn build(&self, mut call: Call<'_>) -> Result<Expr, String> {
        match self.how {
            How::One(f) => Ok(unary(call.take(), f)),
            How::Two(f) => Ok(binary(call.take(), call.take(), f)),
            How::Texts(f) => Ok(binary(call.take(), call.take(), move |a, b| {
                f(&string(a)?, &string(b)?)
            })),
            How::Special(f) => f(call),
        }
    }
}

pub(super) static BUILTINS: &[Builtin] = &[
    Builtin::one("STR", string_of),
    Builtin::special("IRI", 1, 1, iri),
    Builtin::special("BNODE", 0, 0, fresh_blank_node),
    Builtin::special("CONCAT", 1, usize::MAX, concatenation),
    Builtin::special("SUBSTR", 2, 3, substring),
    Builtin::one("STRLEN", string_length),
    Builtin::special("REPLACE", 3, 4, replace),
    Builtin::special("REGEX", 2, 3, regex_matches),
    Builtin::one("LCASE", |a| text(string(a)?.to_lowercase())),
    Builtin::one("UCASE", |a| text(string(a)?.to_uppercase())),
    Builtin::texts("STRSTARTS", |a, b| boolean(a.starts_with(b))),
    Builtin::texts("STRENDS", |a, b| boolean(a.ends_with(b))),
    Builtin::texts("CONTAINS", |a, b| boolean(a.contains(b))),
    Builtin::texts("STRBEFORE", before),
    Builtin::texts("STRAFTER", after),
    Builtin::one("ENCODE_FOR_URI", percent_encoded),
    Builtin::one("SHA256", sha256_hex),
    Builtin::two("STRDT", typed),
    Builtin::one("isIRI", |a| boolean(matches!(a, T::NamedNode(_)))),
    Builtin::one("isBLANK", |a| boolean(matches!(a, T::BlankNode(_)))),
    Builtin::one("isLITERAL", |a| {
        boolean(!matches!(a, T::NamedNode(_) | T::BlankNode(_)))
    }),
    Builtin::one("<http://www.w3.org/2001/XMLSchema#integer>", integer),
];

pub(super) fn lookup(function: &Function) -> Option<&'static Builtin> {
    let name = function.to_string();
    BUILTINS.iter().find(|builtin| builtin.name == name)
}

/// A language-tagged string is left to the general evaluator, which knows how its
/// tag combines.
fn string(term: T) -> Result<String, Fail> {
    match term {
        T::StringLiteral(value) => Ok(value),
        T::NamedNode(_)
        | T::BlankNode(_)
        | T::BooleanLiteral(_)
        | T::IntegerLiteral(_)
        | T::DecimalLiteral(_)
        | T::FloatLiteral(_)
        | T::DoubleLiteral(_)
        | T::DateTimeLiteral(_)
        | T::OtherTypedLiteral { .. } => Err(Fail::Error),
        _ => Err(Fail::GiveUp),
    }
}

fn index(term: T) -> Result<usize, Fail> {
    match term {
        T::IntegerLiteral(value) => usize::try_from(i64::from(value)).map_err(|_| Fail::Error),
        _ => Err(Fail::Error),
    }
}

fn string_of(term: T) -> Eval {
    Ok(T::StringLiteral(match Term::from(term) {
        Term::NamedNode(node) => node.into_string(),
        Term::Literal(value) => value.destruct().0,
        _ => return Err(Fail::Error),
    }))
}

fn iri(mut call: Call<'_>) -> Result<Expr, String> {
    let base = call.base.cloned();
    Ok(unary(call.take(), move |a| match a {
        T::NamedNode(node) => Ok(T::NamedNode(node)),
        T::StringLiteral(value) => {
            let iri = match &base {
                Some(base) => base.resolve(&value),
                None => Iri::parse(value),
            }
            .map_err(|_| Fail::Error)?;
            Ok(T::NamedNode(NamedNode::new_unchecked(iri.into_inner())))
        }
        _ => Err(Fail::Error),
    }))
}

/// A blank node no other row and no tree holds, made each time it is evaluated.
fn fresh_blank_node(_: Call<'_>) -> Result<Expr, String> {
    Ok(eval_fn(|_, _| Ok(T::BlankNode(BlankNode::default()))))
}

fn concatenation(call: Call<'_>) -> Result<Expr, String> {
    let parts: Vec<Expr> = call.arguments.collect();
    Ok(eval_fn(move |tree, row| {
        let mut joined = String::new();
        for part in &parts {
            joined += &string(part(tree, row)?)?;
        }
        text(joined)
    }))
}

fn substring(mut call: Call<'_>) -> Result<Expr, String> {
    let (source, start) = (call.take(), call.take());
    let length = call.arguments.next();
    Ok(eval_fn(move |tree, row| {
        let source = string(source(tree, row)?)?;
        let start = index(start(tree, row)?)?;
        let length = length.as_ref().map(|l| index(l(tree, row)?)).transpose()?;
        slice(&source, start, length)
    }))
}

fn slice(source: &str, start: usize, length: Option<usize>) -> Eval {
    let mut from = source
        .char_indices()
        .skip(start.checked_sub(1).ok_or(Fail::Error)?)
        .peekable();
    let Some(&(begin, _)) = from.peek() else {
        return text(String::new());
    };
    let end = length
        .and_then(|length| from.nth(length))
        .map_or(source.len(), |(end, _)| end);
    text(source[begin..end].to_owned())
}

fn string_length(term: T) -> Eval {
    let length = i64::try_from(string(term)?.chars().count()).map_err(|_| Fail::Error)?;
    Ok(T::IntegerLiteral(length.into()))
}

fn replace(mut call: Call<'_>) -> Result<Expr, String> {
    let regex = written_regex(call.written, 3)?;
    let (subject, _, replacement) = (call.take(), call.take(), call.take());
    Ok(eval_fn(move |tree, row| {
        let subject = string(subject(tree, row)?)?;
        let replacement = string(replacement(tree, row)?)?;
        text(
            regex
                .replace_all(&subject, replacement.as_str())
                .into_owned(),
        )
    }))
}

fn regex_matches(mut call: Call<'_>) -> Result<Expr, String> {
    let regex = written_regex(call.written, 2)?;
    Ok(unary(call.take(), move |a| {
        boolean(regex.is_match(&string(a)?))
    }))
}

fn written_regex(written: &[Expression], flags_at: usize) -> Result<Regex, String> {
    let pattern = literal(written.get(1));
    let flags = written
        .get(flags_at)
        .map_or(Some(""), |flags| literal(Some(flags)));
    let (Some(pattern), Some(flags)) = (pattern, flags) else {
        return Err("a regular expression that is not written out".to_owned());
    };
    build_regex(pattern, flags).ok_or_else(|| "a regular expression the engine refuses".to_owned())
}

fn literal(expression: Option<&Expression>) -> Option<&str> {
    match expression? {
        Expression::Literal(value) if value.datatype() == xsd::STRING => Some(value.value()),
        _ => None,
    }
}

fn build_regex(pattern: &str, flags: &str) -> Option<Regex> {
    let pattern = if flags.contains('q') {
        regex::escape(pattern)
    } else {
        pattern.to_owned()
    };
    let mut builder = RegexBuilder::new(&pattern);
    builder.size_limit(REGEX_SIZE_LIMIT);
    for flag in flags.chars() {
        match flag {
            's' => builder.dot_matches_new_line(true),
            'm' => builder.multi_line(true),
            'i' => builder.case_insensitive(true),
            'x' => builder.ignore_whitespace(true),
            'q' => &mut builder,
            _ => return None,
        };
    }
    builder.build().ok()
}

fn before(whole: &str, part: &str) -> Eval {
    text(
        whole
            .find(part)
            .map_or(String::new(), |at| whole[..at].to_owned()),
    )
}

fn after(whole: &str, part: &str) -> Eval {
    text(
        whole
            .find(part)
            .map_or(String::new(), |at| whole[at + part.len()..].to_owned()),
    )
}

fn percent_encoded(term: T) -> Eval {
    let mut encoded = String::new();
    for byte in string(term)?.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded += &format!("%{byte:02X}"),
        }
    }
    text(encoded)
}

fn sha256_hex(term: T) -> Eval {
    let digest = Sha256::digest(string(term)?.as_bytes());
    text(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn typed(value: T, datatype: T) -> Eval {
    match (value, datatype) {
        (T::StringLiteral(value), T::NamedNode(datatype)) => Ok(T::from(Term::from(
            Literal::new_typed_literal(value, datatype),
        ))),
        _ => Err(Fail::Error),
    }
}

fn integer(term: T) -> Eval {
    match term {
        T::IntegerLiteral(value) => Ok(T::IntegerLiteral(value)),
        T::StringLiteral(value) => value
            .parse()
            .map(T::IntegerLiteral)
            .map_err(|_| Fail::Error),
        _ => Err(Fail::GiveUp),
    }
}
