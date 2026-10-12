use super::expr::{holds, order};
use super::tree::{Id, Tree};
use super::{Aggregate, Fail, Group, Kind, Pattern, Place, Plan, Row, Step};
use oxigraph::model::{
    BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term, Triple,
};
use spareval::ExpressionTerm;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

type Sink<'a> = &'a mut dyn FnMut(Row) -> Result<(), Fail>;

/// A CONSTRUCT query lowered to a plan that starts from what is bound and looks
/// up what follows.
pub(crate) struct Compiled {
    pub(super) plan: Plan,
    pub(super) template: Vec<Pattern>,
    pub(super) blanks: usize,
    pub(super) width: usize,
}

impl Compiled {
    pub(crate) fn construct(&self, tree: &Tree) -> Result<Vec<Quad>, Fail> {
        let mut emitted: HashSet<Triple> = HashSet::new();
        let mut quads = Vec::new();
        run(&self.plan, vec![None; self.width], tree, &mut |row| {
            let mut blanks = vec![None; self.blanks];
            for template in &self.template {
                let Some(triple) = instance(template, &row, &mut blanks) else {
                    continue;
                };
                let blank = triple.subject.is_blank_node() || triple.object.is_blank_node();
                if blank || emitted.insert(triple.clone()) {
                    quads.push(Quad::new(
                        triple.subject,
                        triple.predicate,
                        triple.object,
                        GraphName::DefaultGraph,
                    ));
                }
            }
            Ok(())
        })?;
        Ok(quads)
    }
}

fn instance(template: &Pattern, row: &Row, blanks: &mut [Option<BlankNode>]) -> Option<Triple> {
    let mut value = |place: &Place| match place {
        Place::Const(term) => Some(term.clone()),
        Place::Var(slot) => row[*slot].as_deref().cloned(),
        Place::Blank(at) => Some(Term::from(
            blanks[*at].get_or_insert_with(BlankNode::default).clone(),
        )),
    };
    Some(Triple::new(
        NamedOrBlankNode::try_from(value(&template.subject)?).ok()?,
        NamedNode::try_from(value(&template.predicate)?).ok()?,
        value(&template.object)?,
    ))
}

pub(super) fn exists(plan: &Plan, tree: &Tree, row: &Row) -> Result<bool, Fail> {
    match run(plan, row.clone(), tree, &mut |_| Err(Fail::Done)) {
        Ok(()) => Ok(false),
        Err(Fail::Done) => Ok(true),
        Err(other) => Err(other),
    }
}

fn run(steps: &[Step], row: Row, tree: &Tree, sink: Sink<'_>) -> Result<(), Fail> {
    let Some((step, rest)) = steps.split_first() else {
        return sink(row);
    };
    match step {
        Step::Triple(pattern) => scan(pattern, &row, tree, &mut |next| run(rest, next, tree, sink)),
        Step::Filter(expr) => {
            if holds(expr, tree, &row)? {
                run(rest, row, tree, sink)
            } else {
                Ok(())
            }
        }
        Step::Bind(slot, expr) => {
            let mut row = row;
            row[*slot] = match expr(tree, &row) {
                Ok(term) => Some(Rc::new(Term::from(term))),
                Err(Fail::Error) => None,
                Err(other) => return Err(other),
            };
            run(rest, row, tree, sink)
        }
        Step::Values(slots, table) => {
            for values in table {
                if let Some(next) = joined(&row, slots, values) {
                    run(rest, next, tree, sink)?;
                }
            }
            Ok(())
        }
        Step::Union(branches) => {
            for branch in branches {
                run(branch, row.clone(), tree, &mut |next| {
                    run(rest, next, tree, sink)
                })?;
            }
            Ok(())
        }
        Step::Optional(branch, condition) => {
            let mut matched = false;
            run(branch, row.clone(), tree, &mut |next| {
                if let Some(condition) = condition {
                    if !holds(condition, tree, &next)? {
                        return Ok(());
                    }
                }
                matched = true;
                run(rest, next, tree, sink)
            })?;
            if matched {
                Ok(())
            } else {
                run(rest, row, tree, sink)
            }
        }
        Step::Group(group) => group.run(row, tree, &mut |next| run(rest, next, tree, sink)),
    }
}

fn joined(row: &Row, slots: &[usize], values: &[Option<Term>]) -> Option<Row> {
    let mut next = row.clone();
    for (&slot, value) in slots.iter().zip(values) {
        let Some(value) = value else { continue };
        match &next[slot] {
            Some(bound) if **bound != *value => return None,
            Some(_) => {}
            None => next[slot] = Some(Rc::new(value.clone())),
        }
    }
    Some(next)
}

fn scan(
    pattern: &Pattern,
    row: &Row,
    tree: &Tree,
    next: &mut dyn FnMut(Row) -> Result<(), Fail>,
) -> Result<(), Fail> {
    let fixed = |place: &Place| -> Option<Option<Id>> {
        match place {
            Place::Const(term) => tree.id(term).map(Some),
            Place::Blank(_) => unreachable!("only a template holds a blank node"),
            Place::Var(slot) => match &row[*slot] {
                Some(term) => tree.id(term).map(Some),
                None => Some(None),
            },
        }
    };
    let (Some(subject), Some(predicate), Some(object)) = (
        fixed(&pattern.subject),
        fixed(&pattern.predicate),
        fixed(&pattern.object),
    ) else {
        return Ok(());
    };
    tree.scan(subject, predicate, object, &mut |s, p, o| {
        let mut bound = row.clone();
        for (place, id) in [
            (&pattern.subject, s),
            (&pattern.predicate, p),
            (&pattern.object, o),
        ] {
            let Place::Var(slot) = place else { continue };
            let term = tree.term(id);
            match &bound[*slot] {
                Some(existing) if **existing != **term => return Ok(()),
                Some(_) => {}
                None => bound[*slot] = Some(Rc::clone(term)),
            }
        }
        next(bound)
    })
}

enum Accumulator {
    Count {
        counted: u64,
        seen: HashSet<Rc<Term>>,
    },
    /// Empty, or one value, or `Err` where an operand failed and the aggregate is unbound.
    Chosen(Option<Result<ExpressionTerm, ()>>),
}

impl Accumulator {
    fn of(kind: &Kind) -> Self {
        match kind {
            Kind::Count { .. } => Self::Count {
                counted: 0,
                seen: HashSet::new(),
            },
            _ => Self::Chosen(None),
        }
    }

    fn add(&mut self, kind: &Kind, tree: &Tree, row: &Row) -> Result<(), Fail> {
        match (self, kind) {
            (Self::Count { counted, seen }, Kind::Count { of, distinct }) => {
                let fresh = match of.map(|slot| &row[slot]) {
                    None => true,
                    Some(None) => false,
                    Some(Some(term)) => !distinct || seen.insert(Rc::clone(term)),
                };
                *counted += u64::from(fresh);
            }
            (Self::Chosen(chosen), Kind::Sample(expr)) => {
                if chosen.is_none() {
                    match expr(tree, row) {
                        Ok(term) => *chosen = Some(Ok(term)),
                        Err(Fail::Error) => {}
                        Err(other) => return Err(other),
                    }
                }
            }
            (Self::Chosen(chosen), Kind::Min(expr) | Kind::Max(expr)) => {
                let wanted = if matches!(kind, Kind::Min(_)) {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
                match (expr(tree, row), &*chosen) {
                    (Err(Fail::Error), _) => *chosen = Some(Err(())),
                    (Err(other), _) => return Err(other),
                    (Ok(_), Some(Err(()))) => {}
                    (Ok(term), None) => *chosen = Some(Ok(term)),
                    (Ok(term), Some(Ok(best))) => {
                        if order(&term, best)? == wanted {
                            *chosen = Some(Ok(term));
                        }
                    }
                }
            }
            _ => unreachable!("an accumulator is made for its own kind"),
        }
        Ok(())
    }

    fn finish(self) -> Option<Rc<Term>> {
        match self {
            Self::Count { counted, .. } => Some(Rc::new(Term::from(Literal::from(
                i64::try_from(counted).expect("fewer than 2^63 rows"),
            )))),
            Self::Chosen(Some(Ok(term))) => Some(Rc::new(Term::from(term))),
            Self::Chosen(_) => None,
        }
    }
}

impl Group {
    fn run(&self, row: Row, tree: &Tree, next: Sink<'_>) -> Result<(), Fail> {
        let fresh = || -> Vec<Accumulator> {
            self.aggregates
                .iter()
                .map(|aggregate| Accumulator::of(&aggregate.kind))
                .collect()
        };
        let mut groups: HashMap<Vec<Option<Rc<Term>>>, Vec<Accumulator>> = HashMap::new();
        run(&self.inner, row.clone(), tree, &mut |inner| {
            let key = self.keys.iter().map(|&slot| inner[slot].clone()).collect();
            let accumulators = groups.entry(key).or_insert_with(fresh);
            for (accumulator, Aggregate { kind, .. }) in
                accumulators.iter_mut().zip(&self.aggregates)
            {
                accumulator.add(kind, tree, &inner)?;
            }
            Ok(())
        })?;
        if groups.is_empty() && self.keys.is_empty() {
            groups.insert(Vec::new(), fresh());
        }
        for (key, accumulators) in groups {
            let mut grouped = row.clone();
            for (&slot, value) in self.keys.iter().zip(key) {
                grouped[slot] = value;
            }
            for (aggregate, accumulator) in self.aggregates.iter().zip(accumulators) {
                grouped[aggregate.slot] = accumulator.finish();
            }
            next(grouped)?;
        }
        Ok(())
    }
}
