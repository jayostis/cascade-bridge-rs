use oxigraph::model::Term;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// A group's keys and then its aggregates, as one group of the answer holds them.
pub(super) type Tuple = Vec<Option<Rc<Term>>>;

type Index = FxHashMap<Vec<Rc<Term>>, Vec<usize>>;

/// What a group answers for the whole tree, and for each set of keys a row may bind,
/// the tuples that hold those keys, found by an index built the first time it is asked.
pub(super) struct Answer {
    tuples: Vec<Tuple>,
    indexes: RefCell<FxHashMap<Vec<usize>, Option<Index>>>,
}

impl Answer {
    pub(super) fn of(tuples: Vec<Tuple>) -> Self {
        Self {
            tuples,
            indexes: RefCell::default(),
        }
    }

    pub(super) fn tuple(&self, at: usize) -> &Tuple {
        &self.tuples[at]
    }

    /// The positions of the tuples that hold the keys a row binds, each given by its
    /// position among the keys: every tuple where no index can say, as a tuple that
    /// leaves a key unbound agrees with any value of it.
    pub(super) fn agreeing(&self, bound: &[(usize, Rc<Term>)]) -> Vec<usize> {
        if bound.is_empty() {
            return (0..self.tuples.len()).collect();
        }
        let positions: Vec<usize> = bound.iter().map(|(at, _)| *at).collect();
        let mut indexes = self.indexes.borrow_mut();
        let index = indexes
            .entry(positions)
            .or_insert_with(|| self.indexed(bound));
        match index {
            Some(index) => index
                .get(
                    &bound
                        .iter()
                        .map(|(_, term)| Rc::clone(term))
                        .collect::<Vec<_>>(),
                )
                .cloned()
                .unwrap_or_default(),
            None => (0..self.tuples.len()).collect(),
        }
    }

    fn indexed(&self, bound: &[(usize, Rc<Term>)]) -> Option<Index> {
        let mut index = Index::default();
        for (at, tuple) in self.tuples.iter().enumerate() {
            let held: Option<Vec<Rc<Term>>> =
                bound.iter().map(|(key, _)| tuple[*key].clone()).collect();
            index.entry(held?).or_default().push(at);
        }
        Some(index)
    }
}
