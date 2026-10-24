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
    minted: usize,
}

impl Default for Names {
    fn default() -> Self {
        Self {
            next: 0,
            scopes: vec![HashMap::new()],
            labels: None,
            minted: 0,
        }
    }
}

/// A name the parser made up: 128 random bits in hex.
fn minted(name: &str) -> bool {
    name.len() >= 24 && name.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
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
            let label = if minted(name) {
                self.minted += 1;
                format!("gen{}", self.minted - 1)
            } else {
                name.to_owned()
            };
            labels.push(if labels.contains(&label) {
                format!("{label}~{slot}")
            } else {
                label
            });
        }
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), slot);
        }
        slot
    }

    /// An expression's text with each variable of the current scope that is labelled
    /// otherwise than its name written as its label.
    pub(super) fn relabel(&self, text: &str) -> String {
        let (Some(labels), Some(scope)) = (&self.labels, self.scopes.last()) else {
            return text.to_owned();
        };
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(c) = rest.chars().next() {
            let skipped = match c {
                '"' => quoted(rest),
                '<' => iri(rest),
                _ => 0,
            };
            if skipped > 0 {
                out.push_str(&rest[..skipped]);
                rest = &rest[skipped..];
            } else if c == '?' || c == '$' {
                let end = rest[1..]
                    .find(|c: char| !name_char(c))
                    .map_or(rest.len(), |at| at + 1);
                let name = &rest[1..end];
                match scope.get(name).map(|&slot| &labels[slot]) {
                    Some(label) if label != name => {
                        out.push('?');
                        out.push_str(label);
                    }
                    _ => out.push_str(&rest[..end]),
                }
                rest = &rest[end..];
            } else {
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
        out
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

/// The length of the string literal `text` opens with, or 0 if it opens with none.
fn quoted(text: &str) -> usize {
    let mut escaped = false;
    for (at, c) in text.char_indices().skip(1) {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => return at + 1,
            _ => {}
        }
    }
    0
}

/// The length of the IRI `text` opens with, or 0 if it opens with an operator.
fn iri(text: &str) -> usize {
    match text.find(|c: char| c == '>' || c.is_whitespace()) {
        Some(at) if text[at..].starts_with('>') => at + 1,
        _ => 0,
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
/// - `several` is whether more than the one row a query starts from may reach it.
#[derive(Default)]
pub(super) struct Frame {
    pub(super) bound: Bound,
    outer: Maybe,
    touched: Maybe,
    row: Maybe,
    pub(super) several: bool,
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
            several: true,
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
            several: self.several,
        }
    }

    pub(super) fn absorb(&mut self, child: Self) {
        self.bound.certain = child.bound.certain;
        self.bound.maybe = child.bound.maybe;
        self.bound.own.extend(child.bound.own.iter());
        self.touched.extend(child.touched.iter());
        self.several |= child.several;
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
