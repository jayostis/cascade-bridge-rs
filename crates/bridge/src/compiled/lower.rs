use super::explain::{Explained, Nested, Noted};
use super::keys::{known_values, settled, Hints};
use super::run::Compiled;
use super::scope::analysis::{self as node, makes_blank_nodes};
use super::scope::{analyze, Bound, Certain, Frame, Hidden, Names, Reads, Scope, Scoped};
use super::{Aggregate, Edge, Expr, Group, Key, Kind, Note, Pattern, Place, Plan, Reach, Step};
use oxigraph::model::vocab::xsd;
use oxigraph::model::{BlankNode, Literal, NamedNode, NamedNodeRef, Term};
use oxiri::Iri;
use spargebra::algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, PropertyPathExpression,
};
use spargebra::term::{GroundTerm, NamedNodePattern, TermPattern, TriplePattern};
use spargebra::Query;

const UNREACHABLE: u8 = u8::MAX;

pub(crate) fn compile(query: &Query) -> Result<Compiled, String> {
    lowered(query, Names::default()).map(|(compiled, _)| compiled)
}

pub(crate) fn compile_explained(query: &Query) -> Result<Explained, String> {
    lowered(query, Names::explaining()).map(|(compiled, labels)| Explained { compiled, labels })
}

fn lowered(query: &Query, mut names: Names) -> Result<(Compiled, Vec<String>), String> {
    let Query::Construct {
        template,
        pattern,
        base_iri,
        ..
    } = query
    else {
        return Err("it is not a CONSTRUCT".to_owned());
    };
    let scoped = analyze(&mut names, pattern, &mut Frame::default());
    let mut lowering = Lowering {
        names,
        base: base_iri.clone(),
        hints: Hints::default(),
        memos: 0,
        nested: Vec::new(),
    };
    let mut built = Built::default();
    lowering.pattern(&scoped, &mut built)?;
    let mut blanks = Vec::new();
    let template = template
        .iter()
        .map(|triple| lowering.template(triple, &mut blanks))
        .collect::<Result<_, _>>()?;
    debug_assert!(
        lowering.nested.is_empty(),
        "an EXISTS was lowered outside a noted expression"
    );
    let compiled = Compiled {
        plan: built.plan(),
        template,
        blanks: blanks.len(),
        width: lowering.names.width(),
    };
    Ok((compiled, lowering.names.into_labels()))
}

type Placed = (Step, Certain);

/// Each step with what is certainly bound once it has run.
#[derive(Default)]
struct Built {
    steps: Vec<Placed>,
}

impl Built {
    fn push(&mut self, step: Step, certain: &Certain) {
        self.steps.push((step, certain.clone()));
    }

    fn plan(self) -> Plan {
        self.steps.into_iter().map(|(step, _)| step).collect()
    }
}

pub(super) struct Lowering {
    pub(super) names: Names,
    pub(super) base: Option<Iri<String>>,
    hints: Hints,
    memos: usize,
    pub(super) nested: Vec<Nested>,
}

impl Lowering {
    /// A name for an answer that depends on the tree alone, unique within the plan.
    pub(super) fn next_memo(&mut self) -> usize {
        self.memos += 1;
        self.memos - 1
    }

    fn note(&mut self, expression: &Expression) -> Note {
        self.names.is_explaining().then(|| {
            Box::new(Noted {
                source: self.names.relabel(&expression.to_string()),
                exists: std::mem::take(&mut self.nested),
            })
        })
    }

    fn noted(&mut self, expression: &Expression, scope: &Scope) -> Result<(Expr, Note), String> {
        let expr = self.expression(expression, scope)?;
        Ok((expr, self.note(expression)))
    }

    /// Runs `lower` without the filters around the pattern.
    fn apart<T>(&mut self, lower: impl FnOnce(&mut Self) -> T) -> T {
        let hints = self.hints.replace(Vec::new());
        let lowered = lower(self);
        self.hints.replace(hints);
        lowered
    }

    pub(super) fn exists(&mut self, body: &GraphPattern, scope: &Scope) -> Result<Plan, String> {
        let scoped = analyze(&mut self.names, body, &mut Frame::within_exists(scope));
        let mut built = Built::default();
        self.apart(|me| me.pattern(&scoped, &mut built))?;
        Ok(built.plan())
    }

    fn pattern(&mut self, scoped: &Scoped, out: &mut Built) -> Result<(), String> {
        match scoped {
            Scoped::Declined(reason) => Err((*reason).to_owned()),
            Scoped::Triples(triples) => self.triples(triples, out),
            Scoped::Join(left, right) => {
                self.pattern(left, out)?;
                self.pattern(right, out)
            }
            Scoped::Filter(filter) => self.filter(filter, out),
            Scoped::Extend(extend) => self.extend(extend, out),
            Scoped::Union(union) => self.union(union, out),
            Scoped::Optional(optional) => self.optional(optional, out),
            Scoped::Values(values) => self.values(values, out),
            Scoped::Subquery(subquery) => self.subquery(subquery, out),
            Scoped::Group(group) => self.group(group, out),
            Scoped::Path(path) => self.walk(path, out),
        }
    }

    fn extend(&mut self, extend: &node::Binding, out: &mut Built) -> Result<(), String> {
        match &extend.inner {
            Scoped::Triples(triples) => {
                let pending = self.patterns(triples.patterns)?;
                let mut bound = triples.entry.clone();
                let until = (!makes_blank_nodes(extend.expression)).then_some(&extend.operands);
                let rest = self.schedule(pending, until, &mut bound, out)?;
                let (operand, note) = self.operand(extend, &bound.scope(Hidden::new()))?;
                bound.binds(extend.slot, extend.certainly);
                out.push(bind_step(extend, operand, note), &bound.certain);
                self.schedule(rest, None, &mut bound, out)?;
            }
            inner => {
                self.pattern(inner, out)?;
                let (operand, note) = self.operand(extend, &extend.scope)?;
                out.push(bind_step(extend, operand, note), &extend.certain_after);
            }
        }
        Ok(())
    }

    fn operand(&mut self, extend: &node::Binding, scope: &Scope) -> Result<(Expr, Note), String> {
        extend.reads?;
        self.noted(extend.expression, scope)
    }

    fn union(&mut self, union: &node::Union, out: &mut Built) -> Result<(), String> {
        let mut plans = Vec::new();
        let around = self.hints.conjuncts().to_vec();
        for branch in &union.branches {
            let known = known_values(branch.pattern);
            self.hints
                .replace(around.iter().map(|hint| settled(hint, &known)).collect());
            let mut built = Built::default();
            self.pattern(&branch.scoped, &mut built)?;
            plans.push(built.plan());
        }
        self.hints.replace(around);
        out.push(Step::Union(plans), &union.certain_after);
        Ok(())
    }

    fn optional(&mut self, optional: &node::Optional, out: &mut Built) -> Result<(), String> {
        self.pattern(&optional.left, out)?;
        optional.reads?;
        let mut built = Built::default();
        self.apart(|me| me.pattern(&optional.right, &mut built))?;
        let (condition, note) = match &optional.condition {
            Some((condition, scope)) => {
                let (expr, note) = self.noted(condition, scope)?;
                (Some(expr), note)
            }
            None => (None, None),
        };
        out.push(
            Step::Optional(built.plan(), condition, note),
            &optional.certain_after,
        );
        Ok(())
    }

    fn values(&mut self, values: &node::Values, out: &mut Built) -> Result<(), String> {
        let table = values
            .bindings
            .iter()
            .map(|row| row.iter().map(value).collect::<Result<Vec<_>, _>>())
            .collect::<Result<Vec<_>, _>>()?;
        out.push(
            Step::Values(values.slots.clone(), table),
            &values.certain_after,
        );
        Ok(())
    }

    fn subquery(&mut self, subquery: &node::Subquery, out: &mut Built) -> Result<(), String> {
        let mut built = Built::default();
        let into = if subquery.distinct.is_some() {
            &mut built
        } else {
            &mut *out
        };
        self.names.enter(subquery.names.clone());
        let lowered = self.apart(|me| me.pattern(&subquery.inner, into));
        self.names.leave();
        lowered?;
        if let Some(distinct) = &subquery.distinct {
            distinct.reads?;
            let step = Step::Group(Group {
                inner: built.plan(),
                keys: subquery.keys.clone(),
                aggregates: Vec::new(),
            });
            out.push(step, &distinct.certain_after);
        }
        Ok(())
    }

    fn group(&mut self, group: &node::GroupBy, out: &mut Built) -> Result<(), String> {
        let mut built = Built::default();
        self.apart(|me| me.pattern(&group.inner, &mut built))?;
        let aggregates = group
            .aggregates
            .iter()
            .map(|aggregate| self.aggregates(aggregate, &group.scope))
            .collect::<Result<Vec<_>, _>>()?;
        group.reads?;
        let step = Step::Group(Group {
            inner: built.plan(),
            keys: group.keys.clone(),
            aggregates,
        });
        out.push(step, &group.certain_after);
        Ok(())
    }

    fn aggregates(
        &mut self,
        aggregate: &node::Aggregated,
        scope: &Scope,
    ) -> Result<Aggregate, String> {
        aggregate.fresh?;
        let (kind, note) = self.aggregate(aggregate, scope)?;
        Ok(Aggregate {
            slot: aggregate.slot,
            kind,
            note,
        })
    }

    fn aggregate(
        &mut self,
        aggregate: &node::Aggregated,
        scope: &Scope,
    ) -> Result<(Kind, Note), String> {
        match aggregate.expression {
            AggregateExpression::CountSolutions { distinct: false } => Ok((
                Kind::Count {
                    of: None,
                    distinct: false,
                },
                None,
            )),
            AggregateExpression::CountSolutions { .. } => Err("COUNT(DISTINCT *)".to_owned()),
            AggregateExpression::FunctionCall {
                name,
                expr,
                distinct,
            } => self.aggregate_call(aggregate, name, expr, *distinct, scope),
        }
    }

    fn aggregate_call(
        &mut self,
        aggregate: &node::Aggregated,
        name: &AggregateFunction,
        expr: &Expression,
        distinct: bool,
        scope: &Scope,
    ) -> Result<(Kind, Note), String> {
        if let (AggregateFunction::Count, Expression::Variable(variable)) = (name, expr) {
            let of = Some(self.names.slot(variable));
            return Ok((Kind::Count { of, distinct }, None));
        }
        if distinct {
            return Err("DISTINCT in an aggregate".to_owned());
        }
        aggregate.reads?;
        let (operand, note) = self.noted(expr, scope)?;
        let kind = match name {
            AggregateFunction::Sum => Kind::Sum(operand),
            AggregateFunction::Min => Kind::Min(operand),
            AggregateFunction::Max => Kind::Max(operand),
            AggregateFunction::Sample => Kind::Sample(operand),
            _ => return Err("an aggregate other than COUNT, SUM, MIN, MAX or SAMPLE".to_owned()),
        };
        Ok((kind, note))
    }

    fn walk(&mut self, path: &node::Path, out: &mut Built) -> Result<(), String> {
        if let Some(edges) = &path.edges {
            return self.choice(path, edges, out);
        }
        let (skipped, from_itself) = walked(path.path)?;
        let (from, to) = (self.place(path.subject)?, self.place(path.object)?);
        path.starts?;
        let step = Step::Reach(Reach {
            from,
            to,
            skipped,
            from_itself,
        });
        out.push(step, &path.certain_after);
        Ok(())
    }

    fn choice(
        &mut self,
        path: &node::Path,
        edges: &[(&NamedNode, bool)],
        out: &mut Built,
    ) -> Result<(), String> {
        let mut taken = Vec::new();
        for &(edge, backwards) in edges {
            let (mut subject, mut object) = (self.place(path.subject)?, self.place(path.object)?);
            if backwards {
                std::mem::swap(&mut subject, &mut object);
            }
            let predicate = Place::Const(Term::from(edge.clone()));
            taken.push(Edge {
                pattern: Pattern {
                    subject,
                    predicate,
                    object,
                },
                backwards,
            });
        }
        out.push(Step::Choice(taken), &path.certain_after);
        Ok(())
    }

    fn triples(&mut self, triples: &node::Triples, out: &mut Built) -> Result<(), String> {
        let pending = self.patterns(triples.patterns)?;
        self.schedule(pending, None, &mut triples.entry.clone(), out)?;
        Ok(())
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
            return Err("a literal in a pattern of a datatype a store matches by value".to_owned());
        }
        Ok(pending)
    }

    /// The pattern a lookup answers most narrowly first, until what `until` names is
    /// bound where one is given. Returns the patterns left.
    fn schedule(
        &mut self,
        mut pending: Vec<Pattern>,
        until: Option<&Reads>,
        bound: &mut Bound,
        out: &mut Built,
    ) -> Result<Vec<Pattern>, String> {
        let reached = |bound: &Bound, pending: &[Pattern]| {
            until.is_some_and(|slots| {
                slots.iter().all(|slot| {
                    bound.certain.contains(slot)
                        && (bound.own.contains(slot) || !pending.iter().any(|t| t.mentions(slot)))
                })
            })
        };
        while !pending.is_empty() && !reached(bound, &pending) {
            let keys: Vec<Option<Expression>> = pending
                .iter()
                .map(|t| self.hints.lookup(&mut self.names, t, bound))
                .collect();
            let (at, rank) = pending
                .iter()
                .zip(&keys)
                .enumerate()
                .map(|(at, (triple, key))| (at, triple.rank(&bound.certain, key.is_some())))
                .min_by_key(|&(at, rank)| (rank, at))
                .unwrap_or_default();
            if rank == UNREACHABLE {
                return Err("a triple pattern with nothing bound to start from".to_owned());
            }
            let mut triple = pending.remove(at);
            if let (Some(key), Place::Var(slot)) = (&keys[at], &triple.object) {
                let reads = self.names.read_by(key);
                self.hints.record(reads.clone());
                let (expr, note) = self.noted(key, &bound.scope(Hidden::new()))?;
                let reads = reads.iter().collect();
                triple.object = Place::Key(*slot, Key { expr, reads, note });
            }
            for place in [&triple.subject, &triple.predicate, &triple.object] {
                if let Place::Var(slot) | Place::Key(slot, _) = place {
                    bound.binds(*slot, true);
                }
            }
            out.push(Step::Triple(triple), &bound.certain);
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
            NamedNodePattern::Variable(variable) => Place::Var(self.names.slot(variable)),
        }
    }

    fn place(&mut self, term: &TermPattern) -> Result<Place, String> {
        match term {
            TermPattern::NamedNode(node) => Ok(Place::Const(Term::from(node.clone()))),
            TermPattern::Literal(literal) => Ok(Place::Const(Term::from(literal.clone()))),
            TermPattern::Variable(variable) => Ok(Place::Var(self.names.slot(variable))),
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

    fn filter(&mut self, filter: &node::Filter, out: &mut Built) -> Result<(), String> {
        let mark = self
            .hints
            .assume(filter.conjuncts.iter().map(|c| c.expression));
        let start = out.steps.len();
        self.pattern(&filter.inner, out)?;
        self.hints.release(&mark);
        let steps = out.steps.split_off(start);
        let certain_after: Vec<Certain> = std::iter::once(filter.entry.clone())
            .chain(steps.iter().map(|(_, certain)| certain.clone()))
            .collect();
        let mut filters: Vec<Vec<Placed>> = certain_after.iter().map(|_| Vec::new()).collect();
        for conjunct in &filter.conjuncts {
            let hidden = conjunct.hidden.clone()?;
            let after = certain_after
                .iter()
                .position(|certain| conjunct.needed.is_subset_of(certain))
                .unwrap_or(steps.len());
            let certain = &certain_after[after];
            let scope = Scope {
                certain: certain.clone(),
                maybe: filter.maybe.clone(),
                hidden,
            };
            let (expr, note) = self.noted(conjunct.expression, &scope)?;
            let test = Step::Filter(expr, note);
            filters[after].push((test, certain.clone()));
        }
        self.hints.keep_clear(&mark, &filter.unbound)?;
        let mut steps = steps.into_iter();
        for (after, placed) in filters.into_iter().enumerate() {
            if after > 0 {
                out.steps.extend(steps.next());
            }
            out.steps.extend(placed);
        }
        Ok(())
    }
}

fn bind_step(extend: &node::Binding, operand: Expr, note: Note) -> Step {
    if extend.joins {
        Step::Agree(extend.slot, operand, note)
    } else {
        Step::Bind(extend.slot, operand, note)
    }
}

impl Place {
    pub(super) fn is_known(&self, bound: &Certain) -> bool {
        match self {
            Place::Const(_) => true,
            Place::Var(slot) => bound.contains(*slot),
            Place::Blank(_) | Place::Key(..) => false,
        }
    }
}

impl Pattern {
    fn mentions(&self, slot: usize) -> bool {
        [&self.subject, &self.predicate, &self.object]
            .iter()
            .any(|place| matches!(place, Place::Var(s) if *s == slot))
    }

    /// How narrowly the lookup it takes answers it: the lower, the narrower.
    fn rank(&self, bound: &Certain, keyed: bool) -> u8 {
        match (
            self.subject.is_known(bound),
            self.predicate.is_known(bound),
            self.object.is_known(bound),
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
        return Err("a property path other than a choice of edges or a walk".to_owned());
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
            Err("a literal in VALUES of a datatype a join matches by value".to_owned())
        }
        Some(_) => Err("a quoted triple in VALUES".to_owned()),
    }
}

/// Whether a place is matched by its term.
fn exact(place: &Place) -> bool {
    !matches!(place, Place::Const(Term::Literal(l)) if !by_term(l))
}

/// Oxigraph stores a literal of these datatypes as its value, so a pattern or a join
/// matches it to another lexical form of that value, and to another of these types.
const STORED_BY_VALUE: &[NamedNodeRef<'_>] = &[
    xsd::BOOLEAN,
    xsd::FLOAT,
    xsd::DOUBLE,
    xsd::INTEGER,
    xsd::BYTE,
    xsd::SHORT,
    xsd::INT,
    xsd::LONG,
    xsd::UNSIGNED_BYTE,
    xsd::UNSIGNED_SHORT,
    xsd::UNSIGNED_INT,
    xsd::UNSIGNED_LONG,
    xsd::POSITIVE_INTEGER,
    xsd::NEGATIVE_INTEGER,
    xsd::NON_POSITIVE_INTEGER,
    xsd::NON_NEGATIVE_INTEGER,
    xsd::DECIMAL,
    xsd::DATE_TIME,
    xsd::DATE_TIME_STAMP,
    xsd::TIME,
    xsd::DATE,
    xsd::G_YEAR_MONTH,
    xsd::G_YEAR,
    xsd::G_MONTH_DAY,
    xsd::G_DAY,
    xsd::G_MONTH,
    xsd::DURATION,
    xsd::YEAR_MONTH_DURATION,
    xsd::DAY_TIME_DURATION,
];

fn by_term(literal: &Literal) -> bool {
    !STORED_BY_VALUE.contains(&literal.datatype())
}
