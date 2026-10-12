use crate::error::{Error, Result};
use crate::rdf::canonical_parts;
use oxigraph::model::{Quad, Term};
use oxigraph::store::Store;
use rustc_hash::FxHashMap;
use spareval::ExpressionTerm;
use std::cell::OnceCell;
use std::rc::Rc;

mod builtin;
mod expr;
mod lower;
mod run;
#[cfg(test)]
mod tests;
mod tree;

pub(crate) use lower::compile;
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
