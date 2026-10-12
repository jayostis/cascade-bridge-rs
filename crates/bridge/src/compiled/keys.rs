use oxigraph::model::NamedNode;
use spargebra::algebra::{Expression, GraphPattern};
use std::collections::HashMap;

pub(super) type Known = HashMap<String, Vec<NamedNode>>;

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
