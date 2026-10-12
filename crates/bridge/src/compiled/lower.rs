use super::keys::{known_values, settled};
use super::run::Compiled;
use super::{Aggregate, Group, Key, Kind, Pattern, Place, Plan, Reach, Step};
use oxigraph::model::vocab::xsd;
use oxigraph::model::{BlankNode, Literal, Term, Variable};
use oxiri::Iri;
use spargebra::algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, PropertyPathExpression,
};
use spargebra::term::{GroundTerm, NamedNodePattern, TermPattern, TriplePattern};
use spargebra::Query;
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) type Slots = BTreeSet<usize>;

const UNREACHABLE: u8 = u8::MAX;

static MEMOS: AtomicUsize = AtomicUsize::new(0);

/// A name for an answer that depends on the tree alone, unique across every plan.
pub(super) fn next_memo() -> usize {
    MEMOS.fetch_add(1, Ordering::Relaxed)
}

/// What an expression stands among: what is bound in every row and in some row where
/// it runs, and what the group it belongs to reads as unbound although a group around
/// it binds it.
pub(super) struct Scope {
    pub(super) certain: Slots,
    pub(super) maybe: Slots,
    pub(super) hidden: Slots,
}

impl Scope {
    pub(super) fn visible(&self) -> Slots {
        self.certain.difference(&self.hidden).copied().collect()
    }
}

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
        hints: Vec::new(),
        key_reads: Vec::new(),
    };
    let mut built = Built::default();
    lowering.pattern(pattern, &mut Frame::default(), &mut built)?;
    let mut blanks = Vec::new();
    let template = template
        .iter()
        .map(|triple| lowering.template(triple, &mut blanks))
        .collect::<Result<_, _>>()?;
    Ok(Compiled {
        plan: built.plan(),
        template,
        blanks: blanks.len(),
        width: lowering.next,
    })
}

type Placed = (Step, Slots);

/// Each step with what is certainly bound once it has run.
#[derive(Default)]
struct Built {
    steps: Vec<Placed>,
}

impl Built {
    fn push(&mut self, step: Step, certain: &Slots) {
        self.steps.push((step, certain.clone()));
    }

    fn plan(self) -> Plan {
        self.steps.into_iter().map(|(step, _)| step).collect()
    }
}

/// What a group of patterns has bound so far.
/// - `certain` and `maybe` are what is bound in every row, and in some row, including
///   what the groups around it bind.
/// - `own` is what its own patterns bind in every row, which it would bind alone.
/// - `outer` is what may be bound by the groups around it, which it does not see: a
///   variable it reads that is in `outer` and not in `own` would be unbound here,
///   and a plan that carries a row inward would see the outer value.
/// - `touched` is what its own patterns may bind, in some row or every row.
/// - `row` is what the row an EXISTS body runs on may bind. A general evaluator runs the
///   whole body on that row, nested groups included, so they read these as bound.
#[derive(Default)]
struct Frame {
    certain: Slots,
    maybe: Slots,
    own: Slots,
    outer: Slots,
    touched: Slots,
    row: Slots,
}

const READS_OUTER: &str = "a group reads a variable only the groups around it bind";

impl Frame {
    fn child(&self) -> Self {
        Self {
            certain: self.certain.clone(),
            maybe: self.maybe.clone(),
            own: Slots::new(),
            outer: self.maybe.clone(),
            touched: Slots::new(),
            row: self.row.clone(),
        }
    }

    fn absorb(&mut self, child: Self) {
        self.certain = child.certain;
        self.maybe = child.maybe;
        self.own.extend(child.own);
        self.touched.extend(child.touched);
    }

    fn binds(&mut self, slot: usize, certainly: bool) {
        self.maybe_binds(slot);
        if certainly {
            self.certain.insert(slot);
            self.own.insert(slot);
        }
    }

    fn maybe_binds(&mut self, slot: usize) {
        self.maybe.insert(slot);
        self.touched.insert(slot);
    }

    fn reads(&self, slots: &Slots) -> Result<(), String> {
        match slots
            .iter()
            .find(|slot| self.outer.contains(slot) && !self.own.contains(slot))
        {
            Some(_) => Err(READS_OUTER.to_owned()),
            None => Ok(()),
        }
    }

    /// What a filter of this group reads that only the groups around it bind: a
    /// general evaluator keeps the group apart from them, so there it is unbound.
    /// Where the group may bind it as well, or an EXISTS body's row binds it, the row
    /// cannot say which it is.
    fn hidden_among(&self, slots: &Slots) -> Result<Slots, String> {
        let outside = |slot: &&usize| self.outer.contains(slot) && !self.own.contains(slot);
        let mut hidden = Slots::new();
        for slot in slots.iter().filter(outside) {
            if self.touched.contains(slot) || self.row.contains(slot) {
                return Err(READS_OUTER.to_owned());
            }
            hidden.insert(*slot);
        }
        Ok(hidden)
    }

    fn scope(&self, hidden: Slots) -> Scope {
        Scope {
            certain: self.certain.clone(),
            maybe: self.maybe.clone(),
            hidden,
        }
    }
}

pub(super) struct Lowering {
    next: usize,
    scopes: Vec<HashMap<String, usize>>,
    pub(super) base: Option<Iri<String>>,
    /// The conjuncts of the filters around the pattern being lowered, each of which
    /// every row it produces must satisfy.
    pub(super) hints: Vec<Expression>,
    /// What each key lowered so far reads, which a filter checks against what it
    /// reads as unbound.
    key_reads: Vec<Slots>,
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

    /// Runs `lower` with the filters around the pattern set aside: a row of an OPTIONAL's
    /// right side, a subquery, a group or an EXISTS need not satisfy them.
    fn apart<T>(&mut self, lower: impl FnOnce(&mut Self) -> T) -> T {
        let hints = std::mem::take(&mut self.hints);
        let lowered = lower(self);
        self.hints = hints;
        lowered
    }

    pub(super) fn exists(&mut self, body: &GraphPattern, scope: &Scope) -> Result<Plan, String> {
        let row: Slots = scope.maybe.difference(&scope.hidden).copied().collect();
        let mut frame = Frame {
            certain: scope.visible(),
            maybe: row.clone(),
            row,
            ..Frame::default()
        };
        let mut built = Built::default();
        self.apart(|me| me.pattern(body, &mut frame, &mut built))?;
        Ok(built.plan())
    }

    fn pattern(
        &mut self,
        pattern: &GraphPattern,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        match pattern {
            GraphPattern::Bgp { patterns } => self.triples(patterns, frame, out),
            GraphPattern::Join { left, right } => self.join(left, right, frame, out),
            GraphPattern::Filter { expr, inner } => self.filter(expr, inner, frame, out),
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => self.extend(inner, variable, expression, frame, out),
            GraphPattern::Union { .. } => self.union(pattern, frame, out),
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => self.optional(left, right, expression.as_ref(), frame, out),
            GraphPattern::Values {
                variables,
                bindings,
            } => self.values(variables, bindings, frame, out),
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
            } => self.group(inner, variables, aggregates, frame, out),
            GraphPattern::Path {
                subject,
                path,
                object,
            } => self.walk(subject, path, object, frame, out),
            GraphPattern::Minus { .. } => Err("MINUS".to_owned()),
            GraphPattern::Reduced { .. } => Err("REDUCED".to_owned()),
            GraphPattern::Slice { .. } => Err("LIMIT or OFFSET".to_owned()),
            GraphPattern::OrderBy { .. } => Err("ORDER BY".to_owned()),
            GraphPattern::Graph { .. } => Err("GRAPH".to_owned()),
            GraphPattern::Service { .. } => Err("SERVICE".to_owned()),
            GraphPattern::Lateral { .. } => Err("LATERAL".to_owned()),
        }
    }

    fn join(
        &mut self,
        left: &GraphPattern,
        right: &GraphPattern,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        self.pattern(left, frame, out)?;
        let mut child = frame.child();
        self.pattern(right, &mut child, out)?;
        frame.absorb(child);
        Ok(())
    }

    fn extend(
        &mut self,
        inner: &GraphPattern,
        variable: &Variable,
        expression: &Expression,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        match inner {
            GraphPattern::Bgp { patterns } => {
                let pending = self.patterns(patterns)?;
                let operands = self.read_by(expression);
                let rest = self.schedule(pending, Some(&operands), frame, out)?;
                self.bind(variable, expression, frame, out)?;
                self.schedule(rest, None, frame, out)?;
            }
            _ => {
                self.pattern(inner, frame, out)?;
                self.bind(variable, expression, frame, out)?;
            }
        }
        Ok(())
    }

    /// Where a group around it binds the variable, a BIND is a join on it.
    fn bind(
        &mut self,
        variable: &Variable,
        expression: &Expression,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        let slot = self.slot(variable);
        let joins = frame.maybe.contains(&slot);
        let read = self.read_by(expression);
        frame.reads(&read)?;
        let expr = self.expression(expression, &frame.scope(Slots::new()))?;
        if joins {
            frame.binds(slot, true);
            out.push(Step::Agree(slot, expr), &frame.certain);
        } else {
            let constant = matches!(
                expression,
                Expression::NamedNode(_) | Expression::Literal(_)
            );
            frame.binds(slot, constant);
            out.push(Step::Bind(slot, expr), &frame.certain);
        }
        Ok(())
    }

    fn union(
        &mut self,
        pattern: &GraphPattern,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        let mut branches = Vec::new();
        union_branches(pattern, &mut branches);
        let mut plans = Vec::new();
        let mut certain: Option<Slots> = None;
        let mut own: Option<Slots> = None;
        let mut maybe = Slots::new();
        let mut touched = Slots::new();
        let hints = self.hints.clone();
        for branch in branches {
            let mut child = frame.child();
            let mut built = Built::default();
            let known = known_values(branch);
            self.hints = hints.iter().map(|hint| settled(hint, &known)).collect();
            self.pattern(branch, &mut child, &mut built)?;
            plans.push(built.plan());
            maybe.extend(child.maybe);
            touched.extend(child.touched);
            certain = Some(intersect(certain, child.certain));
            own = Some(intersect(own, child.own));
        }
        self.hints = hints;
        frame.maybe.extend(maybe);
        frame.touched.extend(touched);
        frame.certain = certain.unwrap_or_default();
        frame.own.extend(own.unwrap_or_default());
        out.push(Step::Union(plans), &frame.certain);
        Ok(())
    }

    fn optional(
        &mut self,
        left: &GraphPattern,
        right: &GraphPattern,
        condition: Option<&Expression>,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        self.pattern(left, frame, out)?;
        let mut read = Slots::new();
        self.variables(right, &mut read);
        if let Some(condition) = condition {
            read.extend(self.read_by(condition));
        }
        frame.reads(&read)?;
        let mut child = frame.child();
        let mut built = Built::default();
        self.apart(|me| me.pattern(right, &mut child, &mut built))?;
        let condition = condition
            .map(|e| self.expression(e, &child.scope(Slots::new())))
            .transpose()?;
        frame.maybe.extend(child.maybe);
        frame.touched.extend(child.touched);
        out.push(Step::Optional(built.plan(), condition), &frame.certain);
        Ok(())
    }

    fn values(
        &mut self,
        variables: &[Variable],
        bindings: &[Vec<Option<GroundTerm>>],
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        let slots: Vec<usize> = variables.iter().map(|v| self.slot(v)).collect();
        let table = bindings
            .iter()
            .map(|row| row.iter().map(value).collect::<Result<Vec<_>, _>>())
            .collect::<Result<Vec<_>, _>>()?;
        for (at, &slot) in slots.iter().enumerate() {
            frame.binds(slot, table.iter().all(|row| row[at].is_some()));
        }
        out.push(Step::Values(slots, table), &frame.certain);
        Ok(())
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
        let into = if distinct { &mut built } else { &mut *out };
        let lowered = self.apart(|me| me.pattern(inner, &mut child, into));
        self.scopes.pop();
        lowered?;
        if distinct {
            child.reads(&keys.iter().copied().collect())?;
            for &slot in &keys {
                frame.binds(slot, child.own.contains(&slot));
            }
            let step = Step::Group(Group {
                inner: built.plan(),
                keys,
                aggregates: Vec::new(),
            });
            out.push(step, &frame.certain);
        } else {
            frame.absorb(child);
        }
        Ok(())
    }

    fn group(
        &mut self,
        inner: &GraphPattern,
        variables: &[Variable],
        aggregates: &[(Variable, AggregateExpression)],
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        let mut child = frame.child();
        let mut built = Built::default();
        self.apart(|me| me.pattern(inner, &mut child, &mut built))?;
        let keys: Vec<usize> = variables.iter().map(|v| self.slot(v)).collect();
        let aggregates = self.aggregates(aggregates, frame, &child)?;
        child.reads(&keys.iter().copied().collect())?;
        for &slot in &keys {
            frame.binds(slot, child.own.contains(&slot));
        }
        for aggregate in &aggregates {
            frame.maybe_binds(aggregate.slot);
        }
        let step = Step::Group(Group {
            inner: built.plan(),
            keys,
            aggregates,
        });
        out.push(step, &frame.certain);
        Ok(())
    }

    fn aggregates(
        &mut self,
        aggregates: &[(Variable, AggregateExpression)],
        frame: &Frame,
        group: &Frame,
    ) -> Result<Vec<Aggregate>, String> {
        aggregates
            .iter()
            .map(|(variable, aggregate)| {
                let slot = self.slot(variable);
                if frame.maybe.contains(&slot) {
                    return Err("an aggregate named for a variable already bound".to_owned());
                }
                Ok(Aggregate {
                    slot,
                    kind: self.aggregate(aggregate, group)?,
                })
            })
            .collect()
    }

    fn aggregate(
        &mut self,
        aggregate: &AggregateExpression,
        group: &Frame,
    ) -> Result<Kind, String> {
        match aggregate {
            AggregateExpression::CountSolutions { distinct: false } => Ok(Kind::Count {
                of: None,
                distinct: false,
            }),
            AggregateExpression::CountSolutions { .. } => Err("COUNT(DISTINCT *)".to_owned()),
            AggregateExpression::FunctionCall {
                name,
                expr,
                distinct,
            } => self.aggregate_call(name, expr, *distinct, group),
        }
    }

    fn aggregate_call(
        &mut self,
        name: &AggregateFunction,
        expr: &Expression,
        distinct: bool,
        group: &Frame,
    ) -> Result<Kind, String> {
        if let (AggregateFunction::Count, Expression::Variable(variable)) = (name, expr) {
            return Ok(Kind::Count {
                of: Some(self.slot(variable)),
                distinct,
            });
        }
        if distinct {
            return Err("DISTINCT in an aggregate".to_owned());
        }
        group.reads(&self.read_by(expr))?;
        let operand = self.expression(expr, &group.scope(Slots::new()))?;
        match name {
            AggregateFunction::Sum => Ok(Kind::Sum(operand)),
            AggregateFunction::Min => Ok(Kind::Min(operand)),
            AggregateFunction::Max => Ok(Kind::Max(operand)),
            AggregateFunction::Sample => Ok(Kind::Sample(operand)),
            _ => Err("an aggregate other than COUNT, SUM, MIN, MAX or SAMPLE".to_owned()),
        }
    }

    fn walk(
        &mut self,
        subject: &TermPattern,
        path: &PropertyPathExpression,
        object: &TermPattern,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        let (skipped, from_itself) = walked(path)?;
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
            skipped,
            from_itself,
        });
        out.push(step, &frame.certain);
        Ok(())
    }

    fn triples(
        &mut self,
        patterns: &[TriplePattern],
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<(), String> {
        let pending = self.patterns(patterns)?;
        self.schedule(pending, None, frame, out)?;
        Ok(())
    }

    /// What a filter around the group equates the pattern's object with, where the
    /// pattern is a lookup by predicate, nothing else of it is known, and every
    /// variable the expression reads is bound.
    fn key(&mut self, pattern: &Pattern, frame: &Frame) -> Option<Expression> {
        let (Place::Const(_), Place::Var(slot)) = (&pattern.predicate, &pattern.object) else {
            return None;
        };
        if is_known(&pattern.subject, &frame.certain) || frame.maybe.contains(slot) {
            return None;
        }
        for hint in self.hints.clone() {
            let Expression::Equal(a, b) = &hint else {
                continue;
            };
            for (variable, other) in [(a, b), (b, a)] {
                let Expression::Variable(variable) = &**variable else {
                    continue;
                };
                if self.slot(variable) == *slot && self.read_by(other).is_subset(&frame.maybe) {
                    return Some((**other).clone());
                }
            }
        }
        None
    }

    fn patterns(&mut self, patterns: &[TriplePattern]) -> Result<Vec<Pattern>, String> {
        let pending = patterns
            .iter()
            .map(|triple| self.triple(triple))
            .collect::<Result<Vec<_>, _>>()?;
        if pending
            .iter()
            .any(|t| !exact(&t.subject) || !exact(&t.object))
        {
            return Err("a typed literal in a pattern, which a store matches by value".to_owned());
        }
        Ok(pending)
    }

    /// The pattern a lookup answers most narrowly first, until what `until` names is
    /// bound where one is given. Returns the patterns left.
    fn schedule(
        &mut self,
        mut pending: Vec<Pattern>,
        until: Option<&Slots>,
        frame: &mut Frame,
        out: &mut Built,
    ) -> Result<Vec<Pattern>, String> {
        let reached = |frame: &Frame, pending: &[Pattern]| {
            until.is_some_and(|slots| {
                slots.iter().all(|slot| {
                    frame.certain.contains(slot)
                        && (frame.own.contains(slot) || !pending.iter().any(|t| t.mentions(*slot)))
                })
            })
        };
        while !pending.is_empty() && !reached(frame, &pending) {
            let keys: Vec<Option<Expression>> =
                pending.iter().map(|t| self.key(t, frame)).collect();
            let (at, rank) = pending
                .iter()
                .zip(&keys)
                .enumerate()
                .map(|(at, (triple, key))| (at, triple.rank(&frame.certain, key.is_some())))
                .min_by_key(|&(at, rank)| (rank, at))
                .unwrap_or_default();
            if rank == UNREACHABLE {
                return Err("a triple pattern with nothing bound to start from".to_owned());
            }
            let mut triple = pending.remove(at);
            if let (Some(key), Place::Var(slot)) = (&keys[at], &triple.object) {
                let reads = self.read_by(key);
                self.key_reads.push(reads.clone());
                let expr = self.expression(key, &frame.scope(Slots::new()))?;
                let reads = reads.into_iter().collect();
                triple.object = Place::Key(*slot, Key { expr, reads });
            }
            for place in [&triple.subject, &triple.predicate, &triple.object] {
                if let Place::Var(slot) | Place::Key(slot, _) = place {
                    frame.binds(*slot, true);
                }
            }
            out.push(Step::Triple(triple), &frame.certain);
        }
        Ok(pending)
    }

    fn triple(&mut self, triple: &TriplePattern) -> Result<Pattern, String> {
        Ok(Pattern {
            subject: self.place(&triple.subject)?,
            predicate: self.predicate(&triple.predicate),
            object: self.place(&triple.object)?,
        })
    }

    fn predicate(&mut self, predicate: &NamedNodePattern) -> Place {
        match predicate {
            NamedNodePattern::NamedNode(node) => Place::Const(Term::from(node.clone())),
            NamedNodePattern::Variable(variable) => Place::Var(self.slot(variable)),
        }
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
        Ok(Pattern {
            subject: self.constructed(&triple.subject, blanks)?,
            predicate: self.predicate(&triple.predicate),
            object: self.constructed(&triple.object, blanks)?,
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
        let mut conjuncts = Vec::new();
        conjunction(expr, &mut conjuncts);
        let hinted = self.hints.len();
        let keyed = self.key_reads.len();
        self.hints.extend(conjuncts.iter().map(|c| (*c).clone()));
        let entry = frame.certain.clone();
        let start = out.steps.len();
        self.pattern(inner, frame, out)?;
        self.hints.truncate(hinted);
        let steps = out.steps.split_off(start);
        let certain_after: Vec<Slots> = std::iter::once(entry)
            .chain(steps.iter().map(|(_, certain)| certain.clone()))
            .collect();
        let mut in_group = Slots::new();
        self.variables(inner, &mut in_group);
        let mut filters: Vec<Vec<Placed>> = certain_after.iter().map(|_| Vec::new()).collect();
        let mut unbound = Slots::new();
        for conjunct in conjuncts {
            let hidden = frame.hidden_among(&self.read_by(conjunct))?;
            unbound.extend(hidden.iter().copied());
            let needed = self.needed_to_place(conjunct, &in_group);
            let after = certain_after
                .iter()
                .position(|certain| needed.is_subset(certain))
                .unwrap_or(steps.len());
            let certain = &certain_after[after];
            let scope = Scope {
                certain: certain.clone(),
                maybe: frame.maybe.clone(),
                hidden,
            };
            let test = Step::Filter(self.expression(conjunct, &scope)?);
            filters[after].push((test, certain.clone()));
        }
        if self.key_reads[keyed..]
            .iter()
            .any(|reads| !reads.is_disjoint(&unbound))
        {
            return Err("a key reads a variable a filter reads as unbound".to_owned());
        }
        let mut steps = steps.into_iter();
        for (after, placed) in filters.into_iter().enumerate() {
            if after > 0 {
                out.steps.extend(steps.next());
            }
            out.steps.extend(placed);
        }
        Ok(())
    }

    pub(super) fn variables(&mut self, pattern: &GraphPattern, into: &mut Slots) {
        match pattern {
            GraphPattern::Bgp { patterns } => self.triple_variables(patterns, into),
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
                    into.extend(self.read_by(expression));
                }
            }
            GraphPattern::Filter { expr, inner } => {
                into.extend(self.read_by(expr));
                self.variables(inner, into);
            }
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => {
                into.insert(self.slot(variable));
                into.extend(self.read_by(expression));
                self.variables(inner, into);
            }
            GraphPattern::Values { variables, .. } | GraphPattern::Project { variables, .. } => {
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

    fn triple_variables(&mut self, patterns: &[TriplePattern], into: &mut Slots) {
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

    /// Every variable the expression reads, those inside an EXISTS too.
    fn read_by(&mut self, expression: &Expression) -> Slots {
        let mut read = Slots::new();
        self.collect_reads(expression, &mut read, &|_| true);
        read
    }

    /// The variables that must be bound before a conjunct of a filter can be placed:
    /// those it reads, but of an EXISTS only the ones the filtered group holds, since
    /// the others are the EXISTS' own.
    fn needed_to_place(&mut self, conjunct: &Expression, filtered: &Slots) -> Slots {
        let mut needed = Slots::new();
        self.collect_reads(conjunct, &mut needed, &|slot| filtered.contains(&slot));
        needed
    }

    fn collect_reads(
        &mut self,
        expression: &Expression,
        into: &mut Slots,
        inside_exists: &dyn Fn(usize) -> bool,
    ) {
        match expression {
            Expression::Variable(v) | Expression::Bound(v) => {
                into.insert(self.slot(v));
            }
            Expression::Exists(body) => {
                let mut inside = Slots::new();
                self.variables(body, &mut inside);
                into.extend(inside.into_iter().filter(|&slot| inside_exists(slot)));
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
                self.collect_reads(a, into, inside_exists);
                self.collect_reads(b, into, inside_exists);
            }
            Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                self.collect_reads(a, into, inside_exists);
            }
            Expression::In(a, list) => {
                self.collect_reads(a, into, inside_exists);
                for e in list {
                    self.collect_reads(e, into, inside_exists);
                }
            }
            Expression::If(a, b, c) => {
                for e in [a, b, c] {
                    self.collect_reads(e, into, inside_exists);
                }
            }
            Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
                for e in list {
                    self.collect_reads(e, into, inside_exists);
                }
            }
            Expression::NamedNode(_) | Expression::Literal(_) => {}
        }
    }
}

fn is_known(place: &Place, bound: &Slots) -> bool {
    match place {
        Place::Const(_) => true,
        Place::Var(slot) => bound.contains(slot),
        Place::Blank(_) | Place::Key(..) => false,
    }
}

impl Pattern {
    fn mentions(&self, slot: usize) -> bool {
        [&self.subject, &self.predicate, &self.object]
            .iter()
            .any(|place| matches!(place, Place::Var(s) if *s == slot))
    }

    /// How narrowly the lookup it takes answers it: the lower, the narrower.
    fn rank(&self, bound: &Slots, keyed: bool) -> u8 {
        match (
            is_known(&self.subject, bound),
            is_known(&self.predicate, bound),
            is_known(&self.object, bound),
        ) {
            (true, _, _) => 0,
            (false, true, false) if keyed => 1,
            (false, true, true) => 2,
            (false, _, true) => 3,
            (false, true, false) => 4,
            (false, false, false) => UNREACHABLE,
        }
    }
}

fn walked(path: &PropertyPathExpression) -> Result<(Vec<Term>, bool), String> {
    let (PropertyPathExpression::ZeroOrMore(edges) | PropertyPathExpression::OneOrMore(edges)) =
        path
    else {
        return Err("a property path that is not a walk".to_owned());
    };
    let PropertyPathExpression::NegatedPropertySet(skipped) = &**edges else {
        return Err("a property path that is not a walk over all but some edges".to_owned());
    };
    Ok((
        skipped.iter().map(|n| Term::from(n.clone())).collect(),
        matches!(path, PropertyPathExpression::ZeroOrMore(_)),
    ))
}

fn value(cell: &Option<GroundTerm>) -> Result<Option<Term>, String> {
    match cell {
        None => Ok(None),
        Some(GroundTerm::NamedNode(n)) => Ok(Some(Term::from(n.clone()))),
        Some(GroundTerm::Literal(l)) if by_term(l) => Ok(Some(Term::from(l.clone()))),
        Some(GroundTerm::Literal(_)) => {
            Err("a typed literal in VALUES, which a join matches by value".to_owned())
        }
        Some(_) => Err("a quoted triple in VALUES".to_owned()),
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
