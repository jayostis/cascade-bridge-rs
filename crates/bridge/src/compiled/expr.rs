use super::lower::{Lowering, Slots};
use super::{run, Eval, Expr, Fail};
use oxigraph::model::vocab::xsd;
use oxigraph::model::{Literal, NamedNode, Term};
use oxiri::Iri;
use oxsdatatypes::{Boolean, Decimal};
use regex::{Regex, RegexBuilder};
use sha2::{Digest, Sha256};
use spareval::ExpressionTerm as T;
use spargebra::algebra::{Expression, Function};
use std::cmp::Ordering;

const REGEX_SIZE_LIMIT: usize = 1_000_000;

/// A string, or the reason there is none: a language-tagged one is left to the
/// general evaluator, which knows how its tag combines.
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

fn truth(term: &T) -> Result<bool, Fail> {
    match term {
        T::BooleanLiteral(value) => Ok((*value).into()),
        T::StringLiteral(value) => Ok(!value.is_empty()),
        T::IntegerLiteral(value) => Ok(Boolean::from(*value).into()),
        T::DecimalLiteral(value) => Ok(Boolean::from(*value).into()),
        T::FloatLiteral(value) => Ok(Boolean::from(*value).into()),
        T::DoubleLiteral(value) => Ok(Boolean::from(*value).into()),
        _ => Err(Fail::Error),
    }
}

pub(super) fn holds(expr: &Expr, tree: &super::Tree, row: &super::Row) -> Result<bool, Fail> {
    match expr(tree, row).and_then(|term| truth(&term)) {
        Err(Fail::Error) => Ok(false),
        outcome => outcome,
    }
}

fn equals(a: &T, b: &T) -> Result<bool, Fail> {
    match (a, b) {
        (T::NamedNode(_) | T::BlankNode(_) | T::LangStringLiteral { .. }, _) => Ok(a == b),
        (T::StringLiteral(a), T::StringLiteral(b)) => Ok(a == b),
        (T::StringLiteral(_), T::OtherTypedLiteral { .. }) => Err(Fail::Error),
        (T::StringLiteral(_), _) => Ok(false),
        (T::IntegerLiteral(a), T::IntegerLiteral(b)) => Ok(a == b),
        (T::BooleanLiteral(a), T::BooleanLiteral(b)) => Ok(a == b),
        _ => Err(Fail::GiveUp),
    }
}

fn compare(a: &T, b: &T) -> Result<Ordering, Fail> {
    if a == b {
        return Ok(Ordering::Equal);
    }
    match (a, b) {
        (T::StringLiteral(a), T::StringLiteral(b)) => Ok(a.cmp(b)),
        (T::IntegerLiteral(a), T::IntegerLiteral(b)) => a.partial_cmp(b).ok_or(Fail::Error),
        (T::DateTimeLiteral(a), T::DateTimeLiteral(b)) => a.partial_cmp(b).ok_or(Fail::Error),
        (T::StringLiteral(_), _) | (_, T::StringLiteral(_)) => Err(Fail::Error),
        _ => Err(Fail::GiveUp),
    }
}

/// The order MIN and MAX keep: blank nodes, then IRIs, then literals.
pub(super) fn order(a: &T, b: &T) -> Result<Ordering, Fail> {
    let rank = |term: &T| match term {
        T::BlankNode(_) => Some(0),
        T::NamedNode(_) => Some(1),
        T::StringLiteral(_) | T::IntegerLiteral(_) => Some(2),
        _ => None,
    };
    let (ra, rb) = (rank(a).ok_or(Fail::GiveUp)?, rank(b).ok_or(Fail::GiveUp)?);
    match (a, b) {
        _ if ra != rb => Ok(ra.cmp(&rb)),
        (T::BlankNode(a), T::BlankNode(b)) => Ok(a.as_str().cmp(b.as_str())),
        (T::NamedNode(a), T::NamedNode(b)) => Ok(a.as_str().cmp(b.as_str())),
        _ => compare(a, b),
    }
}

fn index(term: T) -> Result<usize, Fail> {
    match term {
        T::IntegerLiteral(value) => usize::try_from(i64::from(value)).map_err(|_| Fail::Error),
        _ => Err(Fail::Error),
    }
}

fn pattern(text: &str, flags: Option<&str>) -> Option<Regex> {
    let flags = flags.unwrap_or_default();
    let text = if flags.contains('q') {
        regex::escape(text)
    } else {
        text.to_owned()
    };
    let mut builder = RegexBuilder::new(&text);
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

fn literal(expression: Option<&Expression>) -> Option<&str> {
    match expression? {
        Expression::Literal(value) if value.datatype() == xsd::STRING => Some(value.value()),
        _ => None,
    }
}

fn unary(a: Expr, f: impl Fn(T) -> Eval + Send + Sync + 'static) -> Expr {
    Box::new(move |tree, row| f(a(tree, row)?))
}

fn binary(a: Expr, b: Expr, f: impl Fn(T, T) -> Eval + Send + Sync + 'static) -> Expr {
    Box::new(move |tree, row| f(a(tree, row)?, b(tree, row)?))
}

fn text(value: String) -> Eval {
    Ok(T::StringLiteral(value))
}

fn boolean(value: bool) -> Eval {
    Ok(T::from(value))
}

fn arithmetic(
    a: Expr,
    b: Expr,
    integer: fn(oxsdatatypes::Integer, oxsdatatypes::Integer) -> Option<T>,
) -> Expr {
    binary(a, b, move |a, b| match (a, b) {
        (T::IntegerLiteral(a), T::IntegerLiteral(b)) => integer(a, b).ok_or(Fail::Error),
        _ => Err(Fail::GiveUp),
    })
}

fn strings(a: Expr, b: Expr, f: impl Fn(&str, &str) -> Eval + Send + Sync + 'static) -> Expr {
    binary(a, b, move |a, b| f(&string(a)?, &string(b)?))
}

impl Lowering {
    pub(super) fn expression(
        &mut self,
        expression: &Expression,
        bound: &Slots,
    ) -> Result<Expr, String> {
        Ok(match expression {
            Expression::NamedNode(node) => {
                let term = T::NamedNode(node.clone());
                Box::new(move |_, _| Ok(term.clone()))
            }
            Expression::Literal(value) => {
                let term = T::from(Term::from(value.clone()));
                Box::new(move |_, _| Ok(term.clone()))
            }
            Expression::Variable(variable) => {
                let slot = self.slot(variable);
                Box::new(move |_, row| {
                    row[slot]
                        .as_deref()
                        .map(|term| T::from(term.clone()))
                        .ok_or(Fail::Error)
                })
            }
            Expression::Bound(variable) => {
                let slot = self.slot(variable);
                Box::new(move |_, row| boolean(row[slot].is_some()))
            }
            Expression::Exists(body) => {
                let plan = self.exists(body, bound)?;
                Box::new(move |tree, row| boolean(run::exists(&plan, tree, row)?))
            }
            Expression::Or(a, b) => {
                let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
                Box::new(move |tree, row| connective(&[&a, &b], tree, row, true))
            }
            Expression::And(a, b) => {
                let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
                Box::new(move |tree, row| connective(&[&a, &b], tree, row, false))
            }
            Expression::Not(a) => {
                let a = self.expression(a, bound)?;
                unary(a, |a| boolean(!truth(&a)?))
            }
            Expression::Equal(a, b) => {
                let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
                binary(a, b, |a, b| boolean(equals(&a, &b)?))
            }
            Expression::SameTerm(a, b) => {
                let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
                binary(a, b, |a, b| boolean(a == b))
            }
            Expression::Greater(a, b) => self.ordering(a, b, bound, Ordering::is_gt)?,
            Expression::GreaterOrEqual(a, b) => self.ordering(a, b, bound, Ordering::is_ge)?,
            Expression::Less(a, b) => self.ordering(a, b, bound, Ordering::is_lt)?,
            Expression::LessOrEqual(a, b) => self.ordering(a, b, bound, Ordering::is_le)?,
            Expression::In(a, list) => {
                let a = self.expression(a, bound)?;
                let list = list
                    .iter()
                    .map(|e| self.expression(e, bound))
                    .collect::<Result<Vec<_>, _>>()?;
                Box::new(move |tree, row| {
                    let needle = a(tree, row)?;
                    let mut failed = false;
                    for candidate in &list {
                        match candidate(tree, row).and_then(|c| equals(&needle, &c)) {
                            Ok(true) => return boolean(true),
                            Ok(false) => {}
                            Err(Fail::Error) => failed = true,
                            Err(other) => return Err(other),
                        }
                    }
                    if failed {
                        Err(Fail::Error)
                    } else {
                        boolean(false)
                    }
                })
            }
            Expression::Add(a, b) => {
                let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
                arithmetic(a, b, |a, b| a.checked_add(b).map(T::IntegerLiteral))
            }
            Expression::Subtract(a, b) => {
                let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
                arithmetic(a, b, |a, b| a.checked_sub(b).map(T::IntegerLiteral))
            }
            Expression::Multiply(a, b) => {
                let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
                arithmetic(a, b, |a, b| a.checked_mul(b).map(T::IntegerLiteral))
            }
            Expression::Divide(a, b) => {
                let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
                arithmetic(a, b, |a, b| {
                    Decimal::from(a).checked_div(b).map(T::DecimalLiteral)
                })
            }
            Expression::If(c, a, b) => {
                let (c, a, b) = (
                    self.expression(c, bound)?,
                    self.expression(a, bound)?,
                    self.expression(b, bound)?,
                );
                Box::new(move |tree, row| {
                    if truth(&c(tree, row)?)? {
                        a(tree, row)
                    } else {
                        b(tree, row)
                    }
                })
            }
            Expression::Coalesce(list) => {
                let list = list
                    .iter()
                    .map(|e| self.expression(e, bound))
                    .collect::<Result<Vec<_>, _>>()?;
                Box::new(move |tree, row| {
                    for candidate in &list {
                        match candidate(tree, row) {
                            Err(Fail::Error) => {}
                            outcome => return outcome,
                        }
                    }
                    Err(Fail::Error)
                })
            }
            Expression::FunctionCall(function, arguments) => {
                let arguments = arguments
                    .iter()
                    .map(|e| self.expression(e, bound))
                    .collect::<Result<Vec<_>, _>>()?;
                self.function(function, arguments, expression)?
            }
            other => return Err(format!("the expression {other}")),
        })
    }

    fn ordering(
        &mut self,
        a: &Expression,
        b: &Expression,
        bound: &Slots,
        wanted: fn(Ordering) -> bool,
    ) -> Result<Expr, String> {
        let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
        Ok(binary(a, b, move |a, b| boolean(wanted(compare(&a, &b)?))))
    }

    fn function(
        &mut self,
        function: &Function,
        arguments: Vec<Expr>,
        call: &Expression,
    ) -> Result<Expr, String> {
        let mut arguments = arguments.into_iter();
        let Expression::FunctionCall(_, written) = call else {
            unreachable!("a function is compiled from its call")
        };
        Ok(match function {
            Function::Str => unary(take(&mut arguments), |a| {
                Ok(T::StringLiteral(match Term::from(a) {
                    Term::NamedNode(node) => node.into_string(),
                    Term::Literal(value) => value.destruct().0,
                    _ => return Err(Fail::Error),
                }))
            }),
            Function::Iri => {
                let base = self.base.clone();
                unary(take(&mut arguments), move |a| match a {
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
                })
            }
            Function::Concat => {
                let parts: Vec<Expr> = arguments.collect();
                Box::new(move |tree, row| {
                    let mut joined = String::new();
                    for part in &parts {
                        joined += &string(part(tree, row)?)?;
                    }
                    text(joined)
                })
            }
            Function::SubStr => {
                let (source, start) = (take(&mut arguments), take(&mut arguments));
                let length = arguments.next();
                Box::new(move |tree, row| {
                    let source = string(source(tree, row)?)?;
                    let start = index(start(tree, row)?)?;
                    let length = length.as_ref().map(|l| index(l(tree, row)?)).transpose()?;
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
                })
            }
            Function::StrLen => unary(take(&mut arguments), |a| {
                let length = i64::try_from(string(a)?.chars().count()).map_err(|_| Fail::Error)?;
                Ok(T::IntegerLiteral(length.into()))
            }),
            Function::Replace | Function::Regex => {
                let replacing = matches!(function, Function::Replace);
                let flags_at = if replacing { 3 } else { 2 };
                let static_pattern = literal(written.get(1));
                let static_flags = written.get(flags_at).map(|f| literal(Some(f)));
                let (Some(text_of), Some(flags)) =
                    (static_pattern, static_flags.unwrap_or(Some("")))
                else {
                    return Err("a regular expression that is not written out".to_owned());
                };
                let regex = pattern(text_of, Some(flags))
                    .ok_or_else(|| "a regular expression the engine refuses".to_owned())?;
                let subject = take(&mut arguments);
                let _ = take(&mut arguments);
                if replacing {
                    let replacement = take(&mut arguments);
                    Box::new(move |tree, row| {
                        let subject = string(subject(tree, row)?)?;
                        let replacement = string(replacement(tree, row)?)?;
                        text(
                            regex
                                .replace_all(&subject, replacement.as_str())
                                .into_owned(),
                        )
                    })
                } else {
                    unary(subject, move |a| boolean(regex.is_match(&string(a)?)))
                }
            }
            Function::LCase => unary(take(&mut arguments), |a| text(string(a)?.to_lowercase())),
            Function::UCase => unary(take(&mut arguments), |a| text(string(a)?.to_uppercase())),
            Function::StrStarts => strings(take(&mut arguments), take(&mut arguments), |a, b| {
                boolean(a.starts_with(b))
            }),
            Function::StrEnds => strings(take(&mut arguments), take(&mut arguments), |a, b| {
                boolean(a.ends_with(b))
            }),
            Function::Contains => strings(take(&mut arguments), take(&mut arguments), |a, b| {
                boolean(a.contains(b))
            }),
            Function::StrBefore => strings(take(&mut arguments), take(&mut arguments), |a, b| {
                text(a.find(b).map_or(String::new(), |at| a[..at].to_owned()))
            }),
            Function::StrAfter => strings(take(&mut arguments), take(&mut arguments), |a, b| {
                text(
                    a.find(b)
                        .map_or(String::new(), |at| a[at + b.len()..].to_owned()),
                )
            }),
            Function::EncodeForUri => unary(take(&mut arguments), |a| {
                let mut encoded = String::new();
                for byte in string(a)?.bytes() {
                    match byte {
                        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                            encoded.push(char::from(byte));
                        }
                        _ => encoded += &format!("%{byte:02X}"),
                    }
                }
                text(encoded)
            }),
            Function::Sha256 => unary(take(&mut arguments), |a| {
                let digest = Sha256::digest(string(a)?.as_bytes());
                text(digest.iter().map(|byte| format!("{byte:02x}")).collect())
            }),
            Function::StrDt => binary(take(&mut arguments), take(&mut arguments), |a, b| {
                match (a, b) {
                    (T::StringLiteral(value), T::NamedNode(datatype)) => Ok(T::from(Term::from(
                        Literal::new_typed_literal(value, datatype),
                    ))),
                    _ => Err(Fail::Error),
                }
            }),
            Function::IsIri => unary(take(&mut arguments), |a| {
                boolean(matches!(a, T::NamedNode(_)))
            }),
            Function::IsBlank => unary(take(&mut arguments), |a| {
                boolean(matches!(a, T::BlankNode(_)))
            }),
            Function::IsLiteral => unary(take(&mut arguments), |a| {
                boolean(!matches!(a, T::NamedNode(_) | T::BlankNode(_)))
            }),
            Function::Custom(name) if name.as_ref() == xsd::INTEGER => {
                unary(take(&mut arguments), |a| match a {
                    T::IntegerLiteral(value) => Ok(T::IntegerLiteral(value)),
                    T::StringLiteral(value) => value
                        .parse()
                        .map(T::IntegerLiteral)
                        .map_err(|_| Fail::Error),
                    _ => Err(Fail::GiveUp),
                })
            }
            other => return Err(format!("the function {other}")),
        })
    }
}

/// OR and AND: a decided operand wins over an error, else an error stands.
fn connective(operands: &[&Expr], tree: &super::Tree, row: &super::Row, decided_by: bool) -> Eval {
    let mut failed = false;
    for operand in operands {
        match operand(tree, row).and_then(|term| truth(&term)) {
            Ok(value) if value == decided_by => return boolean(decided_by),
            Ok(_) => {}
            Err(Fail::Error) => failed = true,
            Err(other) => return Err(other),
        }
    }
    if failed {
        Err(Fail::Error)
    } else {
        boolean(!decided_by)
    }
}

fn take(arguments: &mut std::vec::IntoIter<Expr>) -> Expr {
    arguments.next().expect("the parser checks the arity")
}
