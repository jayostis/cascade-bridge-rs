// XPath counts what the lift drops: a comment, a processing instruction and a
// whitespace-only text node each take a place among their siblings. So an address is
// followed through a tree parsed here, never through the lifted store.
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

const DOCUMENT: usize = 0;

struct Data {
    kind: NodeKind,
    name: Option<ExpandedName>,
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

/// Nodes in document order, so comparing two indices compares document order.
struct Tree {
    nodes: Vec<Data>,
    element: usize,
}

#[derive(Default)]
struct Parsed(RefCell<HashMap<String, Option<Rc<Expr>>>>);

impl Parsed {
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

fn resolved(namespace: ResolveResult) -> Option<Option<String>> {
    match namespace {
        ResolveResult::Bound(bound) => Some(Some(std::str::from_utf8(bound.as_ref()).ok()?.into())),
        _ => Some(None),
    }
}

impl Tree {
    fn of(xml: &str) -> Option<Self> {
        let mut reader = NsReader::from_str(xml);
        reader.config_mut().expand_empty_elements = true;
        let mut nodes = vec![Data::of(NodeKind::Root, None)];
        let mut open: Vec<usize> = vec![DOCUMENT];
        // The text node still taking characters: adjacent character data is one text node.
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

    /// The record as a document of its own, the tree a conversion builds from a unit.
    fn record(&self, at: usize) -> Self {
        let mut nodes = vec![Data::of(NodeKind::Root, None)];
        let element = nodes.len();
        self.copy(at, DOCUMENT, &mut nodes);
        place(&mut nodes);
        Self { nodes, element }
    }

    /// Attributes before children, as the parser reads them, which keeps document order.
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

    /// An attribute takes no index: an element carries one of each name.
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

/// Character data outside the document element is XML's own whitespace and no node.
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

    /// No namespace node: an address walking that axis is reported rather than answered wrongly.
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
    static EVALUATED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static PARSED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

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

fn one(tree: &Tree, parsed: &Parsed, context: usize, written: &str) -> Option<usize> {
    followed(tree, parsed, context, written).ok()
}

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

#[derive(Default)]
pub(crate) struct Followed {
    parsed: Parsed,
}

impl Followed {
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

/// The one stage that holds a tree of the whole document: a record's selector is absolute.
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

/// Each selector node's new spelling, and each address that selected other than one node.
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
        let record = match followed(tree, parsed, DOCUMENT, address) {
            Ok(node) => node,
            Err(other) => {
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
            // Read from the record alone, which is where a conversion reads it.
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

    fn selects(tree: &Tree, from: usize, written: &str) -> Option<usize> {
        one(tree, &Parsed::default(), from, written)
    }

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

    #[test]
    fn selects_no_node_for_an_address_that_leaves_the_record() {
        let whole =
            tree(r#"<catalog><item>said<!-- aside --><note a="b"/></item><item/></catalog>"#);
        let record = selects(&whole, DOCUMENT, "/catalog/item[1]").expect("the record");
        // Taken out of the document or read alone, a record is one tree.
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
