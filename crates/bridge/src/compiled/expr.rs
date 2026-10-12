use super::builtin::{self, Call};
use super::lower::{Lowering, Slots};
use super::{run, Eval, Expr, Fail, Row, Tree};
use oxigraph::model::{Term, Variable};
use oxsdatatypes::{Boolean, Decimal, Integer};
use spareval::ExpressionTerm as T;
use spargebra::algebra::{Expression, Function, GraphPattern};
use std::cmp::Ordering;

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

pub(super) fn holds(expr: &Expr, tree: &Tree, row: &Row) -> Result<bool, Fail> {
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
        _ => compare(a, b).map_err(|_| Fail::GiveUp),
    }
}

pub(super) fn eval_fn(f: impl Fn(&Tree, &Row) -> Eval + Send + Sync + 'static) -> Expr {
    Box::new(f)
}

fn constant(term: T) -> Expr {
    eval_fn(move |_, _| Ok(term.clone()))
}

pub(super) fn unary(a: Expr, f: impl Fn(T) -> Eval + Send + Sync + 'static) -> Expr {
    eval_fn(move |tree, row| f(a(tree, row)?))
}

pub(super) fn binary(a: Expr, b: Expr, f: impl Fn(T, T) -> Eval + Send + Sync + 'static) -> Expr {
    eval_fn(move |tree, row| f(a(tree, row)?, b(tree, row)?))
}

pub(super) fn text(value: String) -> Eval {
    Ok(T::StringLiteral(value))
}

pub(super) fn boolean(value: bool) -> Eval {
    Ok(T::from(value))
}

/// OR and AND: a decided operand wins over an error, else an error stands.
fn connective(operands: &[&Expr], tree: &Tree, row: &Row, decided_by: bool) -> Eval {
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

fn member(needle: &T, candidates: &[Expr], tree: &Tree, row: &Row) -> Eval {
    let mut failed = false;
    for candidate in candidates {
        match candidate(tree, row).and_then(|c| equals(needle, &c)) {
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
}

fn first_value(candidates: &[Expr], tree: &Tree, row: &Row) -> Eval {
    for candidate in candidates {
        match candidate(tree, row) {
            Err(Fail::Error) => {}
            outcome => return outcome,
        }
    }
    Err(Fail::Error)
}

impl Lowering {
    pub(super) fn expression(
        &mut self,
        expression: &Expression,
        bound: &Slots,
    ) -> Result<Expr, String> {
        match expression {
            Expression::NamedNode(node) => Ok(constant(T::NamedNode(node.clone()))),
            Expression::Literal(value) => Ok(constant(T::from(Term::from(value.clone())))),
            Expression::Variable(variable) => Ok(self.variable(variable)),
            Expression::Bound(variable) => Ok(self.is_bound(variable)),
            Expression::Exists(body) => self.exists_test(body, bound),
            Expression::Or(a, b) => self.logical(a, b, bound, true),
            Expression::And(a, b) => self.logical(a, b, bound, false),
            Expression::Not(a) => {
                let a = self.expression(a, bound)?;
                Ok(unary(a, |a| boolean(!truth(&a)?)))
            }
            Expression::Equal(a, b) => self.binary_of(a, b, bound, |a, b| boolean(equals(&a, &b)?)),
            Expression::SameTerm(a, b) => self.binary_of(a, b, bound, |a, b| boolean(a == b)),
            Expression::Greater(a, b) => self.ordering(a, b, bound, Ordering::is_gt),
            Expression::GreaterOrEqual(a, b) => self.ordering(a, b, bound, Ordering::is_ge),
            Expression::Less(a, b) => self.ordering(a, b, bound, Ordering::is_lt),
            Expression::LessOrEqual(a, b) => self.ordering(a, b, bound, Ordering::is_le),
            Expression::In(a, list) => self.membership(a, list, bound),
            Expression::Add(a, b) => {
                self.integers(a, b, bound, |a, b| a.checked_add(b).map(T::IntegerLiteral))
            }
            Expression::Subtract(a, b) => {
                self.integers(a, b, bound, |a, b| a.checked_sub(b).map(T::IntegerLiteral))
            }
            Expression::Multiply(a, b) => {
                self.integers(a, b, bound, |a, b| a.checked_mul(b).map(T::IntegerLiteral))
            }
            Expression::Divide(a, b) => self.integers(a, b, bound, |a, b| {
                Decimal::from(a).checked_div(b).map(T::DecimalLiteral)
            }),
            Expression::If(c, a, b) => self.conditional(c, a, b, bound),
            Expression::Coalesce(list) => {
                let list = self.operands(list, bound)?;
                Ok(eval_fn(move |tree, row| first_value(&list, tree, row)))
            }
            Expression::FunctionCall(function, written) => self.call(function, written, bound),
            other => Err(format!("the expression {other}")),
        }
    }

    fn operands(&mut self, list: &[Expression], bound: &Slots) -> Result<Vec<Expr>, String> {
        list.iter().map(|e| self.expression(e, bound)).collect()
    }

    fn variable(&mut self, variable: &Variable) -> Expr {
        let slot = self.slot(variable);
        eval_fn(move |_, row| {
            row[slot]
                .as_deref()
                .map(|term| T::from(term.clone()))
                .ok_or(Fail::Error)
        })
    }

    fn is_bound(&mut self, variable: &Variable) -> Expr {
        let slot = self.slot(variable);
        eval_fn(move |_, row| boolean(row[slot].is_some()))
    }

    fn exists_test(&mut self, body: &GraphPattern, bound: &Slots) -> Result<Expr, String> {
        let plan = self.exists(body, bound)?;
        Ok(eval_fn(move |tree, row| {
            boolean(run::exists(&plan, tree, row)?)
        }))
    }

    fn logical(
        &mut self,
        a: &Expression,
        b: &Expression,
        bound: &Slots,
        decided_by: bool,
    ) -> Result<Expr, String> {
        let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
        Ok(eval_fn(move |tree, row| {
            connective(&[&a, &b], tree, row, decided_by)
        }))
    }

    fn binary_of(
        &mut self,
        a: &Expression,
        b: &Expression,
        bound: &Slots,
        f: impl Fn(T, T) -> Eval + Send + Sync + 'static,
    ) -> Result<Expr, String> {
        let (a, b) = (self.expression(a, bound)?, self.expression(b, bound)?);
        Ok(binary(a, b, f))
    }

    fn ordering(
        &mut self,
        a: &Expression,
        b: &Expression,
        bound: &Slots,
        wanted: fn(Ordering) -> bool,
    ) -> Result<Expr, String> {
        self.binary_of(a, b, bound, move |a, b| boolean(wanted(compare(&a, &b)?)))
    }

    fn integers(
        &mut self,
        a: &Expression,
        b: &Expression,
        bound: &Slots,
        arithmetic: fn(Integer, Integer) -> Option<T>,
    ) -> Result<Expr, String> {
        self.binary_of(a, b, bound, move |a, b| match (a, b) {
            (T::IntegerLiteral(a), T::IntegerLiteral(b)) => arithmetic(a, b).ok_or(Fail::Error),
            _ => Err(Fail::GiveUp),
        })
    }

    fn membership(
        &mut self,
        needle: &Expression,
        list: &[Expression],
        bound: &Slots,
    ) -> Result<Expr, String> {
        let needle = self.expression(needle, bound)?;
        let list = self.operands(list, bound)?;
        Ok(eval_fn(move |tree, row| {
            member(&needle(tree, row)?, &list, tree, row)
        }))
    }

    fn conditional(
        &mut self,
        condition: &Expression,
        then: &Expression,
        otherwise: &Expression,
        bound: &Slots,
    ) -> Result<Expr, String> {
        let condition = self.expression(condition, bound)?;
        let then = self.expression(then, bound)?;
        let otherwise = self.expression(otherwise, bound)?;
        Ok(eval_fn(move |tree, row| {
            if truth(&condition(tree, row)?)? {
                then(tree, row)
            } else {
                otherwise(tree, row)
            }
        }))
    }

    fn call(
        &mut self,
        function: &Function,
        written: &[Expression],
        bound: &Slots,
    ) -> Result<Expr, String> {
        let arguments = self.operands(written, bound)?;
        let builtin = builtin::lookup(function)
            .filter(|builtin| builtin.accepts(arguments.len()))
            .ok_or_else(|| format!("the function {function}"))?;
        builtin.build(Call {
            arguments: arguments.into_iter(),
            written,
            base: self.base.as_ref(),
        })
    }
}
