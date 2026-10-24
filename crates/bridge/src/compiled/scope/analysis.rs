use super::{Bound, Certain, Frame, Hidden, Maybe, Names, Reads, Scope, Slots};
use oxigraph::model::Variable;
use spargebra::algebra::{
    AggregateExpression, Expression, Function, GraphPattern, PropertyPathExpression,
};
use spargebra::term::{GroundTerm, NamedNode, NamedNodePattern, TermPattern, TriplePattern};
use std::collections::HashMap;

pub(crate) type Declined = &'static str;

/// A pattern of the algebra with what is bound around and after each part of it, and the
/// checks that fail where the part stands.
pub(crate) enum Scoped<'q> {
    Declined(Declined),
    Triples(Triples<'q>),
    Join(Box<Scoped<'q>>, Box<Scoped<'q>>),
    Filter(Box<Filter<'q>>),
    Extend(Box<Binding<'q>>),
    Union(Union<'q>),
    Optional(Box<Optional<'q>>),
    Values(Values<'q>),
    Subquery(Box<Subquery<'q>>),
    Group(Box<GroupBy<'q>>),
    Path(Path<'q>),
}

pub(crate) struct Triples<'q> {
    pub(crate) patterns: &'q [TriplePattern],
    pub(crate) entry: Bound,
}

pub(crate) struct Filter<'q> {
    pub(crate) inner: Scoped<'q>,
    pub(crate) conjuncts: Vec<Conjunct<'q>>,
    /// Certain before the group, which a conjunct can already stand after.
    pub(crate) entry: Certain,
    pub(crate) maybe: Maybe,
    /// What the conjuncts read as unbound.
    pub(crate) unbound: Hidden,
}

pub(crate) struct Conjunct<'q> {
    pub(crate) expression: &'q Expression,
    pub(crate) hidden: Result<Hidden, Declined>,
    pub(crate) needed: Reads,
}

pub(crate) struct Binding<'q> {
    pub(crate) inner: Scoped<'q>,
    pub(crate) slot: usize,
    pub(crate) expression: &'q Expression,
    pub(crate) operands: Reads,
    /// A group around it binds the variable.
    pub(crate) joins: bool,
    /// It binds the variable in every row.
    pub(crate) certainly: bool,
    pub(crate) reads: Result<(), Declined>,
    /// What the expression stands among, where the BIND does not follow a group of triples.
    pub(crate) scope: Scope,
    pub(crate) certain_after: Certain,
}

pub(crate) struct Union<'q> {
    pub(crate) branches: Vec<Branch<'q>>,
    pub(crate) certain_after: Certain,
}

pub(crate) struct Branch<'q> {
    pub(crate) pattern: &'q GraphPattern,
    pub(crate) scoped: Scoped<'q>,
}

pub(crate) struct Optional<'q> {
    pub(crate) left: Scoped<'q>,
    pub(crate) reads: Result<(), Declined>,
    pub(crate) right: Scoped<'q>,
    pub(crate) condition: Option<(&'q Expression, Scope)>,
    pub(crate) certain_after: Certain,
}

pub(crate) struct Values<'q> {
    pub(crate) slots: Vec<usize>,
    pub(crate) bindings: &'q [Vec<Option<GroundTerm>>],
    pub(crate) certain_after: Certain,
}

pub(crate) struct Subquery<'q> {
    /// Its variables as it names them; those it projects are shared with the group
    /// around it.
    pub(crate) names: HashMap<String, usize>,
    pub(crate) inner: Scoped<'q>,
    pub(crate) keys: Vec<usize>,
    pub(crate) distinct: Option<Distinct>,
}

pub(crate) struct Distinct {
    pub(crate) reads: Result<(), Declined>,
    pub(crate) certain_after: Certain,
}

pub(crate) struct GroupBy<'q> {
    pub(crate) inner: Scoped<'q>,
    pub(crate) keys: Vec<usize>,
    pub(crate) aggregates: Vec<Aggregated<'q>>,
    /// What the aggregates' operands stand among.
    pub(crate) scope: Scope,
    pub(crate) reads: Result<(), Declined>,
    pub(crate) certain_after: Certain,
}

pub(crate) struct Aggregated<'q> {
    pub(crate) slot: usize,
    pub(crate) expression: &'q AggregateExpression,
    /// The variable is not one the group around it binds.
    pub(crate) fresh: Result<(), Declined>,
    pub(crate) reads: Result<(), Declined>,
}

pub(crate) struct Path<'q> {
    pub(crate) subject: &'q TermPattern,
    pub(crate) path: &'q PropertyPathExpression,
    pub(crate) object: &'q TermPattern,
    /// The edges it may take, each with whether it is taken backwards, where it takes one.
    pub(crate) edges: Option<Vec<(&'q NamedNode, bool)>>,
    /// It starts from a constant or from a node bound in every row, or takes one of
    /// some edges, which may start from either end or neither.
    pub(crate) starts: Result<(), Declined>,
    pub(crate) certain_after: Certain,
}

pub(crate) fn analyze<'q>(
    names: &mut Names,
    pattern: &'q GraphPattern,
    frame: &mut Frame,
) -> Scoped<'q> {
    Analysis { names }.pattern(pattern, frame)
}

struct Analysis<'n> {
    names: &'n mut Names,
}

impl Analysis<'_> {
    fn pattern<'q>(&mut self, pattern: &'q GraphPattern, frame: &mut Frame) -> Scoped<'q> {
        match pattern {
            GraphPattern::Bgp { patterns } => self.triples(patterns, frame),
            GraphPattern::Join { left, right } => {
                let left = self.pattern(left, frame);
                let mut child = frame.child();
                let right = self.pattern(right, &mut child);
                frame.absorb(child);
                Scoped::Join(Box::new(left), Box::new(right))
            }
            GraphPattern::Filter { expr, inner } => self.filter(expr, inner, frame),
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => self.extend(inner, variable, expression, frame),
            GraphPattern::Union { .. } => self.union(pattern, frame),
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => self.optional(left, right, expression.as_ref(), frame),
            GraphPattern::Values {
                variables,
                bindings,
            } => self.values(variables, bindings, frame),
            GraphPattern::Project { inner, .. } | GraphPattern::Distinct { inner }
                if frame.bound.maybe.iter().next().is_some() && binds_blank_nodes(inner) =>
            {
                Scoped::Declined("a subquery that makes blank nodes")
            }
            GraphPattern::Project { inner, variables } => {
                self.subquery(inner, variables, false, frame)
            }
            GraphPattern::Distinct { inner } => match &**inner {
                GraphPattern::Project { inner, variables } => {
                    self.subquery(inner, variables, true, frame)
                }
                _ => Scoped::Declined("DISTINCT outside a SELECT"),
            },
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => self.group(inner, variables, aggregates, frame),
            GraphPattern::Path {
                subject,
                path,
                object,
            } => self.path(subject, path, object, frame),
            GraphPattern::Minus { .. } => Scoped::Declined("MINUS"),
            GraphPattern::Reduced { .. } => Scoped::Declined("REDUCED"),
            GraphPattern::Slice { .. } => Scoped::Declined("LIMIT or OFFSET"),
            GraphPattern::OrderBy { .. } => Scoped::Declined("ORDER BY"),
            GraphPattern::Graph { .. } => Scoped::Declined("GRAPH"),
            GraphPattern::Service { .. } => Scoped::Declined("SERVICE"),
            GraphPattern::Lateral { .. } => Scoped::Declined("LATERAL"),
        }
    }

    fn triples<'q>(&mut self, patterns: &'q [TriplePattern], frame: &mut Frame) -> Scoped<'q> {
        let entry = frame.bound.clone();
        let variable = |term: &'q TermPattern| match term {
            TermPattern::Variable(variable) => Some(variable),
            _ => None,
        };
        for triple in patterns {
            let predicate = match &triple.predicate {
                NamedNodePattern::Variable(variable) => Some(variable),
                NamedNodePattern::NamedNode(_) => None,
            };
            let terms = [
                variable(&triple.subject),
                predicate,
                variable(&triple.object),
            ];
            for variable in terms.into_iter().flatten() {
                let slot = self.names.slot(variable);
                frame.binds(slot, true);
            }
        }
        Scoped::Triples(Triples { patterns, entry })
    }

    fn filter<'q>(
        &mut self,
        expression: &'q Expression,
        inner: &'q GraphPattern,
        frame: &mut Frame,
    ) -> Scoped<'q> {
        let mut written = Vec::new();
        conjunction(expression, &mut written);
        let entry = frame.bound.certain.clone();
        let scoped = self.pattern(inner, frame);
        let in_group = self.names.variables_of(inner);
        let mut unbound = Hidden::new();
        let mut conjuncts = Vec::new();
        for expression in written {
            let hidden = frame.hidden_among(&self.names.read_by(expression));
            if let Ok(hidden) = &hidden {
                unbound.extend(hidden.iter());
            }
            conjuncts.push(Conjunct {
                expression,
                hidden,
                needed: self.names.needed_to_place(expression, &in_group),
            });
        }
        Scoped::Filter(Box::new(Filter {
            inner: scoped,
            conjuncts,
            entry,
            maybe: frame.bound.maybe.clone(),
            unbound,
        }))
    }

    fn extend<'q>(
        &mut self,
        inner: &'q GraphPattern,
        variable: &Variable,
        expression: &'q Expression,
        frame: &mut Frame,
    ) -> Scoped<'q> {
        let inner = self.pattern(inner, frame);
        let (slot, operands) = if matches!(inner, Scoped::Triples(_)) {
            let operands = self.names.read_by(expression);
            (self.names.slot(variable), operands)
        } else {
            let slot = self.names.slot(variable);
            (slot, self.names.read_by(expression))
        };
        let joins = frame.bound.maybe.contains(slot);
        let reads = frame.reads(&operands);
        let scope = frame.scope(Hidden::new());
        let constant = matches!(
            expression,
            Expression::NamedNode(_) | Expression::Literal(_)
        );
        let certainly = joins || constant;
        frame.binds(slot, certainly);
        Scoped::Extend(Box::new(Binding {
            inner,
            slot,
            expression,
            operands,
            joins,
            certainly,
            reads,
            scope,
            certain_after: frame.bound.certain.clone(),
        }))
    }

    fn union<'q>(&mut self, pattern: &'q GraphPattern, frame: &mut Frame) -> Scoped<'q> {
        let mut written = Vec::new();
        union_branches(pattern, &mut written);
        let mut branches = Vec::new();
        let mut certain: Option<Certain> = None;
        let mut own: Option<Certain> = None;
        let mut maybe = Maybe::new();
        let mut touched = Maybe::new();
        for pattern in written {
            let mut child = frame.child();
            let scoped = self.pattern(pattern, &mut child);
            branches.push(Branch { pattern, scoped });
            maybe.extend(child.bound.maybe.iter());
            touched.extend(child.touched.iter());
            certain = Some(intersect(certain, child.bound.certain));
            own = Some(intersect(own, child.bound.own));
        }
        frame.bound.maybe.extend(maybe.iter());
        frame.touched.extend(touched.iter());
        frame.bound.certain = certain.unwrap_or_default();
        frame.bound.own.extend(own.unwrap_or_default().iter());
        Scoped::Union(Union {
            branches,
            certain_after: frame.bound.certain.clone(),
        })
    }

    fn optional<'q>(
        &mut self,
        left: &'q GraphPattern,
        right: &'q GraphPattern,
        condition: Option<&'q Expression>,
        frame: &mut Frame,
    ) -> Scoped<'q> {
        let left = self.pattern(left, frame);
        let mut read = self.names.variables_of(right);
        if let Some(condition) = condition {
            read.extend(self.names.read_by(condition).iter());
        }
        let reads = frame.reads(&read);
        let certain_after = frame.bound.certain.clone();
        let mut child = frame.child();
        let right = self.pattern(right, &mut child);
        let condition = condition.map(|e| (e, child.scope(Hidden::new())));
        frame.bound.maybe.extend(child.bound.maybe.iter());
        frame.touched.extend(child.touched.iter());
        Scoped::Optional(Box::new(Optional {
            left,
            reads,
            right,
            condition,
            certain_after,
        }))
    }

    fn values<'q>(
        &mut self,
        variables: &[Variable],
        bindings: &'q [Vec<Option<GroundTerm>>],
        frame: &mut Frame,
    ) -> Scoped<'q> {
        let slots: Vec<usize> = variables.iter().map(|v| self.names.slot(v)).collect();
        for (at, &slot) in slots.iter().enumerate() {
            frame.binds(slot, bindings.iter().all(|row| row[at].is_some()));
        }
        Scoped::Values(Values {
            slots,
            bindings,
            certain_after: frame.bound.certain.clone(),
        })
    }

    fn subquery<'q>(
        &mut self,
        inner: &'q GraphPattern,
        variables: &[Variable],
        distinct: bool,
        frame: &mut Frame,
    ) -> Scoped<'q> {
        let shared: HashMap<String, usize> = variables
            .iter()
            .map(|v| (v.as_str().to_owned(), self.names.slot(v)))
            .collect();
        let keys: Vec<usize> = variables.iter().map(|v| self.names.slot(v)).collect();
        self.names.enter(shared);
        let mut child = frame.child();
        let scoped = self.pattern(inner, &mut child);
        let names = self.names.leave();
        let distinct = distinct.then(|| {
            let reads = child.reads(&keys.iter().copied().collect());
            for &slot in &keys {
                frame.binds(slot, child.bound.own.contains(slot));
            }
            Distinct {
                reads,
                certain_after: frame.bound.certain.clone(),
            }
        });
        if distinct.is_none() {
            frame.absorb(child);
        }
        Scoped::Subquery(Box::new(Subquery {
            names,
            inner: scoped,
            keys,
            distinct,
        }))
    }

    fn group<'q>(
        &mut self,
        inner: &'q GraphPattern,
        variables: &[Variable],
        aggregates: &'q [(Variable, AggregateExpression)],
        frame: &mut Frame,
    ) -> Scoped<'q> {
        let mut child = frame.child();
        let scoped = self.pattern(inner, &mut child);
        let keys: Vec<usize> = variables.iter().map(|v| self.names.slot(v)).collect();
        let aggregates: Vec<Aggregated> = aggregates
            .iter()
            .map(|(variable, expression)| {
                let slot = self.names.slot(variable);
                let reads = match expression {
                    AggregateExpression::FunctionCall { expr, .. } => {
                        child.reads(&self.names.read_by(expr))
                    }
                    AggregateExpression::CountSolutions { .. } => Ok(()),
                };
                Aggregated {
                    slot,
                    expression,
                    fresh: if frame.bound.maybe.contains(slot) {
                        Err("an aggregate named for a variable already bound")
                    } else {
                        Ok(())
                    },
                    reads,
                }
            })
            .collect();
        let reads = child.reads(&keys.iter().copied().collect());
        for &slot in &keys {
            frame.binds(slot, child.bound.own.contains(slot));
        }
        for aggregate in &aggregates {
            frame.maybe_binds(aggregate.slot);
        }
        Scoped::Group(Box::new(GroupBy {
            inner: scoped,
            keys,
            aggregates,
            scope: child.scope(Hidden::new()),
            reads,
            certain_after: frame.bound.certain.clone(),
        }))
    }

    fn path<'q>(
        &mut self,
        subject: &'q TermPattern,
        path: &'q PropertyPathExpression,
        object: &'q TermPattern,
        frame: &mut Frame,
    ) -> Scoped<'q> {
        let edges = edges(path, false);
        let starts = match subject {
            TermPattern::Variable(variable)
                if edges.is_none() && !frame.bound.certain.contains(self.names.slot(variable)) =>
            {
                Err("a property path from a node not yet bound")
            }
            _ => Ok(()),
        };
        let ends = if edges.is_some() {
            vec![subject, object]
        } else {
            vec![object]
        };
        for end in ends {
            if let TermPattern::Variable(variable) = end {
                let slot = self.names.slot(variable);
                frame.binds(slot, true);
            }
        }
        Scoped::Path(Path {
            subject,
            path,
            object,
            edges,
            starts,
            certain_after: frame.bound.certain.clone(),
        })
    }
}

/// The edges a path that takes one edge may take, each with whether it is taken backwards.
fn edges(path: &PropertyPathExpression, backwards: bool) -> Option<Vec<(&NamedNode, bool)>> {
    match path {
        PropertyPathExpression::NamedNode(edge) => Some(vec![(edge, backwards)]),
        PropertyPathExpression::Reverse(inner) => edges(inner, !backwards),
        PropertyPathExpression::Alternative(left, right) => {
            let mut taken = edges(left, backwards)?;
            taken.extend(edges(right, backwards)?);
            Some(taken)
        }
        _ => None,
    }
}

/// Whether an expression calls BNODE, whose node is another for each row: a BIND
/// of one is evaluated after every pattern of its group, as the algebra puts it.
pub(crate) fn makes_blank_nodes(expression: &Expression) -> bool {
    match expression {
        Expression::FunctionCall(Function::BNode, _) => true,
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => makes_blank_nodes(a) || makes_blank_nodes(b),
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            makes_blank_nodes(a)
        }
        Expression::If(a, b, c) => {
            makes_blank_nodes(a) || makes_blank_nodes(b) || makes_blank_nodes(c)
        }
        Expression::In(a, list) => makes_blank_nodes(a) || list.iter().any(makes_blank_nodes),
        Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
            list.iter().any(makes_blank_nodes)
        }
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_)
        | Expression::Exists(_) => false,
    }
}

/// Whether a pattern binds a variable to an expression that calls BNODE. Stock
/// Oxigraph evaluates a subquery once, so such a node is the same for every row it
/// joins, where a plan runs the subquery again for each row before it.
fn binds_blank_nodes(pattern: &GraphPattern) -> bool {
    match pattern {
        GraphPattern::Extend {
            inner, expression, ..
        } => makes_blank_nodes(expression) || binds_blank_nodes(inner),
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            aggregates.iter().any(|(_, aggregate)| {
                matches!(aggregate, AggregateExpression::FunctionCall { expr, .. } if makes_blank_nodes(expr))
            }) || binds_blank_nodes(inner)
        }
        GraphPattern::Join { left, right }
        | GraphPattern::LeftJoin { left, right, .. }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right } => {
            binds_blank_nodes(left) || binds_blank_nodes(right)
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Service { inner, .. } => binds_blank_nodes(inner),
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => false,
    }
}

fn union_branches<'a>(pattern: &'a GraphPattern, into: &mut Vec<&'a GraphPattern>) {
    if let GraphPattern::Union { left, right } = pattern {
        union_branches(left, into);
        union_branches(right, into);
    } else {
        into.push(pattern);
    }
}

fn conjunction<'a>(expression: &'a Expression, into: &mut Vec<&'a Expression>) {
    if let Expression::And(a, b) = expression {
        conjunction(a, into);
        conjunction(b, into);
    } else {
        into.push(expression);
    }
}

fn intersect<Role>(so_far: Option<Slots<Role>>, next: Slots<Role>) -> Slots<Role> {
    match so_far {
        Some(slots) => slots.intersection(&next),
        None => next,
    }
}
