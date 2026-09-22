// Following an address, rather than only writing one. A finding's address is
// an XPath, and what the finding is about is the node that XPath selects, so
// an address selecting no node, or several, is reported where a conversion
// wrote it and fails the entry where a comparison reads it, and two findings
// whose addresses reach one node are compared as one however either is
// spelled.
//
// The tree is parsed here rather than read out of the lifted store, because
// XPath counts what the lift drops: a comment, a processing instruction and a
// whitespace-only text node each take a place among their siblings, so a
// positional step read off the store would stand at a different node. The
// characters are decoded exactly as the lift decodes them, so an address is
// followed through the text the lift saw.
//
// An address selects one node of what it is read from: a refinement one node
// of its record, a record's own selector one node of the document. So a
// conversion follows a refinement through the record read as a document of its
// own, which the lift writes out with the comments and instructions XPath
// counts, and holds one record where it would otherwise hold the document. A
// comparison also reads a record's selector, which is absolute, and is the one
// stage here that builds a tree of the whole document.
use crate::annotation::{self, Record};
use crate::decode::{normalise_attribute_value, normalise_line_endings};
use crate::error::Result;
use crate::lift::Step;
use crate::rdf::{OA_HAS_SELECTOR, OA_REFINED_BY, RDF_VALUE};
use oxrdf::{BlankNode, Literal, NamedOrBlankNode, Quad, Term};
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use std::cell::{OnceCell, RefCell};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;
use xpath_eval::{
    evaluate, parse, Document, EvaluationContext, ExpandedName, Expr, Node, NodeKind, Value,
};

/// The document node, which is where an absolute address is read from and the
/// first node of any tree.
const DOCUMENT: usize = 0;

struct Data {
    kind: NodeKind,
    name: Option<ExpandedName>,
    /// Where it stands among the siblings one step of an address counts, which
    /// is the place that step carries.
    position: usize,
    text: String,
    parent: Option<usize>,
    children: Vec<usize>,
    attributes: Vec<usize>,
}

impl Data {
    fn of(kind: NodeKind, parent: Option<usize>) -> Self {
        Self {
            kind,
            name: None,
            position: 1,
            text: String::new(),
            parent,
            children: Vec::new(),
            attributes: Vec::new(),
        }
    }

    fn named(mut self, namespace: Option<String>, local: String) -> Self {
        self.name = Some(ExpandedName {
            namespace_uri: namespace,
            local_name: local,
        });
        self
    }

    fn saying(mut self, text: String) -> Self {
        self.text = text;
        self
    }
}

/// What one step of an address names a node by: an element by its expanded
/// name, and every other node by its kind, a node test naming no other.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Among {
    Named(Option<String>, String),
    Kind(NodeKind),
}

fn among(data: &Data) -> Among {
    match (data.kind, &data.name) {
        (NodeKind::Element, Some(name)) => {
            Among::Named(name.namespace_uri.clone(), name.local_name.clone())
        }
        (kind, _) => Among::Kind(kind),
    }
}

/// Where each node stands among the siblings its own node test counts, walked
/// once for the whole tree so that no later walk of it counts siblings again.
fn place(nodes: &mut [Data]) {
    for at in 0..nodes.len() {
        let children = std::mem::take(&mut nodes[at].children);
        let mut counted: HashMap<Among, usize> = HashMap::new();
        for &child in &children {
            let seen = counted.entry(among(&nodes[child])).or_default();
            *seen += 1;
            nodes[child].position = *seen;
        }
        nodes[at].children = children;
    }
}

/// One document as XPath counts it, held in one vector in document order: a
/// node is an index into it, and one index against another is document order.
struct Tree {
    nodes: Vec<Data>,
    /// The tree's own element: a document's document element, and a record's
    /// own element where the record is the document.
    element: usize,
}

/// Each address as the parser read it, kept because an address written once in
/// an adapter is followed once for every record of the document, and it is the
/// same expression each time. It stands outside the tree, a record being one
/// tree and the addresses of every record one set of expressions.
#[derive(Default)]
struct Parsed(RefCell<HashMap<String, Option<Rc<Expr>>>>);

impl Parsed {
    /// The expression an address is, or nothing at all where it is no XPath.
    fn expression(&self, written: &str) -> Option<Rc<Expr>> {
        if let Some(expression) = self.0.borrow().get(written) {
            return expression.clone();
        }
        #[cfg(test)]
        PARSED.with(|count| count.set(count.get() + 1));
        let expression = parse(written).ok().map(Rc::new);
        self.0
            .borrow_mut()
            .insert(written.to_owned(), expression.clone());
        expression
    }
}

/// An owned name, where the parser bound one.
fn resolved(namespace: ResolveResult) -> Option<Option<String>> {
    match namespace {
        ResolveResult::Bound(bound) => Some(Some(std::str::from_utf8(bound.as_ref()).ok()?.into())),
        _ => Some(None),
    }
}

impl Tree {
    /// The document, or nothing at all where it is not one tree: a stage that
    /// reports never refuses, so a document no tree can be built from is a
    /// document this says nothing about.
    fn of(xml: &str) -> Option<Self> {
        let mut reader = NsReader::from_str(xml);
        reader.config_mut().expand_empty_elements = true;
        let mut nodes = vec![Data::of(NodeKind::Root, None)];
        let mut open: Vec<usize> = vec![DOCUMENT];
        // The text node still taking characters. Adjacent character data is one
        // text node however many events it arrived in, and a node standing
        // between two runs of character data leaves them not adjacent, so the
        // run after it begins a second text node.
        let mut taking: Option<usize> = None;
        loop {
            let (namespace, event) = reader.read_resolved_event().ok()?;
            let namespace = resolved(namespace)?;
            match event {
                Event::Start(start) => {
                    taking = None;
                    let local = std::str::from_utf8(start.local_name().as_ref())
                        .ok()?
                        .to_owned();
                    let parent = *open.last()?;
                    let at = nodes.len();
                    nodes.push(Data::of(NodeKind::Element, Some(parent)).named(namespace, local));
                    nodes[parent].children.push(at);
                    for attribute in start.attributes() {
                        let attribute = attribute.ok()?;
                        let key = attribute.key;
                        // A namespace declaration is no attribute of the
                        // element it stands on.
                        if key.as_ref() == b"xmlns" || key.as_ref().starts_with(b"xmlns:") {
                            continue;
                        }
                        let (bound, local) = reader.resolve_attribute(key);
                        let namespace = resolved(bound)?;
                        let local = std::str::from_utf8(local.as_ref()).ok()?.to_owned();
                        let raw = std::str::from_utf8(&attribute.value).ok()?;
                        let value = quick_xml::escape::unescape(&normalise_attribute_value(raw))
                            .ok()?
                            .into_owned();
                        let carried = nodes.len();
                        nodes.push(
                            Data::of(NodeKind::Attribute, Some(at))
                                .named(namespace, local)
                                .saying(value),
                        );
                        nodes[at].attributes.push(carried);
                    }
                    open.push(at);
                }
                Event::End(_) => {
                    taking = None;
                    if open.len() < 2 {
                        return None;
                    }
                    open.pop();
                }
                Event::Text(text) => {
                    let raw = text.into_inner();
                    let raw = std::str::from_utf8(&raw).ok()?;
                    let characters = quick_xml::escape::unescape(&normalise_line_endings(raw))
                        .ok()?
                        .into_owned();
                    say(&mut nodes, &mut taking, &open, characters);
                }
                Event::CData(data) => {
                    let raw = data.into_inner();
                    let raw = std::str::from_utf8(&raw).ok()?;
                    let characters = normalise_line_endings(raw).into_owned();
                    say(&mut nodes, &mut taking, &open, characters);
                }
                Event::Comment(comment) => {
                    taking = None;
                    let raw = comment.into_inner();
                    let text = std::str::from_utf8(&raw).ok()?.to_owned();
                    let parent = *open.last()?;
                    let at = nodes.len();
                    nodes.push(Data::of(NodeKind::Comment, Some(parent)).saying(text));
                    nodes[parent].children.push(at);
                }
                Event::PI(instruction) => {
                    taking = None;
                    let target = std::str::from_utf8(instruction.target()).ok()?.to_owned();
                    let content = std::str::from_utf8(instruction.content()).ok()?.to_owned();
                    let parent = *open.last()?;
                    let at = nodes.len();
                    nodes.push(
                        Data::of(NodeKind::ProcessingInstruction, Some(parent))
                            .named(None, target)
                            .saying(content),
                    );
                    nodes[parent].children.push(at);
                }
                Event::Eof => break,
                _ => {}
            }
        }
        if open.len() != 1 {
            return None;
        }
        let mut elements = nodes[DOCUMENT]
            .children
            .iter()
            .copied()
            .filter(|&child| nodes[child].kind == NodeKind::Element);
        let element = elements.next()?;
        if elements.next().is_some() {
            return None;
        }
        place(&mut nodes);
        Some(Self { nodes, element })
    }

    fn at(&self, at: usize) -> Handle<'_> {
        Handle { tree: self, at }
    }

    /// One record of this tree as the document of its own that a refinement is
    /// followed through, which is the tree a conversion builds from the record
    /// the lift writes out again. A comparison reads a record's own selector,
    /// which only the document can answer, and reads each refinement of it
    /// here, so neither stage can answer an address the other reports.
    fn record(&self, at: usize) -> Self {
        let mut nodes = vec![Data::of(NodeKind::Root, None)];
        let element = nodes.len();
        self.copy(at, DOCUMENT, &mut nodes);
        place(&mut nodes);
        Self { nodes, element }
    }

    /// A node and everything below it, in document order: a node's attributes
    /// stand before its children, as the parser reads them.
    fn copy(&self, from: usize, parent: usize, nodes: &mut Vec<Data>) {
        let source = &self.nodes[from];
        let at = nodes.len();
        let mut data = Data::of(source.kind, Some(parent));
        data.name.clone_from(&source.name);
        data.text.clone_from(&source.text);
        nodes.push(data);
        match source.kind {
            NodeKind::Attribute => nodes[parent].attributes.push(at),
            _ => nodes[parent].children.push(at),
        }
        for &attribute in &source.attributes {
            self.copy(attribute, at, nodes);
        }
        for &child in &source.children {
            self.copy(child, at, nodes);
        }
    }

    /// Whether a node is one of the node an address was read from: a
    /// refinement selects one node of its record, and a node standing outside
    /// the record is no node of it however an address reaches it. A record
    /// read on its own carries a document node of its own, which the record
    /// does not hold either.
    fn holds(&self, from: usize, at: usize) -> bool {
        let mut walk = Some(at);
        while let Some(node) = walk {
            if node == from {
                return true;
            }
            walk = self.nodes[node].parent;
        }
        false
    }

    fn step(&self, at: usize) -> Option<Step> {
        let name = self.nodes[at].name.as_ref()?;
        Some(Step {
            local: name.local_name.clone(),
            namespace: name.namespace_uri.clone(),
            position: self.nodes[at].position,
        })
    }

    /// One step of an address, which tells a node from its siblings: an
    /// element by its name, every other node by its node test, and an
    /// attribute by its name alone, an element carrying one of each name.
    fn spell_step(&self, at: usize, indexed: bool) -> Option<String> {
        let data = &self.nodes[at];
        let test = match data.kind {
            NodeKind::Element => self.step(at)?.write(false),
            NodeKind::Attribute => return Some(format!("@{}", self.step(at)?.write(false))),
            NodeKind::Text => "text()".to_owned(),
            NodeKind::Comment => "comment()".to_owned(),
            NodeKind::ProcessingInstruction => "processing-instruction()".to_owned(),
            NodeKind::Root | NodeKind::Namespace => return None,
        };
        Some(match indexed {
            true => format!("{test}[{}]", data.position),
            false => test,
        })
    }

    /// A node of the document as this Bridge spells it from the document
    /// element: for a record element, the record's own selector.
    fn spell_absolute(&self, at: usize) -> Option<String> {
        let Some(parent) = self.nodes[at].parent else {
            return Some("/".to_owned());
        };
        let above = match self.spell_absolute(parent)?.as_str() {
            "/" => String::new(),
            above => above.to_owned(),
        };
        Some(format!(
            "{above}/{}",
            self.spell_step(at, at != self.element)?
        ))
    }

    /// A node of a record as this Bridge spells it from the record, which is
    /// what a finding's oa:refinedBy carries.
    fn spell_relative(&self, at: usize, from: usize) -> Option<String> {
        let mut steps = Vec::new();
        let mut walk = at;
        while walk != from {
            let parent = self.nodes[walk].parent?;
            steps.push(self.spell_step(walk, true)?);
            walk = parent;
        }
        if steps.is_empty() {
            return Some(".".to_owned());
        }
        steps.reverse();
        Some(steps.join("/"))
    }
}

/// Characters said inside the element the walk is in, which the text node
/// already taking them takes where there is one. Character data outside the
/// document element is XML's own whitespace and no node.
fn say(nodes: &mut Vec<Data>, taking: &mut Option<usize>, open: &[usize], characters: String) {
    let Some(&parent) = open.last().filter(|&&node| node != DOCUMENT) else {
        return;
    };
    if let Some(at) = *taking {
        nodes[at].text.push_str(&characters);
        return;
    }
    let at = nodes.len();
    nodes.push(Data::of(NodeKind::Text, Some(parent)).saying(characters));
    nodes[parent].children.push(at);
    *taking = Some(at);
}

#[derive(Clone, Copy)]
struct Handle<'a> {
    tree: &'a Tree,
    at: usize,
}

impl PartialEq for Handle<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.tree, other.tree) && self.at == other.at
    }
}

impl Eq for Handle<'_> {}

impl<'a> Node<'a> for Handle<'a> {
    fn kind(self) -> NodeKind {
        self.tree.nodes[self.at].kind
    }

    fn parent(self) -> Option<Self> {
        self.tree.nodes[self.at].parent.map(|at| self.tree.at(at))
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        let tree = self.tree;
        tree.nodes[self.at].children.iter().map(|&at| tree.at(at))
    }

    fn attributes(self) -> impl Iterator<Item = Self> + 'a {
        let tree = self.tree;
        tree.nodes[self.at].attributes.iter().map(|&at| tree.at(at))
    }

    /// No namespace node. An address is written of `local-name()` and
    /// `namespace-uri()`, which read an element's own expanded name, so the
    /// axis that would list the declarations in scope is one nothing here
    /// walks, and one an address walking it is reported for rather than
    /// answered wrongly.
    fn namespaces(self) -> impl Iterator<Item = Self> + 'a {
        std::iter::empty()
    }

    fn expanded_name(self) -> Option<ExpandedName> {
        self.tree.nodes[self.at].name.clone()
    }

    fn string_value(self) -> String {
        let data = &self.tree.nodes[self.at];
        match data.kind {
            NodeKind::Root | NodeKind::Element => {
                let mut said = String::new();
                said_below(self.tree, self.at, &mut said);
                said
            }
            _ => data.text.clone(),
        }
    }

    fn document_order(self, other: Self) -> Ordering {
        self.at.cmp(&other.at)
    }
}

fn said_below(tree: &Tree, at: usize, into: &mut String) {
    for &child in &tree.nodes[at].children {
        match tree.nodes[child].kind {
            NodeKind::Text => into.push_str(&tree.nodes[child].text),
            NodeKind::Element => said_below(tree, child, into),
            _ => {}
        }
    }
}

impl Document for Tree {
    type N<'a> = Handle<'a>;

    fn root(&self) -> Self::N<'_> {
        self.at(DOCUMENT)
    }
}

#[cfg(test)]
thread_local! {
    /// How many addresses this thread has put through the evaluator, which a
    /// test reads to hold a record's lookup off it.
    static EVALUATED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// How many addresses this thread has put through the parser, which a test
    /// reads to hold an address every record carries off it once a record.
    static PARSED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Every node of the context node an address selects from it; nothing at all
/// where it is no XPath, or is one whose value is not a set of nodes. A node
/// the context does not hold is no node of what the address is about, so an
/// address reaching one reaches nothing.
fn selects(tree: &Tree, parsed: &Parsed, context: usize, written: &str) -> Option<Vec<usize>> {
    #[cfg(test)]
    EVALUATED.with(|count| count.set(count.get() + 1));
    let expression = parsed.expression(written)?;
    let at = EvaluationContext::new(tree.at(context));
    match evaluate(&expression, &at).ok()? {
        Value::NodeSet(nodes) => Some(
            nodes
                .iter()
                .map(|node| node.at)
                .filter(|&at| tree.holds(context, at))
                .collect(),
        ),
        _ => None,
    }
}

/// What an address selected, where it did not select the one node a finding is
/// about.
enum Missed {
    Nodes(usize),
    NoNodeSet,
}

impl std::fmt::Display for Missed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nodes(0) => f.write_str("selects no node"),
            Self::Nodes(count) => write!(f, "selects {count} nodes"),
            Self::NoNodeSet => f.write_str("is no XPath, or selects no set of nodes"),
        }
    }
}

/// The node an address selects, or what it selected instead.
fn followed(
    tree: &Tree,
    parsed: &Parsed,
    context: usize,
    written: &str,
) -> std::result::Result<usize, Missed> {
    let Some(nodes) = selects(tree, parsed, context, written) else {
        return Err(Missed::NoNodeSet);
    };
    match nodes.as_slice() {
        [only] => Ok(*only),
        several => Err(Missed::Nodes(several.len())),
    }
}

/// The node an address selects, where it selects the one.
fn one(tree: &Tree, parsed: &Parsed, context: usize, written: &str) -> Option<usize> {
    followed(tree, parsed, context, written).ok()
}

/// Each address the findings refine their record by, once however many
/// findings carry it.
fn refinements(findings: &[Quad]) -> BTreeSet<String> {
    let refined: HashSet<&BlankNode> = findings
        .iter()
        .filter(|quad| quad.predicate.as_str() == OA_REFINED_BY)
        .filter_map(|quad| match &quad.object {
            Term::BlankNode(node) => Some(node),
            _ => None,
        })
        .collect();
    if refined.is_empty() {
        return BTreeSet::new();
    }
    findings
        .iter()
        .filter(|quad| quad.predicate.as_str() == RDF_VALUE)
        .filter(|quad| {
            matches!(&quad.subject, NamedOrBlankNode::BlankNode(node) if refined.contains(node))
        })
        .filter_map(|quad| match &quad.object {
            Term::Literal(literal) => Some(literal.value().to_owned()),
            _ => None,
        })
        .collect()
}

/// Each record's refinements, followed through the record read as a document
/// of its own: a refinement selects one node of its record, so the record is
/// the whole of what one can reach, and a conversion holds one record's tree
/// and never the document's. The expressions are kept across records, an
/// address written once in an adapter being followed once for every record.
#[derive(Default)]
pub(crate) struct Followed {
    parsed: Parsed,
}

impl Followed {
    /// A report for each address these findings carry that does not select
    /// exactly one node of the record they are about, selecting the document
    /// element as every report does. A record no tree can be built from is one
    /// this Bridge says nothing about.
    pub(crate) fn unresolved(
        &self,
        document: &Record,
        record: &str,
        findings: &[Quad],
    ) -> Result<Vec<Quad>> {
        let addresses = refinements(findings);
        if addresses.is_empty() {
            return Ok(Vec::new());
        }
        let Some(tree) = Tree::of(record) else {
            return Ok(Vec::new());
        };
        let mut reports = Vec::new();
        for written in addresses {
            if one(&tree, &self.parsed, tree.element, &written).is_none() {
                reports.extend(annotation::address(document, &written)?);
            }
        }
        Ok(reports)
    }
}

/// The source document as XPath counts it, for a comparison: a record's own
/// selector is an absolute address, which only the document can answer, so
/// this is the one stage that holds a tree of the whole document. A
/// conformance run holds both finding graphs whole and is bounded by the
/// fixtures it is given, where a conversion is bounded by nothing.
pub(crate) struct Spelled<'a> {
    xml: &'a str,
    tree: OnceCell<Option<Tree>>,
    parsed: Parsed,
}

impl<'a> Spelled<'a> {
    pub(crate) fn of(xml: &'a str) -> Self {
        Self {
            xml,
            tree: OnceCell::new(),
            parsed: Parsed::default(),
        }
    }

    fn tree(&self) -> Option<&Tree> {
        self.tree.get_or_init(|| Tree::of(self.xml)).as_ref()
    }

    /// These findings with every address re-spelled as this Bridge spells the
    /// node it selects, so two findings about one node are one finding however
    /// either of them was spelled.
    pub(crate) fn respelled(&self, findings: Vec<Quad>) -> Respelled {
        let Some(tree) = self.tree() else {
            return Respelled {
                findings,
                missed: BTreeSet::new(),
            };
        };
        let (respell, missed) = respelling(tree, &self.parsed, &findings);
        let findings = findings
            .into_iter()
            .map(|quad| {
                let spelled = match &quad.subject {
                    NamedOrBlankNode::BlankNode(node) if quad.predicate.as_str() == RDF_VALUE => {
                        respell.get(node).cloned()
                    }
                    _ => None,
                };
                match spelled {
                    Some(address) => Quad::new(
                        quad.subject,
                        quad.predicate,
                        Literal::new_simple_literal(address),
                        quad.graph_name,
                    ),
                    None => quad,
                }
            })
            .collect();
        Respelled { findings, missed }
    }
}

/// What each selector node of these findings is to be re-spelled as: a record
/// selector by the node it selects of the document, and each address refining
/// it by the node that one selects of the record. Beside it, each address that
/// selected other than the one node, as the address and what it selected, once
/// however many findings carry it.
fn respelling(
    tree: &Tree,
    parsed: &Parsed,
    quads: &[Quad],
) -> (HashMap<BlankNode, String>, BTreeSet<String>) {
    let mut refining: HashMap<&BlankNode, Vec<&BlankNode>> = HashMap::new();
    let mut refinement: HashSet<&BlankNode> = HashSet::new();
    let mut written: HashMap<&BlankNode, &str> = HashMap::new();
    for quad in quads {
        let NamedOrBlankNode::BlankNode(subject) = &quad.subject else {
            continue;
        };
        match (quad.predicate.as_str(), &quad.object) {
            (OA_REFINED_BY, Term::BlankNode(refined)) => {
                refining.entry(subject).or_default().push(refined);
                refinement.insert(refined);
            }
            (RDF_VALUE, Term::Literal(address)) => {
                written.insert(subject, address.value());
            }
            _ => {}
        }
    }

    let mut respell: HashMap<BlankNode, String> = HashMap::new();
    let mut missed: BTreeSet<String> = BTreeSet::new();
    // One record is one tree, however many of its findings name it.
    let mut records: HashMap<usize, Tree> = HashMap::new();
    for quad in quads {
        let Term::BlankNode(selector) = &quad.object else {
            continue;
        };
        // A record selector is the one a target names and no address refines.
        if quad.predicate.as_str() != OA_HAS_SELECTOR || refinement.contains(selector) {
            continue;
        }
        let Some(address) = written.get(selector) else {
            continue;
        };
        // An absolute address is read from the document node.
        let record = match followed(tree, parsed, DOCUMENT, address) {
            Ok(node) => node,
            Err(other) => {
                // A refinement of a record this names no one of cannot be
                // followed either, and says nothing a reader does not read here.
                missed.insert(format!("{address:?} {other}"));
                continue;
            }
        };
        if let Some(spelled) = tree.spell_absolute(record) {
            respell.insert(selector.clone(), spelled);
        }
        for refined in refining.get(selector).into_iter().flatten() {
            let Some(address) = written.get(refined) else {
                continue;
            };
            // A refinement is read from the record as a document of its own,
            // which is where a conversion reads it.
            let alone = records.entry(record).or_insert_with(|| tree.record(record));
            let node = match followed(alone, parsed, alone.element, address) {
                Ok(node) => node,
                Err(other) => {
                    missed.insert(format!("{address:?} {other}"));
                    continue;
                }
            };
            if let Some(spelled) = alone.spell_relative(node, alone.element) {
                respell.insert((*refined).clone(), spelled);
            }
        }
    }
    (respell, missed)
}

/// Findings with every address re-spelled as this Bridge spells the node it
/// selects, and every address that selected other than that one node.
pub(crate) struct Respelled {
    pub(crate) findings: Vec<Quad>,
    pub(crate) missed: BTreeSet<String>,
}

#[cfg(test)]
mod tests {
    use super::{one, Followed, Parsed, Tree, DOCUMENT, EVALUATED, PARSED};
    use crate::annotation::Record;
    use crate::rdf::{OA_REFINED_BY, RDF_VALUE};
    use oxrdf::{BlankNode, GraphName, Literal, NamedNode, Quad};
    use xpath_eval::NodeKind;

    fn tree(xml: &str) -> Tree {
        Tree::of(xml).expect("a tree")
    }

    /// The one node an address selects of the node it is read from, with no
    /// expression kept from one call to the next.
    fn selects(tree: &Tree, from: usize, written: &str) -> Option<usize> {
        one(tree, &Parsed::default(), from, written)
    }

    /// One finding refining its record by an address, which is what a record's
    /// findings carry for a Bridge to follow.
    fn refining(address: &str) -> Vec<Quad> {
        let selector = BlankNode::default();
        let refined = BlankNode::default();
        let named = |iri: &str| NamedNode::new(iri).expect("an IRI");
        vec![
            Quad::new(
                selector,
                named(OA_REFINED_BY),
                refined.clone(),
                GraphName::DefaultGraph,
            ),
            Quad::new(
                refined,
                named(RDF_VALUE),
                Literal::new_simple_literal(address),
                GraphName::DefaultGraph,
            ),
        ]
    }

    fn kinds(tree: &Tree) -> Vec<NodeKind> {
        tree.nodes[tree.element]
            .children
            .iter()
            .map(|&at| tree.nodes[at].kind)
            .collect()
    }

    fn said(tree: &Tree) -> Vec<&str> {
        tree.nodes[tree.element]
            .children
            .iter()
            .map(|&at| tree.nodes[at].text.as_str())
            .collect()
    }

    #[test]
    fn reads_character_data_either_side_of_a_reference_as_one_text_node() {
        let tree = tree("<c>NG_016761.1:g.12726T&gt;G</c>");
        assert_eq!(kinds(&tree), [NodeKind::Text]);
        assert_eq!(said(&tree), ["NG_016761.1:g.12726T>G"]);
    }

    #[test]
    fn reads_character_data_either_side_of_a_section_as_one_text_node() {
        let tree = tree("<c>Re<![CDATA[ti]]>r&#101;d</c>");
        assert_eq!(kinds(&tree), [NodeKind::Text]);
        assert_eq!(said(&tree), ["Retired"]);
    }

    #[test]
    fn reads_character_data_either_side_of_a_comment_as_two_text_nodes() {
        let tree = tree("<c>Re<![CDATA[ti]]>r<!-- said twice -->&#101;d</c>");
        assert_eq!(
            kinds(&tree),
            [NodeKind::Text, NodeKind::Comment, NodeKind::Text]
        );
        assert_eq!(said(&tree), ["Retir", " said twice ", "ed"]);
    }

    #[test]
    fn reads_character_data_either_side_of_an_instruction_as_two_text_nodes() {
        let tree = tree("<c>Retir<?say it twice?>ed</c>");
        assert_eq!(
            kinds(&tree),
            [
                NodeKind::Text,
                NodeKind::ProcessingInstruction,
                NodeKind::Text
            ]
        );
        assert_eq!([said(&tree)[0], said(&tree)[2]], ["Retir", "ed"]);
    }

    #[test]
    fn counts_a_text_node_made_only_of_space_as_a_node() {
        let tree = tree("<c>\n  <a/>\n</c>");
        assert_eq!(
            kinds(&tree),
            [NodeKind::Text, NodeKind::Element, NodeKind::Text]
        );
    }

    /// One spelling rule: what the lift writes for a record is what following
    /// that address and spelling the node it reaches writes again, and the
    /// steps it wrote that address from reach the same node without it.
    #[test]
    fn spells_a_record_as_the_lift_wrote_its_selector() {
        let document =
            br#"<s:set xmlns:s="urn:example:set"><g><s:item/><s:item/></g><g><item/></g></s:set>"#;
        let units: Vec<crate::lift::Unit> = crate::lift::lift_slice(document, Some("item"))
            .expect("the lift")
            .map(|unit| unit.expect("a unit"))
            .collect();
        assert_eq!(units.len(), 3);
        let tree = tree(std::str::from_utf8(document).expect("utf-8"));
        for unit in &units {
            let selector = unit.selector();
            let at = selects(&tree, DOCUMENT, &selector).unwrap_or_else(|| panic!("{selector}"));
            assert_eq!(tree.spell_absolute(at).as_deref(), Some(selector.as_str()));
        }
    }

    /// Two addresses that reach one node are one address, so every node a
    /// record holds is spelled the one way from the record.
    #[test]
    fn spells_the_node_an_address_reaches_however_it_was_written() {
        let tree = tree(
            r#"<catalog><item>said<!-- aside --><?say again?><note a="b">text</note></item><item/></catalog>"#,
        );
        let record = selects(&tree, DOCUMENT, "/catalog/item[1]").expect("the record");
        for (written, spelled) in [
            ("note[1]/text()", "note[1]/text()[1]"),
            ("note/text()[1]", "note[1]/text()[1]"),
            ("text()", "text()[1]"),
            ("comment()", "comment()[1]"),
            ("processing-instruction()", "processing-instruction()[1]"),
            ("note/@a", "note[1]/@a"),
            (".", "."),
            ("self::item", "."),
        ] {
            let at = selects(&tree, record, written).unwrap_or_else(|| panic!("{written}"));
            assert_eq!(
                tree.spell_relative(at, record).as_deref(),
                Some(spelled),
                "{written}"
            );
        }
    }

    /// A refinement selects one node of its record, so an address reaching
    /// anything else reaches no node the finding could be about: a node
    /// standing outside the record, and the document node a record read on its
    /// own carries, which the record does not hold either.
    #[test]
    fn selects_no_node_for_an_address_that_leaves_the_record() {
        let whole =
            tree(r#"<catalog><item>said<!-- aside --><note a="b"/></item><item/></catalog>"#);
        let record = selects(&whole, DOCUMENT, "/catalog/item[1]").expect("the record");
        // A record taken out of the document and the same record read on its
        // own are one tree, so a comparison and a conversion follow an address
        // through the same nodes.
        let taken = whole.record(record);
        let alone = tree(r#"<item>said<!-- aside --><note a="b"/></item>"#);
        for written in [
            "..",
            "/",
            "../item[2]",
            "following-sibling::item[1]",
            "/catalog/item[1]/note[1]",
            "ancestor::catalog",
        ] {
            assert_eq!(selects(&taken, taken.element, written), None, "{written}");
            assert_eq!(selects(&alone, alone.element, written), None, "{written}");
        }
        for (written, spelled) in [
            (".", "."),
            ("note[1]/@a", "note[1]/@a"),
            ("comment()", "comment()[1]"),
            ("text()", "text()[1]"),
        ] {
            let at = selects(&taken, taken.element, written).unwrap_or_else(|| panic!("{written}"));
            assert_eq!(
                taken.spell_relative(at, taken.element).as_deref(),
                Some(spelled),
                "{written}"
            );
            let at = selects(&alone, alone.element, written).unwrap_or_else(|| panic!("{written}"));
            assert_eq!(
                alone.spell_relative(at, alone.element).as_deref(),
                Some(spelled),
                "{written}"
            );
        }
    }

    /// Each record is a document of its own, whose element is the record, so a
    /// conversion puts each address of a record's findings through the
    /// evaluator and the record's own selector not at all.
    #[test]
    fn reaches_each_record_without_evaluating_its_selector() {
        let document =
            b"<catalog><item><note/></item><item><note/></item><item><note/></item></catalog>";
        let followed = Followed::default();
        let units: Vec<crate::lift::Unit> = crate::lift::lift_slice(document, Some("item"))
            .expect("the lift")
            .map(|unit| unit.expect("a unit"))
            .collect();
        assert_eq!(units.len(), 3);
        let findings = refining("note[1]");
        EVALUATED.with(|count| count.set(0));
        for unit in &units {
            let selector = unit.selector();
            let record = Record {
                source: "urn:example:catalog",
                selector: &selector,
            };
            assert!(followed
                .unresolved(&record, &unit.xml, &findings)
                .expect("the reports")
                .is_empty());
        }
        assert_eq!(EVALUATED.with(std::cell::Cell::get), units.len());
    }

    /// An address is one expression however many records carry it, so a
    /// conversion parses each distinct address once and not once per record,
    /// however many trees it builds.
    #[test]
    fn parses_an_address_once_however_many_records_are_followed_for_it() {
        let document =
            b"<catalog><item><note/></item><item><note/></item><item><note/></item></catalog>";
        let followed = Followed::default();
        let units: Vec<crate::lift::Unit> = crate::lift::lift_slice(document, Some("item"))
            .expect("the lift")
            .map(|unit| unit.expect("a unit"))
            .collect();
        assert_eq!(units.len(), 3);
        let findings = refining("note[1]");
        PARSED.with(|count| count.set(0));
        for unit in &units {
            let selector = unit.selector();
            let record = Record {
                source: "urn:example:catalog",
                selector: &selector,
            };
            assert!(followed
                .unresolved(&record, &unit.xml, &findings)
                .expect("the reports")
                .is_empty());
        }
        assert_eq!(PARSED.with(std::cell::Cell::get), 1);
    }

    #[test]
    fn spells_an_attribute_of_a_record_by_the_element_that_carries_it() {
        let tree = tree(r#"<item><label colour="red"/><label colour="blue"/></item>"#);
        let at = selects(&tree, tree.element, "label[2]/@colour").expect("the attribute");
        assert_eq!(
            tree.spell_relative(at, tree.element).as_deref(),
            Some("label[2]/@colour")
        );
    }
}
