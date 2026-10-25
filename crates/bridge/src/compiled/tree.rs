use super::Fail;
use crate::budget::Watch;
use crate::error::Result;
use oxigraph::model::{GraphName, Quad, Term};
use oxigraph::store::Store;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) type Id = u32;

/// The triples every record of a document is given beside its own: indexed once
/// per document, not once per record.
pub(crate) struct Layer {
    first: Id,
    terms: Vec<Rc<Term>>,
    ids: FxHashMap<Rc<Term>, Id>,
    outgoing: FxHashMap<Id, Vec<(Id, Id)>>,
    incoming: FxHashMap<Id, Vec<(Id, Id)>>,
    by_predicate: FxHashMap<Id, Vec<(Id, Id)>>,
    by_predicate_object: FxHashMap<(Id, Id), Vec<Id>>,
}

impl Layer {
    pub(crate) fn of(quads: &[Quad]) -> Self {
        Self::over(None, quads.iter().cloned().map(Ok))
            .expect("triples from memory are read without error")
    }

    fn over(under: Option<&Layer>, quads: impl Iterator<Item = Result<Quad>>) -> Result<Self> {
        let first = under.map_or(0, |under| {
            under.first + Id::try_from(under.terms.len()).expect("fewer than 2^32 terms")
        });
        let mut layer = Self {
            first,
            terms: Vec::new(),
            ids: FxHashMap::default(),
            outgoing: FxHashMap::default(),
            incoming: FxHashMap::default(),
            by_predicate: FxHashMap::default(),
            by_predicate_object: FxHashMap::default(),
        };
        let mut seen = FxHashSet::default();
        for quad in quads {
            let quad = quad?;
            if quad.graph_name != GraphName::DefaultGraph {
                continue;
            }
            let triple = (
                layer.intern(under, quad.subject.into()),
                layer.intern(under, quad.predicate.into()),
                layer.intern(under, quad.object),
            );
            let held_under = under.is_some_and(|under| under.holds(triple));
            if !held_under && seen.insert(triple) {
                layer.add(triple);
            }
        }
        Ok(layer)
    }

    fn intern(&mut self, under: Option<&Layer>, term: Term) -> Id {
        if let Some(id) = under.and_then(|under| under.ids.get(&term)) {
            return *id;
        }
        if let Some(&id) = self.ids.get(&term) {
            return id;
        }
        let id = self.first + Id::try_from(self.terms.len()).expect("fewer than 2^32 terms");
        let term = Rc::new(term);
        self.terms.push(Rc::clone(&term));
        self.ids.insert(term, id);
        id
    }

    fn add(&mut self, (subject, predicate, object): (Id, Id, Id)) {
        self.outgoing
            .entry(subject)
            .or_default()
            .push((predicate, object));
        self.incoming
            .entry(object)
            .or_default()
            .push((subject, predicate));
        self.by_predicate
            .entry(predicate)
            .or_default()
            .push((subject, object));
        self.by_predicate_object
            .entry((predicate, object))
            .or_default()
            .push(subject);
    }

    fn holds(&self, (subject, predicate, object): (Id, Id, Id)) -> bool {
        self.outgoing
            .get(&subject)
            .is_some_and(|edges| edges.contains(&(predicate, object)))
    }

    fn scan<E>(
        &self,
        subject: Option<Id>,
        predicate: Option<Id>,
        object: Option<Id>,
        each: &mut dyn FnMut(Id, Id, Id) -> std::result::Result<(), E>,
    ) -> std::result::Result<(), E> {
        match (subject, predicate, object) {
            (Some(s), p, o) => {
                for &(pp, oo) in self.outgoing.get(&s).into_iter().flatten() {
                    if p.is_none_or(|p| p == pp) && o.is_none_or(|o| o == oo) {
                        each(s, pp, oo)?;
                    }
                }
            }
            (None, Some(p), Some(o)) => {
                for &s in self.by_predicate_object.get(&(p, o)).into_iter().flatten() {
                    each(s, p, o)?;
                }
            }
            (None, p, Some(o)) => {
                for &(s, pp) in self.incoming.get(&o).into_iter().flatten() {
                    if p.is_none_or(|p| p == pp) {
                        each(s, pp, o)?;
                    }
                }
            }
            (None, Some(p), None) => {
                for &(s, o) in self.by_predicate.get(&p).into_iter().flatten() {
                    each(s, p, o)?;
                }
            }
            (None, None, None) => {}
        }
        Ok(())
    }
}

/// A record's triples over the document's shared ones, a triple in exactly one of them.
pub(crate) struct Tree {
    shared: Option<Rc<Layer>>,
    own: Layer,
    answers: RefCell<FxHashMap<usize, bool>>,
    watch: Watch,
}

impl Tree {
    #[cfg(test)]
    pub(crate) fn of(store: &Store, watch: Watch) -> Result<Self> {
        Self::over(None, store, watch)
    }

    pub(crate) fn over(shared: Option<Rc<Layer>>, store: &Store, watch: Watch) -> Result<Self> {
        let own = Layer::over(shared.as_deref(), store.iter().map(|quad| Ok(quad?)))?;
        Ok(Self {
            shared,
            own,
            answers: RefCell::default(),
            watch,
        })
    }

    pub(super) fn stopped(&self) -> bool {
        self.watch.pulse()
    }

    pub(super) fn id(&self, term: &Term) -> Option<Id> {
        self.own
            .ids
            .get(term)
            .or_else(|| self.shared.as_ref()?.ids.get(term))
            .copied()
    }

    pub(super) fn term(&self, id: Id) -> &Rc<Term> {
        match &self.shared {
            Some(shared) if id < self.own.first => &shared.terms[id as usize],
            _ => &self.own.terms[(id - self.own.first) as usize],
        }
    }

    pub(super) fn forget_answers(&self) {
        self.answers.take();
    }

    /// An answer that depends on the tree alone, computed the first time it is asked.
    pub(super) fn remembered(
        &self,
        question: usize,
        compute: impl FnOnce() -> std::result::Result<bool, Fail>,
    ) -> std::result::Result<bool, Fail> {
        if let Some(&answer) = self.answers.borrow().get(&question) {
            return Ok(answer);
        }
        let answer = compute()?;
        self.answers.borrow_mut().insert(question, answer);
        Ok(answer)
    }

    /// Every triple that agrees with what is fixed, by the narrowest index that
    /// answers it. Nothing fixed is not a lookup, so the lowering never plans it.
    pub(super) fn scan<E>(
        &self,
        subject: Option<Id>,
        predicate: Option<Id>,
        object: Option<Id>,
        each: &mut dyn FnMut(Id, Id, Id) -> std::result::Result<(), E>,
    ) -> std::result::Result<(), E> {
        if let Some(shared) = &self.shared {
            shared.scan(subject, predicate, object, each)?;
        }
        self.own.scan(subject, predicate, object, each)
    }
}
