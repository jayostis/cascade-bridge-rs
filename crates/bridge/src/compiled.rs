use crate::budget::Watch;
use crate::error::{Error, Result};
use crate::rdf::canonical_parts;
use oxigraph::model::{Quad, Term};
use oxigraph::store::Store;
use rustc_hash::FxHashMap;
use spareval::ExpressionTerm;
use std::cell::OnceCell;
use std::rc::Rc;

mod builtin;
mod explain;
mod expr;
mod keys;
mod lower;
mod run;
mod scope;
#[cfg(test)]
mod tests;
mod tree;

pub(crate) use lower::{compile, compile_explained};
pub(crate) use run::Compiled;
pub(crate) use tree::Tree;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Executor {
    #[default]
    Compiled,
    Spareval,
    Both,
}

impl Executor {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "compiled" => Some(Self::Compiled),
            "spareval" => Some(Self::Spareval),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

pub(crate) struct Dataset<'a> {
    pub(crate) store: &'a Store,
    tree: OnceCell<Tree>,
}

impl<'a> Dataset<'a> {
    pub(crate) fn new(store: &'a Store) -> Self {
        Self {
            store,
            tree: OnceCell::new(),
        }
    }

    pub(crate) fn tree(&self, watch: &Watch) -> Result<&Tree> {
        if self.tree.get().is_none() {
            let _ = self.tree.set(Tree::of(self.store, watch.clone())?);
        }
        Ok(self.tree.get().expect("the tree was just set"))
    }
}

/// What a plan cannot answer itself. An error is SPARQL's, which a filter reads as
/// false. A give-up is a value no compiled builtin covers: the general evaluator
/// answers the query instead. `Done` ends a search that has found what it was for.
/// `Stopped` ends the run, because a budget of the convert is spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fail {
    Error,
    GiveUp,
    Done,
    Stopped,
}

type Row = Vec<Option<Rc<Term>>>;
type Eval = std::result::Result<ExpressionTerm, Fail>;
type Expr = Box<dyn Fn(&Tree, &Row) -> Eval + Send + Sync>;
type Plan = Vec<Step>;

enum Place {
    Const(Term),
    Var(usize),
    Blank(usize),
    /// A variable a filter equates with an expression, looked up by its value.
    Key(usize, Key),
}

type Note = Option<Box<explain::Noted>>;

/// A lookup by the one string a filter equates the variable with, in place of a scan,
/// where `Hints` finds one safe.
struct Key {
    expr: Expr,
    reads: Vec<usize>,
    note: Note,
}

struct Pattern {
    subject: Place,
    predicate: Place,
    object: Place,
}

enum Step {
    Triple(Pattern),
    Filter(Expr, Note),
    Bind(usize, Expr, Note),
    Agree(usize, Expr, Note),
    Values(Vec<usize>, Vec<Vec<Option<Term>>>),
    Union(Vec<Plan>),
    Optional(Plan, Option<Expr>, Note),
    Group(Group),
    Reach(Reach),
    Choice(Vec<Edge>),
}

/// One edge a path that takes one of some edges may take, as the triple pattern it
/// matches, its ends swapped where it is taken backwards.
struct Edge {
    pattern: Pattern,
    backwards: bool,
}

struct Reach {
    from: Place,
    to: Place,
    skipped: Vec<Term>,
    from_itself: bool,
}

struct Group {
    inner: Plan,
    keys: Vec<usize>,
    aggregates: Vec<Aggregate>,
}

struct Aggregate {
    slot: usize,
    kind: Kind,
    note: Note,
}

enum Kind {
    Count { of: Option<usize>, distinct: bool },
    Sum(Expr),
    Min(Expr),
    Max(Expr),
    Sample(Expr),
}

/// Equal as graphs: each part no blank node reaches out of, compared on its own.
pub(crate) fn agree(query: &str, compiled: &[Quad], evaluated: &[Quad]) -> Result<()> {
    let counted = |quads: &[Quad]| -> Result<FxHashMap<String, usize>> {
        let mut counts = FxHashMap::default();
        for part in canonical_parts(quads.iter().cloned())? {
            *counts.entry(part).or_default() += 1;
        }
        Ok(counts)
    };
    let (mine, theirs) = (counted(compiled)?, counted(evaluated)?);
    let differing = mine
        .iter()
        .find(|(part, times)| theirs.get(*part) != Some(*times))
        .map(|(part, _)| ("the compiled plan", part))
        .or_else(|| {
            theirs
                .iter()
                .find(|(part, times)| mine.get(*part) != Some(*times))
                .map(|(part, _)| ("the general evaluator", part))
        });
    match differing {
        None => Ok(()),
        Some((whose, part)) => Err(Error::adapter(format!(
            "{query}: the compiled plan and the general evaluator differ; {whose} has {part} where the other has not as many"
        ))),
    }
}
