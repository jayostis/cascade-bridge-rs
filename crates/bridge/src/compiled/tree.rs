use crate::budget::Watch;
use crate::error::Result;
use oxigraph::model::{GraphName, Term};
use oxigraph::store::Store;
use rustc_hash::FxHashMap;
use std::rc::Rc;

pub(super) type Id = u32;

pub(crate) struct Tree {
    terms: Vec<Rc<Term>>,
    ids: FxHashMap<Rc<Term>, Id>,
    outgoing: Vec<Vec<(Id, Id)>>,
    incoming: Vec<Vec<(Id, Id)>>,
    by_predicate: Vec<Vec<(Id, Id)>>,
    by_predicate_object: FxHashMap<(Id, Id), Vec<Id>>,
    watch: Watch,
}

impl Tree {
    pub(crate) fn of(store: &Store, watch: Watch) -> Result<Self> {
        let mut terms: Vec<Rc<Term>> = Vec::new();
        let mut ids: FxHashMap<Rc<Term>, Id> = FxHashMap::default();
        let mut intern = |term: Term| -> Id {
            if let Some(&id) = ids.get(&term) {
                return id;
            }
            let id = Id::try_from(terms.len()).expect("fewer than 2^32 terms");
            let term = Rc::new(term);
            terms.push(Rc::clone(&term));
            ids.insert(term, id);
            id
        };
        let mut triples = Vec::new();
        for quad in store.iter() {
            let quad = quad?;
            if quad.graph_name == GraphName::DefaultGraph {
                let subject = intern(quad.subject.into());
                let predicate = intern(quad.predicate.into());
                triples.push((subject, predicate, intern(quad.object)));
            }
        }
        let mut tree = Self {
            outgoing: vec![Vec::new(); terms.len()],
            incoming: vec![Vec::new(); terms.len()],
            by_predicate: vec![Vec::new(); terms.len()],
            by_predicate_object: FxHashMap::default(),
            terms,
            ids,
            watch,
        };
        for (subject, predicate, object) in triples {
            tree.outgoing[subject as usize].push((predicate, object));
            tree.incoming[object as usize].push((subject, predicate));
            tree.by_predicate[predicate as usize].push((subject, object));
            tree.by_predicate_object
                .entry((predicate, object))
                .or_default()
                .push(subject);
        }
        Ok(tree)
    }

    pub(super) fn stopped(&self) -> bool {
        self.watch.pulse()
    }

    pub(super) fn id(&self, term: &Term) -> Option<Id> {
        self.ids.get(term).copied()
    }

    pub(super) fn term(&self, id: Id) -> &Rc<Term> {
        &self.terms[id as usize]
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
        match (subject, predicate, object) {
            (Some(s), p, o) => {
                for &(pp, oo) in &self.outgoing[s as usize] {
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
                for &(s, pp) in &self.incoming[o as usize] {
                    if p.is_none_or(|p| p == pp) {
                        each(s, pp, o)?;
                    }
                }
            }
            (None, Some(p), None) => {
                for &(s, o) in &self.by_predicate[p as usize] {
                    each(s, p, o)?;
                }
            }
            (None, None, None) => {}
        }
        Ok(())
    }
}
