use oxigraph::model::Variable;
use std::collections::HashMap;

pub(super) mod analysis;
mod reads;
mod slots;
#[cfg(test)]
mod tests;

pub(super) use analysis::{analyze, Scoped};
pub(super) use slots::{Certain, Hidden, Maybe, Reads, Slots};

/// The slot a variable has, in the scope it is named in.
pub(super) struct Names {
    next: usize,
    scopes: Vec<HashMap<String, usize>>,
    labels: Option<Vec<String>>,
}

impl Default for Names {
    fn default() -> Self {
        Self {
            next: 0,
            scopes: vec![HashMap::new()],
            labels: None,
        }
    }
}

impl Names {
    pub(super) fn explaining() -> Self {
        Self {
            labels: Some(Vec::new()),
            ..Self::default()
        }
    }

    pub(super) fn is_explaining(&self) -> bool {
        self.labels.is_some()
    }

    pub(super) fn labels(&self) -> &[String] {
        self.labels.as_deref().unwrap_or_default()
    }

    pub(super) fn into_labels(self) -> Vec<String> {
        self.labels.unwrap_or_default()
    }

    pub(super) fn slot(&mut self, variable: &Variable) -> usize {
        let name = variable.as_str();
        if let Some(&slot) = self.scopes.last().and_then(|scope| scope.get(name)) {
            return slot;
        }
        let slot = self.next;
        self.next += 1;
        if let Some(labels) = &mut self.labels {
            labels.push(if labels.iter().any(|label| label == name) {
                format!("{name}~{slot}")
            } else {
                name.to_owned()
            });
        }
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
    pub(super) own: Certain,
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
/// - `bound.own` is what its own patterns bind in every row.
/// - `outer` is what the groups around it may bind.
/// - `touched` is what its own patterns may bind.
/// - `row` is what the row an EXISTS body runs on may bind.
#[derive(Default)]
pub(super) struct Frame {
    pub(super) bound: Bound,
    outer: Maybe,
    touched: Maybe,
    row: Maybe,
}

impl Frame {
    pub(super) fn within_exists(scope: &Scope) -> Self {
        let visible = |slot: &usize| !scope.hidden.contains(*slot);
        let row: Maybe = scope.maybe.iter().filter(visible).collect();
        Self {
            bound: Bound {
                certain: scope.certain.iter().filter(visible).collect(),
                maybe: row.iter().collect(),
                own: Certain::new(),
            },
            outer: Maybe::new(),
            touched: Maybe::new(),
            row,
        }
    }

    pub(super) fn child(&self) -> Self {
        Self {
            bound: Bound {
                certain: self.bound.certain.clone(),
                maybe: self.bound.maybe.clone(),
                own: Certain::new(),
            },
            outer: self.bound.maybe.iter().collect(),
            touched: Maybe::new(),
            row: self.row.clone(),
        }
    }

    pub(super) fn absorb(&mut self, child: Self) {
        self.bound.certain = child.bound.certain;
        self.bound.maybe = child.bound.maybe;
        self.bound.own.extend(child.bound.own.iter());
        self.touched.extend(child.touched.iter());
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

    pub(super) fn reads(&self, slots: &Reads) -> Result<(), &'static str> {
        match slots.iter().find(|&slot| self.outside(slot)) {
            Some(_) => Err(READS_OUTER),
            None => Ok(()),
        }
    }

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
