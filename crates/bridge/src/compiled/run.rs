use super::expr::{holds, order};
use super::tree::{Id, Tree};
use super::{
    Aggregate, Edge, Expr, Fail, Group, Key, Kind, Pattern, Place, Plan, Reach, Row, Step,
};
use oxigraph::model::vocab::xsd;
use oxigraph::model::{
    BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term, Triple,
};
use oxsdatatypes::Integer;
use spareval::ExpressionTerm;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

type Sink<'a> = &'a mut dyn FnMut(&mut Row) -> Result<(), Fail>;
type Picked = Option<Result<ExpressionTerm, ()>>;
type Groups = HashMap<Vec<Option<Rc<Term>>>, Vec<Accumulator>>;

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
        tree.forget_answers();
        let ran = run(&self.plan, &mut vec![None; self.width], tree, &mut |row| {
            let mut blanks = vec![None; self.blanks];
            for template in &self.template {
                let Some(triple) = instance(template, row, &mut blanks) else {
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
        });
        tree.forget_answers();
        ran?;
        Ok(quads)
    }
}

fn instance(template: &Pattern, row: &Row, blanks: &mut [Option<BlankNode>]) -> Option<Triple> {
    let mut value = |place: &Place| match place {
        Place::Blank(at) => Some(Term::from(
            blanks[*at].get_or_insert_with(BlankNode::default).clone(),
        )),
        place => term_at(place, row),
    };
    Some(Triple::new(
        NamedOrBlankNode::try_from(value(&template.subject)?).ok()?,
        NamedNode::try_from(value(&template.predicate)?).ok()?,
        value(&template.object)?,
    ))
}

fn term_at(place: &Place, row: &Row) -> Option<Term> {
    match place {
        Place::Const(term) => Some(term.clone()),
        Place::Var(slot) => row[*slot].as_deref().cloned(),
        Place::Blank(_) | Place::Key(..) => {
            unreachable!("only a pattern holds a key, a template a blank node")
        }
    }
}

pub(super) fn exists(plan: &Plan, tree: &Tree, row: &Row, hidden: &[usize]) -> Result<bool, Fail> {
    let mut seen = row.clone();
    unbind(&mut seen, hidden);
    match run(plan, &mut seen, tree, &mut |_| Err(Fail::Done)) {
        Ok(()) => Ok(false),
        Err(Fail::Done) => Ok(true),
        Err(other) => Err(other),
    }
}

fn run(steps: &[Step], row: &mut Row, tree: &Tree, sink: Sink<'_>) -> Result<(), Fail> {
    if tree.stopped() {
        return Err(Fail::Stopped);
    }
    let Some((step, rest)) = steps.split_first() else {
        return sink(row);
    };
    let mut next = |row: &mut Row| run(rest, row, tree, sink);
    match step {
        Step::Triple(pattern) => scan(pattern, row, tree, &mut next),
        Step::Filter(expr, _) => {
            if holds(expr, tree, row)? {
                next(row)
            } else {
                Ok(())
            }
        }
        Step::Bind(slot, expr, _) => bind(*slot, expr, row, tree, &mut next),
        Step::Agree(slot, expr, _) => agree(*slot, expr, row, tree, &mut next),
        Step::Values(slots, table) => each_values(slots, table, row, &mut next),
        Step::Union(branches) => {
            for branch in branches {
                run(branch, row, tree, &mut next)?;
            }
            Ok(())
        }
        Step::Optional(branch, condition, _) => {
            optional(branch, condition.as_ref(), row, tree, &mut next)
        }
        Step::Group(group) => group.run(row, tree, &mut next),
        Step::Reach(walk) => reach(walk, row, tree, &mut next),
        Step::Choice(edges) => choose(edges, row, tree, &mut next),
    }
}

fn bind(slot: usize, expr: &Expr, row: &mut Row, tree: &Tree, next: Sink<'_>) -> Result<(), Fail> {
    row[slot] = match expr(tree, row) {
        Ok(term) => Some(Rc::new(Term::from(term))),
        Err(Fail::Error) => None,
        Err(other) => return Err(other),
    };
    let done = next(row);
    row[slot] = None;
    done
}

/// A BIND of a variable a group around it binds: a join on that variable. An error
/// leaves the variable unbound in the group alone, which no row of ours says, so the
/// plan gives up.
fn agree(slot: usize, expr: &Expr, row: &mut Row, tree: &Tree, next: Sink<'_>) -> Result<(), Fail> {
    let computed = match expr(tree, row) {
        Ok(term) => term,
        Err(Fail::Error) => return Err(Fail::GiveUp),
        Err(other) => return Err(other),
    };
    match &row[slot] {
        Some(bound) if **bound != Term::from(computed.clone()) => Ok(()),
        Some(_) => next(row),
        None => {
            row[slot] = Some(Rc::new(Term::from(computed)));
            let done = next(row);
            row[slot] = None;
            done
        }
    }
}

fn each_values(
    slots: &[usize],
    table: &[Vec<Option<Term>>],
    row: &mut Row,
    next: Sink<'_>,
) -> Result<(), Fail> {
    for values in table {
        let Some(set) = joined(row, slots, values) else {
            continue;
        };
        let done = next(row);
        unbind(row, &set);
        done?;
    }
    Ok(())
}

fn optional(
    branch: &Plan,
    condition: Option<&Expr>,
    row: &mut Row,
    tree: &Tree,
    next: Sink<'_>,
) -> Result<(), Fail> {
    let mut matched = false;
    run(branch, row, tree, &mut |row| {
        if let Some(condition) = condition {
            if !holds(condition, tree, row)? {
                return Ok(());
            }
        }
        matched = true;
        next(row)
    })?;
    if matched {
        Ok(())
    } else {
        next(row)
    }
}

/// Each pair of ends that one of the edges links, once, however many link it, as
/// stock Oxigraph evaluates an alternative path.
fn choose(edges: &[Edge], row: &mut Row, tree: &Tree, next: Sink<'_>) -> Result<(), Fail> {
    let mut seen: HashSet<(Id, Id)> = HashSet::new();
    for edge in edges {
        let Some((subject, predicate, object)) = fixed(&edge.pattern, row, tree)? else {
            continue;
        };
        tree.scan(subject, predicate, object, &mut |s, p, o| {
            if !seen.insert(if edge.backwards { (o, s) } else { (s, o) }) {
                return Ok(());
            }
            let Some(set) = unify(&edge.pattern, [s, p, o], row, tree) else {
                return Ok(());
            };
            let done = next(row);
            release(row, set);
            done
        })?;
    }
    Ok(())
}

fn reach(walk: &Reach, row: &mut Row, tree: &Tree, next: Sink<'_>) -> Result<(), Fail> {
    let Some(start) = term_at(&walk.from, row) else {
        return Ok(());
    };
    for term in reached(walk, start, tree)? {
        match &walk.to {
            Place::Var(slot) if row[*slot].is_none() => {
                row[*slot] = Some(term);
                let done = next(row);
                row[*slot] = None;
                done?;
            }
            to => {
                if term_at(to, row).is_some_and(|bound| bound == *term) {
                    next(row)?;
                }
            }
        }
    }
    Ok(())
}

fn reached(reach: &Reach, start: Term, tree: &Tree) -> Result<Vec<Rc<Term>>, Fail> {
    let start_id = tree.id(&start);
    let start = Rc::new(start);
    let mut found: HashSet<Id> = HashSet::new();
    let mut start_found = false;
    let mut reached = Vec::new();
    for walk in &reach.walks {
        let skipped: Vec<Id> = walk
            .skipped
            .iter()
            .filter_map(|edge| tree.id(edge))
            .collect();
        let mut seen: HashSet<Id> = HashSet::new();
        if walk.from_itself {
            seen.extend(start_id);
            let new = match start_id {
                Some(id) => found.insert(id),
                None => !std::mem::replace(&mut start_found, true),
            };
            if new {
                reached.push(Rc::clone(&start));
            }
        }
        let mut pending: Vec<Id> = start_id.into_iter().collect();
        while let Some(node) = pending.pop() {
            let (subject, object) = if walk.backwards {
                (None, Some(node))
            } else {
                (Some(node), None)
            };
            tree.scan(subject, None, object, &mut |from, edge, to| {
                let next = if walk.backwards { from } else { to };
                if !skipped.contains(&edge) && seen.insert(next) {
                    if found.insert(next) {
                        reached.push(Rc::clone(tree.term(next)));
                    }
                    pending.push(next);
                }
                Ok::<(), Fail>(())
            })?;
        }
    }
    Ok(reached)
}

fn unbind(row: &mut Row, slots: &[usize]) {
    for &slot in slots {
        row[slot] = None;
    }
}

fn joined(row: &mut Row, slots: &[usize], values: &[Option<Term>]) -> Option<Vec<usize>> {
    let mut set = Vec::new();
    for (&slot, value) in slots.iter().zip(values) {
        let Some(value) = value else { continue };
        match &row[slot] {
            Some(bound) if **bound != *value => {
                unbind(row, &set);
                return None;
            }
            Some(_) => {}
            None => {
                row[slot] = Some(Rc::new(value.clone()));
                set.push(slot);
            }
        }
    }
    Some(set)
}

fn scan(pattern: &Pattern, row: &mut Row, tree: &Tree, next: Sink<'_>) -> Result<(), Fail> {
    let Some((subject, predicate, object)) = fixed(pattern, row, tree)? else {
        return Ok(());
    };
    tree.scan(subject, predicate, object, &mut |s, p, o| {
        let Some(set) = unify(pattern, [s, p, o], row, tree) else {
            return Ok(());
        };
        let done = next(row);
        release(row, set);
        done
    })
}

type Fixed = (Option<Id>, Option<Id>, Option<Id>);

/// What a pattern fixes in the tree, or none where a part is a term the tree lacks.
fn fixed(pattern: &Pattern, row: &Row, tree: &Tree) -> Result<Option<Fixed>, Fail> {
    let (Some(subject), Some(predicate), Some(object)) = (
        id_of(&pattern.subject, row, tree)?,
        id_of(&pattern.predicate, row, tree)?,
        id_of(&pattern.object, row, tree)?,
    ) else {
        return Ok(None);
    };
    Ok(Some((subject, predicate, object)))
}

fn id_of(place: &Place, row: &Row, tree: &Tree) -> Result<Option<Option<Id>>, Fail> {
    Ok(match place {
        Place::Const(term) => tree.id(term).map(Some),
        Place::Blank(_) => unreachable!("only a template holds a blank node"),
        Place::Var(slot) => match &row[*slot] {
            Some(term) => tree.id(term).map(Some),
            None => Some(None),
        },
        Place::Key(slot, key) => match &row[*slot] {
            Some(term) => tree.id(term).map(Some),
            None => keyed(key, row, tree)?,
        },
    })
}

/// The one string a filter equating the variable with an expression admits. A value
/// of another kind is equal by value to terms the tree holds under other names, so
/// it fixes nothing and the filter decides.
fn keyed(key: &Key, row: &Row, tree: &Tree) -> Result<Option<Option<Id>>, Fail> {
    if key.reads.iter().any(|&slot| row[slot].is_none()) {
        return Ok(Some(None));
    }
    match (key.expr)(tree, row) {
        Ok(ExpressionTerm::StringLiteral(text)) => Ok(tree
            .id(&Term::from(Literal::new_simple_literal(text)))
            .map(Some)),
        Ok(_) => Ok(Some(None)),
        Err(Fail::Error) => Ok(None),
        Err(other) => Err(other),
    }
}

fn unify(
    pattern: &Pattern,
    ids: [Id; 3],
    row: &mut Row,
    tree: &Tree,
) -> Option<[Option<usize>; 3]> {
    let mut set = [None; 3];
    let places = [&pattern.subject, &pattern.predicate, &pattern.object];
    for (at, (place, id)) in places.into_iter().zip(ids).enumerate() {
        let (Place::Var(slot) | Place::Key(slot, _)) = place else {
            continue;
        };
        let term = tree.term(id);
        match &row[*slot] {
            Some(existing) if **existing != **term => {
                release(row, set);
                return None;
            }
            Some(_) => {}
            None => {
                row[*slot] = Some(Rc::clone(term));
                set[at] = Some(*slot);
            }
        }
    }
    Some(set)
}

fn release(row: &mut Row, set: [Option<usize>; 3]) {
    for slot in set.into_iter().flatten() {
        row[slot] = None;
    }
}

enum Accumulator {
    Count {
        counted: u64,
        seen: HashSet<Rc<Term>>,
    },
    /// Empty, or one value, or `Err` where an operand failed and the aggregate is unbound.
    Chosen(Picked),
    /// The strings so far, none until the first.
    Joined(Option<String>),
}

impl Accumulator {
    fn of(kind: &Kind) -> Self {
        match kind {
            Kind::Count { .. } => Self::Count {
                counted: 0,
                seen: HashSet::new(),
            },
            Kind::GroupConcat(..) => Self::Joined(None),
            Kind::Sum(_) => {
                Self::Chosen(Some(Ok(ExpressionTerm::IntegerLiteral(Integer::default()))))
            }
            _ => Self::Chosen(None),
        }
    }

    fn add(&mut self, kind: &Kind, tree: &Tree, row: &Row) -> Result<(), Fail> {
        match (self, kind) {
            (Self::Count { counted, seen }, Kind::Count { of, distinct }) => {
                *counted += u64::from(counts(seen, *of, *distinct, row));
                Ok(())
            }
            (Self::Chosen(chosen), Kind::Sample(expr)) => sample(chosen, expr, tree, row),
            (Self::Chosen(chosen), Kind::Sum(expr)) => sum(chosen, expr, tree, row),
            (Self::Chosen(chosen), Kind::Min(expr)) => {
                extreme(chosen, expr, Ordering::Less, tree, row)
            }
            (Self::Chosen(chosen), Kind::Max(expr)) => {
                extreme(chosen, expr, Ordering::Greater, tree, row)
            }
            (Self::Joined(joined), Kind::GroupConcat(expr, separator)) => match expr(tree, row) {
                Ok(ExpressionTerm::StringLiteral(text)) => {
                    if let Some(so_far) = joined.as_mut() {
                        so_far.push_str(separator);
                        so_far.push_str(&text);
                    } else {
                        *joined = Some(text);
                    }
                    Ok(())
                }
                Ok(_) | Err(Fail::Error) => Err(Fail::GiveUp),
                Err(other) => Err(other),
            },
            _ => unreachable!("an accumulator is made for its own kind"),
        }
    }

    fn finish(self) -> Option<Rc<Term>> {
        match self {
            Self::Count { counted, .. } => Some(Rc::new(Term::from(Literal::from(
                i64::try_from(counted).expect("fewer than 2^63 rows"),
            )))),
            Self::Chosen(Some(Ok(term))) => Some(Rc::new(Term::from(term))),
            Self::Chosen(_) => None,
            Self::Joined(joined) => Some(Rc::new(Term::from(Literal::from(
                joined.unwrap_or_default(),
            )))),
        }
    }
}

/// What a row is ordered by in one variable: SPARQL's order of blank nodes, then IRIs,
/// then strings, which is all the compiler sorts.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Sorted {
    Blank(String),
    Iri(String),
    Text(String),
}

fn sorted(term: &Option<Rc<Term>>) -> Result<Option<Sorted>, Fail> {
    match term.as_deref() {
        None => Ok(None),
        Some(Term::BlankNode(node)) => Ok(Some(Sorted::Blank(node.as_str().to_owned()))),
        Some(Term::NamedNode(node)) => Ok(Some(Sorted::Iri(node.as_str().to_owned()))),
        Some(Term::Literal(literal)) if literal.datatype() == xsd::STRING => {
            Ok(Some(Sorted::Text(literal.value().to_owned())))
        }
        Some(_) => Err(Fail::GiveUp),
    }
}

/// The rows in the order of the variables, each row after the rows equal to it that came
/// before it.
///
/// Where two rows tie on every key, the general evaluator's sort may put them in either order,
/// so `differ` says whether the order of two tied rows could show in the answer, and if it
/// could the query is given up.
fn ordered(
    order: &[(usize, bool)],
    rows: Vec<Row>,
    differ: &dyn Fn(&Row, &Row) -> Result<bool, Fail>,
) -> Result<Vec<Row>, Fail> {
    let mut keyed = rows
        .into_iter()
        .map(|row| {
            let key = order
                .iter()
                .map(|&(slot, _)| sorted(&row[slot]))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((key, row))
        })
        .collect::<Result<Vec<_>, Fail>>()?;
    let compare = |a: &Vec<Option<Sorted>>, b: &Vec<Option<Sorted>>| {
        a.iter()
            .zip(b)
            .zip(order)
            .map(|((a, b), &(_, descending))| if descending { b.cmp(a) } else { a.cmp(b) })
            .find(|ordering| ordering.is_ne())
            .unwrap_or(Ordering::Equal)
    };
    keyed.sort_by(|(a, _), (b, _)| compare(a, b));
    for pair in keyed.windows(2) {
        if compare(&pair[0].0, &pair[1].0).is_eq() && differ(&pair[0].1, &pair[1].1)? {
            return Err(Fail::GiveUp);
        }
    }
    Ok(keyed.into_iter().map(|(_, row)| row).collect())
}

fn counts(seen: &mut HashSet<Rc<Term>>, of: Option<usize>, distinct: bool, row: &Row) -> bool {
    match of.map(|slot| &row[slot]) {
        None => true,
        Some(None) => false,
        Some(Some(term)) => !distinct || seen.insert(Rc::clone(term)),
    }
}

fn sample(chosen: &mut Picked, expr: &Expr, tree: &Tree, row: &Row) -> Result<(), Fail> {
    match (expr(tree, row), &*chosen) {
        (Err(Fail::Error), _) => {}
        (Err(other), _) => return Err(other),
        (Ok(term), None) => *chosen = Some(Ok(term)),
        (Ok(term), Some(Ok(kept))) if term != *kept => return Err(Fail::GiveUp),
        (Ok(_), _) => {}
    }
    Ok(())
}

fn sum(chosen: &mut Picked, expr: &Expr, tree: &Tree, row: &Row) -> Result<(), Fail> {
    match (expr(tree, row), &*chosen) {
        (Err(Fail::Error), _) => *chosen = Some(Err(())),
        (Err(other), _) => return Err(other),
        (Ok(_), Some(Err(()))) => {}
        (
            Ok(ExpressionTerm::IntegerLiteral(term)),
            Some(Ok(ExpressionTerm::IntegerLiteral(sum))),
        ) => {
            *chosen = Some(
                sum.checked_add(term)
                    .map(ExpressionTerm::IntegerLiteral)
                    .ok_or(()),
            );
        }
        _ => return Err(Fail::GiveUp),
    }
    Ok(())
}

fn extreme(
    chosen: &mut Picked,
    expr: &Expr,
    wanted: Ordering,
    tree: &Tree,
    row: &Row,
) -> Result<(), Fail> {
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
    Ok(())
}

impl Group {
    fn run(&self, row: &mut Row, tree: &Tree, next: Sink<'_>) -> Result<(), Fail> {
        let mut groups = self.collected(row, tree)?;
        if groups.is_empty() && self.keys.is_empty() {
            groups.insert(Vec::new(), self.fresh());
        }
        for (key, accumulators) in groups {
            self.emit(key, accumulators, row, next)?;
        }
        Ok(())
    }

    fn fresh(&self) -> Vec<Accumulator> {
        self.aggregates
            .iter()
            .map(|aggregate| Accumulator::of(&aggregate.kind))
            .collect()
    }

    fn collected(&self, row: &mut Row, tree: &Tree) -> Result<Groups, Fail> {
        let mut groups = Groups::new();
        let mut add = |inner: &Row| -> Result<(), Fail> {
            let key = self.keys.iter().map(|&slot| inner[slot].clone()).collect();
            let accumulators = groups.entry(key).or_insert_with(|| self.fresh());
            for (accumulator, Aggregate { kind, .. }) in
                accumulators.iter_mut().zip(&self.aggregates)
            {
                accumulator.add(kind, tree, inner)?;
            }
            Ok(())
        };
        if self.order.is_empty() {
            run(&self.inner, row, tree, &mut |inner| add(inner))?;
        } else {
            let mut rows = Vec::new();
            run(&self.inner, row, tree, &mut |inner| {
                rows.push(inner.clone());
                Ok(())
            })?;
            let differ = |a: &Row, b: &Row| -> Result<bool, Fail> {
                for Aggregate { kind, .. } in &self.aggregates {
                    if let Kind::GroupConcat(expr, _) = kind {
                        if expr(tree, a)? != expr(tree, b)? {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            };
            for inner in ordered(&self.order, rows, &differ)? {
                add(&inner)?;
            }
        }
        Ok(groups)
    }

    fn emit(
        &self,
        key: Vec<Option<Rc<Term>>>,
        accumulators: Vec<Accumulator>,
        row: &mut Row,
        next: Sink<'_>,
    ) -> Result<(), Fail> {
        let mut set = Vec::new();
        for (&slot, value) in self.keys.iter().zip(key) {
            if row[slot].is_none() && value.is_some() {
                row[slot] = value;
                set.push(slot);
            }
        }
        for (aggregate, accumulator) in self.aggregates.iter().zip(accumulators) {
            row[aggregate.slot] = accumulator.finish();
            set.push(aggregate.slot);
        }
        let done = next(row);
        unbind(row, &set);
        done
    }
}
