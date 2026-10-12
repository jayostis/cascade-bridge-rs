use super::run::Compiled;
use super::{Aggregate, Group, Kind, Pattern, Place, Plan, Reach, Step};
use oxigraph::model::vocab::xsd;
use oxigraph::model::{BlankNode, Literal, Term, Variable};
use oxiri::Iri;
use spargebra::algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, PropertyPathExpression,
};
use spargebra::term::{GroundTerm, NamedNodePattern, TermPattern, TriplePattern};
use spargebra::Query;
use std::collections::{BTreeSet, HashMap};

pub(super) type Slots = BTreeSet<usize>;

const UNREACHABLE: u8 = u8::MAX;

/// What a CONSTRUCT query lowers to, or why it does not: the general evaluator then
/// answers it.
pub(crate) fn compile(query: &Query) -> Result<Compiled, String> {
    let Query::Construct {
        template,
        pattern,
        base_iri,
        ..
    } = query
    else {
        return Err("it is not a CONSTRUCT".to_owned());
    };
    let mut lowering = Lowering {
        next: 0,
        scopes: vec![HashMap::new()],
        base: base_iri.clone(),
    };
    let mut built = Built::default();
    lowering.pattern(pattern, &mut Frame::default(), &mut built)?;
    let mut blanks = Vec::new();
    let template = template
        .iter()
        .map(|triple| lowering.template(triple, &mut blanks))
        .collect::<Result<_, _>>()?;
    Ok(Compiled {
        plan: built.steps,
        template,
        blanks: blanks.len(),
        width: lowering.next,
    })
}

/// A plan with, after each step, the slots certainly bound there.
#[derive(Default)]
struct Built {
    steps: Plan,
    after: Vec<Slots>,
}

impl Built {
    fn push(&mut self, step: Step, certain: &Slots) {
        self.steps.push(step);
        self.after.push(certain.clone());
    }
}

/// What a group of patterns has bound so far. `outer` is what may be bound by the
/// groups around it, which it does not see: a variable it reads that only they bind
/// would be unbound here, and a plan that carries a row inward would see it.
#[derive(Default)]
struct Frame {
    certain: Slots,
    maybe: Slots,
    own: Slots,
    outer: Slots,
}

impl Frame {
    fn child(&self) -> Self {
        Self {
            certain: self.certain.clone(),
            maybe: self.maybe.clone(),
            own: Slots::new(),
            outer: self.maybe.clone(),
        }
    }

    fn absorb(&mut self, child: Self) {
        self.certain = child.certain;
        self.maybe = child.maybe;
        self.own.extend(child.own);
    }

    fn binds(&mut self, slot: usize, certainly: bool) {
        self.maybe.insert(slot);
        if certainly {
            self.certain.insert(slot);
            self.own.insert(slot);
        }
    }

    fn reads(&self, slots: &Slots) -> Result<(), String> {
        match slots
            .iter()
            .find(|slot| self.outer.contains(slot) && !self.own.contains(slot))
        {
            Some(_) => Err("a group reads a variable only the groups around it bind".to_owned()),
            None => Ok(()),
        }
    }
}

pub(super) struct Lowering {
    next: usize,
    scopes: Vec<HashMap<String, usize>>,
    pub(super) base: Option<Iri<String>>,
}

impl Lowering {
    pub(super) fn slot(&mut self, variable: &Variable) -> usize {
        let name = variable.as_str();
        if let Some(&slot) = self.scopes.last().and_then(|scope| scope.get(name)) {
            return slot;
        }
        let slot = self.next;
        self.next += 1;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), slot);
        }
        slot
    }

    pub(super) fn exists(&mut self, body: &GraphPattern, bound: &Slots) -> Result<Plan, String> {
        let mut frame = Frame {
            certain: bound.clone(),
            maybe: bound.clone(),
            ..Frame::default()
        };
        let mut built = Built::default();
        self.pattern(body, &mut frame, &mut built)?;
        Ok(built.steps)
    }

    fn pattern(
        &mut self,
        pattern: &GraphPattern,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        match pattern {
            GraphPattern::Bgp { patterns } => self.triples(patterns, frame, out),
            GraphPattern::Join { left, right } => {
                self.pattern(left, frame, out)?;
                let mut child = frame.child();
                self.pattern(right, &mut child, out)?;
                frame.absorb(child);
                Ok(())
            }
            GraphPattern::Filter { expr, inner } => self.filter(expr, inner, frame, out),
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => {
                self.pattern(inner, frame, out)?;
                let slot = self.slot(variable);
                if frame.maybe.contains(&slot) {
                    return Err(format!(
                        "a BIND of ?{} where a group around it binds it",
                        variable.as_str()
                    ));
                }
                let mut read = Slots::new();
                self.mentions(expression, &mut read, None);
                frame.reads(&read)?;
                let expr = self.expression(expression, &frame.certain)?;
                let constant = matches!(
                    expression,
                    Expression::NamedNode(_) | Expression::Literal(_)
                );
                frame.binds(slot, constant);
                out.push(Step::Bind(slot, expr), &frame.certain);
                Ok(())
            }
            GraphPattern::Union { .. } => {
                let mut branches = Vec::new();
                union_branches(pattern, &mut branches);
                let mut plans = Vec::new();
                let mut certain: Option<Slots> = None;
                let mut own: Option<Slots> = None;
                let mut maybe = Slots::new();
                for branch in branches {
                    let mut child = frame.child();
                    let mut built = Built::default();
                    self.pattern(branch, &mut child, &mut built)?;
                    plans.push(built.steps);
                    maybe.extend(child.maybe);
                    certain = Some(intersect(certain, child.certain));
                    own = Some(intersect(own, child.own));
                }
                frame.maybe.extend(maybe);
                frame.certain = certain.unwrap_or_default();
                frame.own.extend(own.unwrap_or_default());
                out.push(Step::Union(plans), &frame.certain);
                Ok(())
            }
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                self.pattern(left, frame, out)?;
                let mut read = Slots::new();
                self.variables(right, &mut read);
                if let Some(expression) = expression {
                    self.mentions(expression, &mut read, None);
                }
                frame.reads(&read)?;
                let mut child = frame.child();
                let mut built = Built::default();
                self.pattern(right, &mut child, &mut built)?;
                let condition = expression
                    .as_ref()
                    .map(|e| self.expression(e, &child.certain))
                    .transpose()?;
                frame.maybe.extend(child.maybe);
                out.push(Step::Optional(built.steps, condition), &frame.certain);
                Ok(())
            }
            GraphPattern::Values {
                variables,
                bindings,
            } => {
                let slots: Vec<usize> = variables.iter().map(|v| self.slot(v)).collect();
                let table = bindings
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|cell| match cell {
                                None => Ok(None),
                                Some(GroundTerm::NamedNode(n)) => Ok(Some(Term::from(n.clone()))),
                                Some(GroundTerm::Literal(l)) if by_term(l) => {
                                    Ok(Some(Term::from(l.clone())))
                                }
                                Some(GroundTerm::Literal(_)) => {
                                    Err("a typed literal in VALUES, which a join matches by value"
                                        .to_owned())
                                }
                                Some(_) => Err("a quoted triple in VALUES".to_owned()),
                            })
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                for (at, &slot) in slots.iter().enumerate() {
                    frame.binds(slot, table.iter().all(|row| row[at].is_some()));
                }
                out.push(Step::Values(slots, table), &frame.certain);
                Ok(())
            }
            GraphPattern::Project { inner, variables } => {
                self.subquery(inner, variables, false, frame, out)
            }
            GraphPattern::Distinct { inner } => match &**inner {
                GraphPattern::Project { inner, variables } => {
                    self.subquery(inner, variables, true, frame, out)
                }
                _ => Err("DISTINCT outside a SELECT".to_owned()),
            },
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => {
                let mut child = frame.child();
                let mut built = Built::default();
                self.pattern(inner, &mut child, &mut built)?;
                let keys: Vec<usize> = variables.iter().map(|v| self.slot(v)).collect();
                let mut specs = Vec::new();
                for (variable, aggregate) in aggregates {
                    let slot = self.slot(variable);
                    if frame.maybe.contains(&slot) {
                        return Err("an aggregate named for a variable already bound".to_owned());
                    }
                    specs.push(Aggregate {
                        slot,
                        kind: self.aggregate(aggregate, &child)?,
                    });
                }
                for &slot in &keys {
                    frame.binds(slot, child.certain.contains(&slot));
                }
                for aggregate in &specs {
                    frame.maybe.insert(aggregate.slot);
                }
                out.push(
                    Step::Group(Group {
                        inner: built.steps,
                        keys,
                        aggregates: specs,
                    }),
                    &frame.certain,
                );
                Ok(())
            }
            GraphPattern::Path {
                subject,
                path,
                object,
            } => {
                let (PropertyPathExpression::ZeroOrMore(edges)
                | PropertyPathExpression::OneOrMore(edges)) = path
                else {
                    return Err("a property path that is not a walk".to_owned());
                };
                let PropertyPathExpression::NegatedPropertySet(skipped) = &**edges else {
                    return Err(
                        "a property path that is not a walk over all but some edges".to_owned()
                    );
                };
                let (from, to) = (self.place(subject)?, self.place(object)?);
                if !matches!(&from, Place::Const(_))
                    && !matches!(&from, Place::Var(s) if frame.certain.contains(s))
                {
                    return Err("a property path from a node not yet bound".to_owned());
                }
                if let Place::Var(slot) = to {
                    frame.binds(slot, true);
                }
                let step = Step::Reach(Reach {
                    from,
                    to,
                    skipped: skipped.iter().map(|n| Term::from(n.clone())).collect(),
                    from_itself: matches!(path, PropertyPathExpression::ZeroOrMore(_)),
                });
                out.push(step, &frame.certain);
                Ok(())
            }
            GraphPattern::Minus { .. } => Err("MINUS".to_owned()),
            GraphPattern::Reduced { .. } => Err("REDUCED".to_owned()),
            GraphPattern::Slice { .. } => Err("LIMIT or OFFSET".to_owned()),
            GraphPattern::OrderBy { .. } => Err("ORDER BY".to_owned()),
            GraphPattern::Graph { .. } => Err("GRAPH".to_owned()),
            GraphPattern::Service { .. } => Err("SERVICE".to_owned()),
            GraphPattern::Lateral { .. } => Err("LATERAL".to_owned()),
        }
    }

    /// A subquery shares only the variables it projects with the group around it. Its
    /// rows are those of its inner pattern, or, where DISTINCT, one for each projection.
    fn subquery(
        &mut self,
        inner: &GraphPattern,
        variables: &[Variable],
        distinct: bool,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        let shared: HashMap<String, usize> = variables
            .iter()
            .map(|v| (v.as_str().to_owned(), self.slot(v)))
            .collect();
        let keys: Vec<usize> = variables.iter().map(|v| self.slot(v)).collect();
        self.scopes.push(shared);
        let mut child = frame.child();
        let mut built = Built::default();
        let lowered = self.pattern(inner, &mut child, if distinct { &mut built } else { out });
        self.scopes.pop();
        lowered?;
        if distinct {
            for &slot in &keys {
                frame.binds(slot, child.certain.contains(&slot));
            }
            let step = Step::Group(Group {
                inner: built.steps,
                keys,
                aggregates: Vec::new(),
            });
            out.push(step, &frame.certain);
        } else {
            frame.absorb(child);
        }
        Ok(())
    }

    fn triples(
        &mut self,
        patterns: &[TriplePattern],
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        let mut pending = patterns
            .iter()
            .map(|triple| self.triple(triple))
            .collect::<Result<Vec<_>, _>>()?;
        if pending
            .iter()
            .any(|t| !exact(&t.subject) || !exact(&t.object))
        {
            return Err("a typed literal in a pattern, which a store matches by value".to_owned());
        }
        while !pending.is_empty() {
            let (at, rank) = pending
                .iter()
                .enumerate()
                .map(|(at, triple)| (at, triple.rank(&frame.certain)))
                .min_by_key(|&(at, rank)| (rank, at))
                .unwrap_or_default();
            if rank == UNREACHABLE {
                return Err("a triple pattern with nothing bound to start from".to_owned());
            }
            let triple = pending.remove(at);
            for place in [&triple.subject, &triple.predicate, &triple.object] {
                if let Place::Var(slot) = place {
                    frame.binds(*slot, true);
                }
            }
            out.push(Step::Triple(triple), &frame.certain);
        }
        Ok(())
    }

    fn triple(&mut self, triple: &TriplePattern) -> Result<Pattern, String> {
        Ok(Pattern {
            subject: self.place(&triple.subject)?,
            predicate: match &triple.predicate {
                NamedNodePattern::NamedNode(node) => Place::Const(Term::from(node.clone())),
                NamedNodePattern::Variable(variable) => Place::Var(self.slot(variable)),
            },
            object: self.place(&triple.object)?,
        })
    }

    fn place(&mut self, term: &TermPattern) -> Result<Place, String> {
        match term {
            TermPattern::NamedNode(node) => Ok(Place::Const(Term::from(node.clone()))),
            TermPattern::Literal(literal) => Ok(Place::Const(Term::from(literal.clone()))),
            TermPattern::Variable(variable) => Ok(Place::Var(self.slot(variable))),
            _ => Err("a blank node or a quoted triple in a pattern".to_owned()),
        }
    }

    fn template(
        &mut self,
        triple: &TriplePattern,
        blanks: &mut Vec<BlankNode>,
    ) -> Result<Pattern, String> {
        let subject = self.constructed(&triple.subject, blanks)?;
        let object = self.constructed(&triple.object, blanks)?;
        let predicate = match &triple.predicate {
            NamedNodePattern::NamedNode(node) => Place::Const(Term::from(node.clone())),
            NamedNodePattern::Variable(variable) => Place::Var(self.slot(variable)),
        };
        Ok(Pattern {
            subject,
            predicate,
            object,
        })
    }

    fn constructed(
        &mut self,
        term: &TermPattern,
        blanks: &mut Vec<BlankNode>,
    ) -> Result<Place, String> {
        match term {
            TermPattern::BlankNode(label) => {
                let at = blanks.iter().position(|b| b == label).unwrap_or_else(|| {
                    blanks.push(label.clone());
                    blanks.len() - 1
                });
                Ok(Place::Blank(at))
            }
            term => self.place(term),
        }
    }

    fn filter(
        &mut self,
        expr: &Expression,
        inner: &GraphPattern,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        let start = out.steps.len();
        let entry = frame.certain.clone();
        self.pattern(inner, frame, out)?;
        let mut group = Slots::new();
        self.variables(inner, &mut group);
        let mut conjuncts = Vec::new();
        conjunction(expr, &mut conjuncts);
        let mut placed = Vec::new();
        for conjunct in conjuncts {
            let mut read = Slots::new();
            self.mentions(conjunct, &mut read, None);
            frame.reads(&read)?;
            let mut needed = Slots::new();
            self.mentions(conjunct, &mut needed, Some(&group));
            let steps = out.steps.len() - start;
            let certain_after = |k: usize| {
                if k == 0 {
                    &entry
                } else {
                    &out.after[start + k - 1]
                }
            };
            let at = (0..=steps)
                .find(|&k| needed.is_subset(certain_after(k)))
                .unwrap_or(steps);
            let certain = certain_after(at).clone();
            placed.push((start + at, self.expression(conjunct, &certain)?, certain));
        }
        placed.sort_by_key(|(at, ..)| *at);
        for (inserted, (at, expr, certain)) in placed.into_iter().enumerate() {
            out.steps.insert(at + inserted, Step::Filter(expr));
            out.after.insert(at + inserted, certain);
        }
        Ok(())
    }

    fn aggregate(
        &mut self,
        aggregate: &AggregateExpression,
        group: &Frame,
    ) -> Result<Kind, String> {
        let (name, expr, distinct) = match aggregate {
            AggregateExpression::CountSolutions { distinct: false } => {
                return Ok(Kind::Count {
                    of: None,
                    distinct: false,
                })
            }
            AggregateExpression::CountSolutions { .. } => {
                return Err("COUNT(DISTINCT *)".to_owned())
            }
            AggregateExpression::FunctionCall {
                name,
                expr,
                distinct,
            } => (name, expr, *distinct),
        };
        if let (AggregateFunction::Count, Expression::Variable(variable)) = (name, expr) {
            return Ok(Kind::Count {
                of: Some(self.slot(variable)),
                distinct,
            });
        }
        if distinct {
            return Err("DISTINCT in an aggregate".to_owned());
        }
        let mut read = Slots::new();
        self.mentions(expr, &mut read, None);
        group.reads(&read)?;
        let operand = self.expression(expr, &group.certain)?;
        match name {
            AggregateFunction::Sum => Ok(Kind::Sum(operand)),
            AggregateFunction::Min => Ok(Kind::Min(operand)),
            AggregateFunction::Max => Ok(Kind::Max(operand)),
            AggregateFunction::Sample => Ok(Kind::Sample(operand)),
            _ => Err("an aggregate other than COUNT, SUM, MIN, MAX or SAMPLE".to_owned()),
        }
    }

    /// Every variable the pattern shows to the group around it. Where `only` is
    /// given, the variables inside an EXISTS count only if the group holds them.
    fn variables(&mut self, pattern: &GraphPattern, into: &mut Slots) {
        match pattern {
            GraphPattern::Bgp { patterns } => {
                for triple in patterns {
                    for term in [&triple.subject, &triple.object] {
                        if let TermPattern::Variable(v) = term {
                            into.insert(self.slot(v));
                        }
                    }
                    if let NamedNodePattern::Variable(v) = &triple.predicate {
                        into.insert(self.slot(v));
                    }
                }
            }
            GraphPattern::Join { left, right }
            | GraphPattern::Union { left, right }
            | GraphPattern::Minus { left, right }
            | GraphPattern::Lateral { left, right } => {
                self.variables(left, into);
                self.variables(right, into);
            }
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                self.variables(left, into);
                self.variables(right, into);
                if let Some(expression) = expression {
                    self.mentions(expression, into, None);
                }
            }
            GraphPattern::Filter { expr, inner } => {
                self.mentions(expr, into, None);
                self.variables(inner, into);
            }
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => {
                into.insert(self.slot(variable));
                self.mentions(expression, into, None);
                self.variables(inner, into);
            }
            GraphPattern::Values { variables, .. } => {
                into.extend(variables.iter().map(|v| self.slot(v)));
            }
            GraphPattern::Project { variables, .. } => {
                into.extend(variables.iter().map(|v| self.slot(v)));
            }
            GraphPattern::Group {
                variables,
                aggregates,
                ..
            } => {
                into.extend(variables.iter().map(|v| self.slot(v)));
                into.extend(aggregates.iter().map(|(v, _)| self.slot(v)));
            }
            GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Graph { inner, .. }
            | GraphPattern::Service { inner, .. } => self.variables(inner, into),
            GraphPattern::Path { .. } => {}
        }
    }

    /// Every variable the expression reads. A variable only inside an EXISTS counts
    /// where `group` is none, or where the group around the filter also holds it.
    fn mentions(&mut self, expression: &Expression, into: &mut Slots, group: Option<&Slots>) {
        match expression {
            Expression::Variable(v) | Expression::Bound(v) => {
                into.insert(self.slot(v));
            }
            Expression::Exists(body) => {
                let mut inside = Slots::new();
                self.variables(body, &mut inside);
                into.extend(
                    inside
                        .into_iter()
                        .filter(|s| group.is_none_or(|g| g.contains(s))),
                );
            }
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
            | Expression::Divide(a, b) => {
                self.mentions(a, into, group);
                self.mentions(b, into, group);
            }
            Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                self.mentions(a, into, group);
            }
            Expression::In(a, list) => {
                self.mentions(a, into, group);
                for e in list {
                    self.mentions(e, into, group);
                }
            }
            Expression::If(a, b, c) => {
                for e in [a, b, c] {
                    self.mentions(e, into, group);
                }
            }
            Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
                for e in list {
                    self.mentions(e, into, group);
                }
            }
            Expression::NamedNode(_) | Expression::Literal(_) => {}
        }
    }
}

impl Pattern {
    fn rank(&self, bound: &Slots) -> u8 {
        let known = |place: &Place| match place {
            Place::Const(_) => true,
            Place::Var(slot) => bound.contains(slot),
            Place::Blank(_) => false,
        };
        match (
            known(&self.subject),
            known(&self.predicate),
            known(&self.object),
        ) {
            (true, _, _) => 0,
            (false, true, true) => 1,
            (false, _, true) => 2,
            (false, true, false) => 3,
            (false, false, false) => UNREACHABLE,
        }
    }
}

/// Whether a place is matched by its term: a typed literal is matched by its value.
fn exact(place: &Place) -> bool {
    !matches!(place, Place::Const(Term::Literal(l)) if !by_term(l))
}

fn by_term(literal: &Literal) -> bool {
    literal.datatype() == xsd::STRING || literal.language().is_some()
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

fn intersect(so_far: Option<Slots>, next: Slots) -> Slots {
    match so_far {
        Some(slots) => slots.intersection(&next).copied().collect(),
        None => next,
    }
}
