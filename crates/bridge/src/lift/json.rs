// The whole document is read before any record is handed over: which envelope admits
// it, and so which path splits it, is said by its value's members.
use super::{
    admitting, member, name, store_of, triple, Admission, Occurrence, Paths, Reading, Unit, Valued,
    FX, RDF, XYZ,
};
use crate::error::{Error, Result};
use crate::json::{self, Node, Value};
use oxigraph::model::{BlankNode, Literal, NamedNode, Quad, Term};
use oxigraph::store::Store;
use std::collections::{HashMap, HashSet};

pub(crate) struct Lift<'a> {
    text: &'a str,
    root: Node,
    records: Vec<Vec<usize>>,
    next: usize,
    paths: Paths,
}

pub(crate) fn lift<'a>(text: &'a str, reading: &Reading<'_>, paths: Paths) -> Result<Lift<'a>> {
    let root = json::parse(text)?;
    if !root.is_container() {
        return Err(Error::msg(
            "the document's value is neither an object nor an array, and the JSON lift lifts \
             nothing from it",
        ));
    }
    let mut lift = Lift {
        text,
        root,
        records: Vec::new(),
        next: 0,
        paths,
    };
    let path = admitting(&lift, reading)
        .and_then(|chosen| reading.envelopes.get(chosen))
        .and_then(|admission| admission.records.as_deref());
    if let Some(path) = path {
        lift.records = json::records(&lift.root, &json::path(path)?);
    }
    Ok(lift)
}

/// Fresh blank nodes, and the triples written from them.
#[derive(Default)]
struct Written {
    quads: Vec<Quad>,
    next: usize,
}

impl Written {
    fn fresh(&mut self) -> BlankNode {
        self.next += 1;
        BlankNode::new_unchecked(format!("j{}", self.next))
    }

    /// A container's node; one in `emptied` keeps its scalar members alone.
    fn container(&mut self, node: &Node, emptied: &HashSet<*const Node>) -> Result<BlankNode> {
        let id = self.fresh();
        let emptying = emptied.contains(&std::ptr::from_ref(node));
        let children: Vec<(NamedNode, &Node)> = match &node.value {
            Value::Object(members) => members
                .iter()
                .map(|(named, member)| Ok((name(XYZ, named)?, member)))
                .collect::<Result<_>>()?,
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(index, item)| Ok((member(index + 1)?, item)))
                .collect::<Result<_>>()?,
            _ => Vec::new(),
        };
        for (predicate, child) in children {
            let object = match child.scalar() {
                Some(text) => Term::from(Literal::new_simple_literal(text)),
                None if child.is_container() && !emptying => {
                    Term::from(self.container(child, emptied)?)
                }
                None => continue,
            };
            self.quads.push(triple(&id, predicate, object));
        }
        Ok(id)
    }

    fn root(mut self, node: &Node, emptied: &HashSet<*const Node>) -> Result<Vec<Quad>> {
        let id = self.container(node, emptied)?;
        self.quads.push(triple(
            &id,
            NamedNode::new(format!("{RDF}type"))?,
            NamedNode::new(format!("{FX}root"))?,
        ));
        Ok(self.quads)
    }
}

/// The lift of a whole document's value.
#[cfg(test)]
pub(crate) fn lifted(text: &str) -> Result<Vec<Quad>> {
    Written::default().root(&json::parse(text)?, &HashSet::new())
}

impl super::Lift for Lift<'_> {
    fn next_unit(&mut self) -> Result<Option<Unit>> {
        let Some(positions) = self.records.get(self.next) else {
            return Ok(None);
        };
        self.next += 1;
        let record = self.root.at(positions).expect("a record the path selected");
        let (occurrences, values) = match &self.paths {
            Paths::Kept { valued } => census(record, valued),
            Paths::Dropped => (Vec::new(), Vec::new()),
        };
        Ok(Some(Unit {
            store: store_of(Written::default().root(record, &HashSet::new())?)?,
            text: self.text[record.span.clone()].to_owned(),
            selector: json::pointer(&json::tokens_at(&self.root, positions)),
            occurrences,
            values,
        }))
    }

    fn admits(&self, admission: &Admission) -> bool {
        let (Value::Object(members), Some(root)) = (&self.root.value, &admission.root) else {
            return false;
        };
        members.iter().any(|(named, member)| {
            named == root
                && admission
                    .value
                    .as_deref()
                    .is_none_or(|value| member.scalar() == Some(value))
        })
    }

    fn document_selector(&self, _described: Option<&str>) -> String {
        String::new()
    }

    fn into_skeleton(self: Box<Self>) -> Result<Store> {
        let emptied: HashSet<*const Node> = self
            .records
            .iter()
            .filter_map(|positions| self.root.at(positions))
            .map(std::ptr::from_ref)
            .collect();
        store_of(Written::default().root(&self.root, &emptied)?)
    }
}

#[derive(Default)]
struct Census {
    seen: HashMap<String, usize>,
    occurrences: Vec<Occurrence>,
    held: HashMap<(String, String), usize>,
    values: Vec<Valued>,
}

/// A path's step is a member's name as a JSON Pointer reference token; an array's
/// items stand at its own path.
fn census(record: &Node, valued: &HashSet<String>) -> (Vec<Occurrence>, Vec<Valued>) {
    let mut census = Census::default();
    census.members(record, "", &mut Vec::new(), valued);
    (census.occurrences, census.values)
}

impl Census {
    fn members(
        &mut self,
        node: &Node,
        path: &str,
        tokens: &mut Vec<String>,
        valued: &HashSet<String>,
    ) {
        let Value::Object(members) = &node.value else {
            return;
        };
        for (named, member) in members {
            let path = format!("{path}/{}", named.replace('~', "~0").replace('/', "~1"));
            tokens.push(named.clone());
            self.stand(member, &path, tokens, valued);
            tokens.pop();
        }
    }

    fn stand(
        &mut self,
        node: &Node,
        path: &str,
        tokens: &mut Vec<String>,
        valued: &HashSet<String>,
    ) {
        match &node.value {
            Value::Null => {}
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    tokens.push(index.to_string());
                    self.stand(item, path, tokens, valued);
                    tokens.pop();
                }
            }
            _ => {
                let within = json::pointer(tokens);
                if let Some(value) = node.scalar().filter(|_| valued.contains(path)) {
                    self.hold(path, value, &within);
                }
                self.add(path, within);
                self.members(node, path, tokens, valued);
            }
        }
    }

    fn add(&mut self, path: &str, within: String) {
        match self.seen.get(path) {
            Some(&first) => self.occurrences[first].count += 1,
            None => {
                self.seen.insert(path.to_owned(), self.occurrences.len());
                self.occurrences.push(Occurrence {
                    path: path.to_owned(),
                    within: Some(within),
                    count: 1,
                });
            }
        }
    }

    fn hold(&mut self, path: &str, value: &str, within: &str) {
        let held = (path.to_owned(), value.to_owned());
        match self.held.get(&held) {
            Some(&first) => self.values[first].count += 1,
            None => {
                self.held.insert(held, self.values.len());
                self.values.push(Valued {
                    path: path.to_owned(),
                    value: value.to_owned(),
                    within: Some(within.to_owned()),
                    count: 1,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod vectors;
