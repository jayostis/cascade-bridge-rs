use crate::error::Result;
use oxigraph::model::Term;
use oxigraph::store::Store;
use spareval::ExpressionTerm;
use std::cell::OnceCell;
use std::rc::Rc;

mod expr;
mod lower;
mod run;
mod tree;

pub(crate) use lower::compile;
pub(crate) use run::Compiled;
pub(crate) use tree::Tree;

/// Which engine answers a query: the plan this module compiles it to, the general
/// evaluator, or both, refusing where they differ.
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

/// A unit's store, and the tree over it that a compiled plan reads, built when the
/// first plan asks.
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

    pub(crate) fn tree(&self) -> Result<&Tree> {
        if self.tree.get().is_none() {
            let _ = self.tree.set(Tree::of(self.store)?);
        }
        Ok(self.tree.get().expect("the tree was just set"))
    }
}

/// What a plan cannot answer itself. An error is SPARQL's, which a filter reads as
/// false. A give-up is a value no compiled builtin covers: the general evaluator
/// answers the query instead. `Done` ends a search that has found what it was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fail {
    Error,
    GiveUp,
    Done,
}

type Row = Vec<Option<Rc<Term>>>;
type Eval = std::result::Result<ExpressionTerm, Fail>;
type Expr = Box<dyn Fn(&Tree, &Row) -> Eval + Send + Sync>;
type Plan = Vec<Step>;

enum Place {
    Const(Term),
    Var(usize),
    Blank(usize),
}

struct Pattern {
    subject: Place,
    predicate: Place,
    object: Place,
}

enum Step {
    Triple(Pattern),
    Filter(Expr),
    Bind(usize, Expr),
    Values(Vec<usize>, Vec<Vec<Option<Term>>>),
    Union(Vec<Plan>),
    Optional(Plan, Option<Expr>),
    Group(Group),
    Reach(Reach),
}

/// A walk from a bound node over every edge but a few, to each node it reaches once.
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
}

enum Kind {
    Count { of: Option<usize>, distinct: bool },
    Sum(Expr),
    Min(Expr),
    Max(Expr),
    Sample(Expr),
}
