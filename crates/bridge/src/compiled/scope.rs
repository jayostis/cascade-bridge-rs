use oxigraph::model::Variable;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::marker::PhantomData;

pub(super) mod analysis;
mod reads;
#[cfg(test)]
mod tests;

pub(super) use analysis::{analyze, Scoped};

pub(super) mod roles {
    pub(crate) enum Certain {}
    pub(crate) enum Maybe {}
    pub(crate) enum Own {}
    pub(crate) enum Outer {}
    pub(crate) enum Touched {}
    pub(crate) enum Row {}
    pub(crate) enum Hidden {}
    pub(crate) enum Reads {}
}

/// Slots tagged with the role they play, so that a set of one role cannot stand for another.
pub(super) struct Slots<Role>(BTreeSet<usize>, PhantomData<Role>);

/// Bound in every row.
pub(super) type Certain = Slots<roles::Certain>;
/// Bound in some row, which includes what is bound in every row.
pub(super) type Maybe = Slots<roles::Maybe>;
/// Bound in every row by a group's own patterns, as it would bind them alone.
pub(super) type Own = Slots<roles::Own>;
/// Bound, perhaps, by the groups around a group, which it does not see.
pub(super) type Outer = Slots<roles::Outer>;
/// Bound, in some row or every row, by a group's own patterns.
pub(super) type Touched = Slots<roles::Touched>;
/// Bound, perhaps, in the row an EXISTS body runs on.
pub(super) type Row = Slots<roles::Row>;
/// Read as unbound by a group although a group around it binds it.
pub(super) type Hidden = Slots<roles::Hidden>;
/// Mentioned by a pattern or read by an expression.
pub(super) type Reads = Slots<roles::Reads>;

impl<Role> Slots<Role> {
    pub(super) fn new() -> Self {
        Self(BTreeSet::new(), PhantomData)
    }

    pub(super) fn contains(&self, slot: usize) -> bool {
        self.0.contains(&slot)
    }

    pub(super) fn insert(&mut self, slot: usize) {
        self.0.insert(slot);
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().copied()
    }

    pub(super) fn intersection(&self, other: &Self) -> Self {
        self.0.intersection(&other.0).copied().collect()
    }

    pub(super) fn is_subset_of<Other>(&self, other: &Slots<Other>) -> bool {
        self.0.is_subset(&other.0)
    }

    pub(super) fn is_disjoint_from<Other>(&self, other: &Slots<Other>) -> bool {
        self.0.is_disjoint(&other.0)
    }
}

impl<Role> Default for Slots<Role> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Role> Clone for Slots<Role> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<Role> fmt::Debug for Slots<Role> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, formatter)
    }
}

impl<Role> FromIterator<usize> for Slots<Role> {
    fn from_iter<T: IntoIterator<Item = usize>>(slots: T) -> Self {
        Self(slots.into_iter().collect(), PhantomData)
    }
}

impl<Role> Extend<usize> for Slots<Role> {
    fn extend<T: IntoIterator<Item = usize>>(&mut self, slots: T) {
        self.0.extend(slots);
    }
}

impl<Role> IntoIterator for Slots<Role> {
    type Item = usize;
    type IntoIter = std::collections::btree_set::IntoIter<usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// The slot a variable has, in the scope it is named in.
pub(super) struct Names {
    next: usize,
    scopes: Vec<HashMap<String, usize>>,
}

impl Default for Names {
    fn default() -> Self {
        Self {
            next: 0,
            scopes: vec![HashMap::new()],
        }
    }
}

impl Names {
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

    pub(super) fn width(&self) -> usize {
        self.next
    }

    pub(super) fn enter(&mut self, scope: HashMap<String, usize>) {
        self.scopes.push(scope);
    }

    pub(super) fn leave(&mut self) -> HashMap<String, usize> {
        self.scopes.pop().unwrap_or_default()
    }
}

/// What an expression stands among: what is bound in every row and in some row where
/// it runs, and what the group it belongs to reads as unbound although a group around
/// it binds it.
pub(super) struct Scope {
    pub(super) certain: Certain,
    pub(super) maybe: Maybe,
    pub(super) hidden: Hidden,
}

/// What a sequence of patterns has bound so far.
#[derive(Clone, Default)]
pub(super) struct Bound {
    pub(super) certain: Certain,
    pub(super) maybe: Maybe,
    pub(super) own: Own,
}

impl Bound {
    pub(super) fn binds(&mut self, slot: usize, certainly: bool) {
        self.maybe.insert(slot);
        if certainly {
            self.certain.insert(slot);
            self.own.insert(slot);
        }
    }

    pub(super) fn scope(&self, hidden: Hidden) -> Scope {
        Scope {
            certain: self.certain.clone(),
            maybe: self.maybe.clone(),
            hidden,
        }
    }
}

pub(super) const READS_OUTER: &str = "a group reads a variable only the groups around it bind";

/// What a group of patterns has bound so far, and what stands around it.
/// - `bound.certain` and `bound.maybe` include what the groups around it bind.
/// - `bound.own` is what its own patterns bind in every row.
/// - `outer` is what may be bound by the groups around it, which it does not see: a
///   variable it reads that is in `outer` and not in `own` would be unbound here,
///   and a plan that carries a row inward would see the outer value.
/// - `touched` is what its own patterns may bind, in some row or every row.
/// - `row` is what the row an EXISTS body runs on may bind. A general evaluator runs the
///   whole body on that row, nested groups included, so they read these as bound.
#[derive(Default)]
pub(super) struct Frame {
    pub(super) bound: Bound,
    outer: Outer,
    touched: Touched,
    row: Row,
}

impl Frame {
    /// The frame of an EXISTS body, which runs on the row of the expression that holds it.
    pub(super) fn within_exists(scope: &Scope) -> Self {
        let visible = |slot: &usize| !scope.hidden.contains(*slot);
        let row: Row = scope.maybe.iter().filter(visible).collect();
        Self {
            bound: Bound {
                certain: scope.certain.iter().filter(visible).collect(),
                maybe: row.iter().collect(),
                own: Own::new(),
            },
            outer: Outer::new(),
            touched: Touched::new(),
            row,
        }
    }

    pub(super) fn child(&self) -> Self {
        Self {
            bound: Bound {
                certain: self.bound.certain.clone(),
                maybe: self.bound.maybe.clone(),
                own: Own::new(),
            },
            outer: self.bound.maybe.iter().collect(),
            touched: Touched::new(),
            row: self.row.clone(),
        }
    }

    pub(super) fn absorb(&mut self, child: Self) {
        self.bound.certain = child.bound.certain;
        self.bound.maybe = child.bound.maybe;
        self.bound.own.extend(child.bound.own);
        self.touched.extend(child.touched);
    }

    pub(super) fn binds(&mut self, slot: usize, certainly: bool) {
        self.bound.binds(slot, certainly);
        self.touched.insert(slot);
    }

    pub(super) fn maybe_binds(&mut self, slot: usize) {
        self.bound.maybe.insert(slot);
        self.touched.insert(slot);
    }

    pub(super) fn scope(&self, hidden: Hidden) -> Scope {
        self.bound.scope(hidden)
    }

    fn outside(&self, slot: usize) -> bool {
        self.outer.contains(slot) && !self.bound.own.contains(slot)
    }

    /// A group reads only what its own patterns bind, or what no group around it binds.
    pub(super) fn reads(&self, slots: &Reads) -> Result<(), &'static str> {
        match slots.iter().find(|&slot| self.outside(slot)) {
            Some(_) => Err(READS_OUTER),
            None => Ok(()),
        }
    }

    /// What a filter of this group reads that only the groups around it bind: a
    /// general evaluator keeps the group apart from them, so there it is unbound.
    /// Where the group may bind it as well, or an EXISTS body's row binds it, the row
    /// cannot say which it is.
    pub(super) fn hidden_among(&self, slots: &Reads) -> Result<Hidden, &'static str> {
        let mut hidden = Hidden::new();
        for slot in slots.iter().filter(|&slot| self.outside(slot)) {
            if self.touched.contains(slot) || self.row.contains(slot) {
                return Err(READS_OUTER);
            }
            hidden.insert(slot);
        }
        Ok(hidden)
    }
}
