use super::scope::{Bound, Hidden, Names, Reads};
use super::{Pattern, Place};
use oxigraph::model::NamedNode;
use spargebra::algebra::{Expression, GraphPattern};
use std::collections::HashMap;

pub(super) type Known = HashMap<String, Vec<NamedNode>>;

/// The conjuncts of the filters around the pattern being lowered, each of which every
/// row it produces must satisfy, and what each key lowered under them reads.
///
/// A conjunct that equates a pattern's object with an expression makes the pattern a
/// lookup by that value, which only skips rows the filter rejects, on three conditions:
/// - the equality is a conjunct of a filter that every row of the pattern must satisfy;
/// - an expression whose value is a string is equal, by `=`, only to the simple literal
///   of that text;
/// - every variable the expression reads is bound in the row, and none is one a filter
///   reads as unbound, so its value is the one the filter will see. Where one is not
///   bound, the pattern is scanned and the filter decides.
#[derive(Default)]
pub(super) struct Hints {
    conjuncts: Vec<Expression>,
    reads: Vec<Reads>,
}

pub(super) struct Mark {
    conjuncts: usize,
    keys: usize,
}

impl Hints {
    pub(super) fn assume<'a>(
        &mut self,
        conjuncts: impl IntoIterator<Item = &'a Expression>,
    ) -> Mark {
        let mark = Mark {
            conjuncts: self.conjuncts.len(),
            keys: self.reads.len(),
        };
        self.conjuncts.extend(conjuncts.into_iter().cloned());
        mark
    }

    pub(super) fn release(&mut self, mark: &Mark) {
        self.conjuncts.truncate(mark.conjuncts);
    }

    pub(super) fn conjuncts(&self) -> &[Expression] {
        &self.conjuncts
    }

    pub(super) fn replace(&mut self, conjuncts: Vec<Expression>) -> Vec<Expression> {
        std::mem::replace(&mut self.conjuncts, conjuncts)
    }

    pub(super) fn record(&mut self, reads: Reads) {
        self.reads.push(reads);
    }

    /// What a filter around the group equates the pattern's object with, where the
    /// pattern is a lookup by predicate, nothing else of it is known, and every
    /// variable the expression reads is bound.
    pub(super) fn lookup(
        &self,
        names: &mut Names,
        pattern: &Pattern,
        bound: &Bound,
    ) -> Option<Expression> {
        let (Place::Const(_), Place::Var(slot)) = (&pattern.predicate, &pattern.object) else {
            return None;
        };
        if pattern.subject.is_known(&bound.certain) || bound.maybe.contains(*slot) {
            return None;
        }
        for hint in &self.conjuncts {
            let Expression::Equal(a, b) = hint else {
                continue;
            };
            for (variable, other) in [(a, b), (b, a)] {
                let Expression::Variable(variable) = &**variable else {
                    continue;
                };
                if names.slot(variable) == *slot && names.read_by(other).is_subset_of(&bound.maybe)
                {
                    return Some((**other).clone());
                }
            }
        }
        None
    }

    /// A key that reads a variable a filter under which it was lowered reads as unbound
    /// would look up a value the filter does not see.
    pub(super) fn keep_clear(&self, mark: &Mark, unbound: &Hidden) -> Result<(), &'static str> {
        if self.reads[mark.keys..]
            .iter()
            .any(|reads| !reads.is_disjoint_from(unbound))
        {
            return Err("a key reads a variable a filter reads as unbound");
        }
        Ok(())
    }
}

/// The IRIs a variable takes in every row of a pattern, where each row binds it to
/// one of a few that the pattern names.
pub(super) fn known_values(pattern: &GraphPattern) -> Known {
    match pattern {
        GraphPattern::Extend {
            inner,
            variable,
            expression: Expression::NamedNode(node),
        } => {
            let mut known = known_values(inner);
            known.insert(variable.as_str().to_owned(), vec![node.clone()]);
            known
        }
        GraphPattern::Extend { inner, .. } | GraphPattern::Filter { inner, .. } => {
            known_values(inner)
        }
        GraphPattern::Join { left, right } => {
            let mut known = known_values(left);
            known.extend(known_values(right));
            known
        }
        GraphPattern::Union { left, right } => {
            let right = known_values(right);
            known_values(left)
                .into_iter()
                .filter_map(|(name, mut nodes)| {
                    nodes.extend(right.get(&name)?.iter().cloned());
                    Some((name, nodes))
                })
                .collect()
        }
        _ => Known::new(),
    }
}

/// What a filter's conjunct says of the rows of a pattern in which some variables are
/// known: a conditional whose condition the known IRIs decide is its branch.
pub(super) fn settled(conjunct: &Expression, known: &Known) -> Expression {
    match conjunct {
        Expression::If(condition, then, otherwise) => match decided(condition, known) {
            Some(true) => settled(then, known),
            Some(false) => settled(otherwise, known),
            None => conjunct.clone(),
        },
        _ => conjunct.clone(),
    }
}

fn decided(condition: &Expression, known: &Known) -> Option<bool> {
    let (Expression::Equal(a, b) | Expression::SameTerm(a, b)) = condition else {
        return None;
    };
    let ((Expression::Variable(variable), Expression::NamedNode(node))
    | (Expression::NamedNode(node), Expression::Variable(variable))) = (&**a, &**b)
    else {
        return None;
    };
    let nodes = known.get(variable.as_str())?;
    if nodes.iter().all(|known| known == node) {
        Some(true)
    } else if nodes.iter().all(|known| known != node) {
        Some(false)
    } else {
        None
    }
}
