// Following an address, rather than only writing one. A finding's address is
// an XPath, and what the finding is about is the node that XPath selects, so
// an address selecting no node, or several, is reported, and two findings
// whose addresses reach one node are compared as one however either is
// spelled.
//
// The tree is parsed here rather than read out of the lifted store, because
// XPath counts what the lift drops: a comment, a processing instruction and a
// whitespace-only text node each take a place among their siblings, so a
// positional step read off the store would stand at a different node. The
// characters are decoded exactly as the lift decodes them, so an address is
// followed through the text the lift saw.
use crate::annotation::{self, Record};
use crate::decode::{normalise_attribute_value, normalise_line_endings};
use crate::error::Result;
use crate::lift::Step;
use crate::rdf::{OA_HAS_SELECTOR, OA_REFINED_BY, RDF_VALUE};
use oxrdf::{BlankNode, Literal, NamedOrBlankNode, Quad, Term};
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use std::cell::OnceCell;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use xpath_eval::{
    evaluate, parse, Document, EvaluationContext, ExpandedName, Node, NodeKind, Value,
};

/// The document node, which is where an absolute address is read from and the
/// first node of any tree.
const DOCUMENT: usize = 0;

struct Data {
    kind: NodeKind,
    name: Option<ExpandedName>,
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

/// One document as XPath counts it, held in one vector in document order: a
/// node is an index into it, and one index against another is document order.
struct Tree {
    nodes: Vec<Data>,
    /// The document's own element, which every address of a record or of a
    /// document is written from.
    element: usize,
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
        Some(Self { nodes, element })
    }

    fn at(&self, at: usize) -> Handle<'_> {
        Handle { tree: self, at }
    }

    /// Where a node stands among the siblings of its own expanded name, which
    /// is the place a step carries.
    fn position(&self, at: usize) -> usize {
        let Some(parent) = self.nodes[at].parent else {
            return 1;
        };
        let name = &self.nodes[at].name;
        self.nodes[parent]
            .children
            .iter()
            .take_while(|&&child| child != at)
            .filter(|&&child| self.nodes[child].kind == NodeKind::Element)
            .filter(|&&child| self.nodes[child].name == *name)
            .count()
            + 1
    }

    fn step(&self, at: usize) -> Option<Step> {
        let name = self.nodes[at].name.as_ref()?;
        Some(Step {
            local: name.local_name.clone(),
            namespace: name.namespace_uri.clone(),
            position: self.position(at),
        })
    }

    /// Every element from a node up to and including `stop`, innermost first,
    /// and nothing at all where the node is no element standing below it.
    fn up_to(&self, at: usize, stop: usize) -> Option<Vec<usize>> {
        let mut chain = Vec::new();
        let mut walk = at;
        loop {
            if self.nodes[walk].kind != NodeKind::Element {
                return None;
            }
            chain.push(walk);
            if walk == stop {
                return Some(chain);
            }
            walk = self.nodes[walk].parent?;
        }
    }

    /// A node of the document as this Bridge spells it from the document
    /// element: for a record element, the record's own selector.
    fn spell_absolute(&self, at: usize) -> Option<String> {
        if self.nodes[at].kind == NodeKind::Attribute {
            let carrier = self.spell_absolute(self.nodes[at].parent?)?;
            return Some(format!("{carrier}/@{}", self.step(at)?.write(false)));
        }
        let chain = self.up_to(at, self.element)?;
        let mut written = String::new();
        for (depth, &node) in chain.iter().rev().enumerate() {
            written.push('/');
            written.push_str(&self.step(node)?.write(depth > 0));
        }
        Some(written)
    }

    /// A node of a record as this Bridge spells it from the record, which is
    /// what a finding's oa:refinedBy carries.
    fn spell_relative(&self, at: usize, from: usize) -> Option<String> {
        if self.nodes[at].kind == NodeKind::Attribute {
            let carrier = self.nodes[at].parent?;
            let name = format!("@{}", self.step(at)?.write(false));
            if carrier == from {
                return Some(name);
            }
            return Some(format!("{}/{name}", self.spell_relative(carrier, from)?));
        }
        if at == from {
            return None;
        }
        let chain = self.up_to(at, from)?;
        let mut steps = Vec::with_capacity(chain.len() - 1);
        for &node in chain[..chain.len() - 1].iter().rev() {
            steps.push(self.step(node)?.write(true));
        }
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

/// Every node an address selects from a context node; nothing at all where it
/// is no XPath, or is one whose value is not a set of nodes.
fn selects(tree: &Tree, context: usize, written: &str) -> Option<Vec<usize>> {
    let expression = parse(written).ok()?;
    let context = EvaluationContext::new(tree.at(context));
    match evaluate(&expression, &context).ok()? {
        Value::NodeSet(nodes) => Some(nodes.iter().map(|node| node.at).collect()),
        _ => None,
    }
}

/// The node an address selects, where it selects the one.
fn one(tree: &Tree, context: usize, written: &str) -> Option<usize> {
    match selects(tree, context, written)?.as_slice() {
        [only] => Some(*only),
        _ => None,
    }
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

/// The source document as XPath counts it, followed for every record of it and
/// for the document itself. It is the source rather than the record the lift
/// rebuilt, because the lift leaves out what XPath counts and a record read on
/// its own has no ancestor: an address is followed here through the tree a
/// comparison follows it through, so a conversion and a comparison cannot
/// disagree about the node one address names. One document is one tree, parsed
/// when an address is first to be followed through it.
pub(crate) struct Followed<'a> {
    xml: &'a str,
    tree: OnceCell<Option<Tree>>,
}

impl<'a> Followed<'a> {
    pub(crate) fn of(xml: &'a str) -> Self {
        Self {
            xml,
            tree: OnceCell::new(),
        }
    }

    fn tree(&self) -> Option<&Tree> {
        self.tree.get_or_init(|| Tree::of(self.xml)).as_ref()
    }

    /// A report for each address these findings carry that does not select
    /// exactly one node from the record they are about. A record this Bridge
    /// cannot find in the tree is one it says nothing about, as a document no
    /// tree can be built from is.
    pub(crate) fn unresolved(&self, record: &Record, findings: &[Quad]) -> Result<Vec<Quad>> {
        let addresses = refinements(findings);
        if addresses.is_empty() {
            return Ok(Vec::new());
        }
        let Some(tree) = self.tree() else {
            return Ok(Vec::new());
        };
        let Some(at) = one(tree, DOCUMENT, record.selector) else {
            return Ok(Vec::new());
        };
        let mut reports = Vec::new();
        for written in addresses {
            if one(tree, at, &written).is_none() {
                reports.extend(annotation::address(record, &written)?);
            }
        }
        Ok(reports)
    }
}

/// What each selector node of these findings is to be re-spelled as: a record
/// selector by the node it selects of the document, and each address refining
/// it by the node that one selects of the record. A selector reaching no one
/// node is in nothing here, and stays as it was written.
fn respelling(tree: &Tree, quads: &[Quad]) -> HashMap<BlankNode, String> {
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
    for quad in quads {
        let Term::BlankNode(selector) = &quad.object else {
            continue;
        };
        // A record selector is the one a target names and no address refines.
        if quad.predicate.as_str() != OA_HAS_SELECTOR || refinement.contains(selector) {
            continue;
        }
        let Some(record) = written.get(selector).and_then(|address| {
            // An absolute address is read from the document node.
            one(tree, DOCUMENT, address)
        }) else {
            continue;
        };
        if let Some(spelled) = tree.spell_absolute(record) {
            respell.insert(selector.clone(), spelled);
        }
        for refined in refining.get(selector).into_iter().flatten() {
            let Some(node) = written
                .get(refined)
                .and_then(|address| one(tree, record, address))
            else {
                continue;
            };
            if let Some(spelled) = tree.spell_relative(node, record) {
                respell.insert((*refined).clone(), spelled);
            }
        }
    }
    respell
}

/// These findings with every address re-spelled as this Bridge spells the node
/// it selects, so two findings about one node are one finding however either
/// of them was spelled. An address reaching no one node is left as it was
/// written, and compared by its characters as every address once was.
pub(crate) fn respelled(quads: Vec<Quad>, xml: &str) -> Vec<Quad> {
    let Some(tree) = Tree::of(xml) else {
        return quads;
    };
    let respell = respelling(&tree, &quads);
    if respell.is_empty() {
        return quads;
    }
    quads
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
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{one, Tree, DOCUMENT};
    use xpath_eval::NodeKind;

    fn tree(xml: &str) -> Tree {
        Tree::of(xml).expect("a tree")
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
    /// that address and spelling the node it reaches writes again.
    #[test]
    fn spells_a_record_as_the_lift_wrote_its_selector() {
        let document =
            br#"<s:set xmlns:s="urn:example:set"><g><s:item/><s:item/></g><g><item/></g></s:set>"#;
        let selectors: Vec<String> = crate::lift::lift_slice(document, Some("item"))
            .expect("the lift")
            .map(|unit| unit.expect("a unit").selector())
            .collect();
        assert_eq!(selectors.len(), 3);
        let tree = tree(std::str::from_utf8(document).expect("utf-8"));
        let spelled: Vec<String> = selectors
            .iter()
            .map(|selector| {
                let at = one(&tree, DOCUMENT, selector).unwrap_or_else(|| panic!("{selector}"));
                tree.spell_absolute(at).expect("an address")
            })
            .collect();
        assert_eq!(spelled, selectors);
    }

    #[test]
    fn spells_an_attribute_of_a_record_by_the_element_that_carries_it() {
        let tree = tree(r#"<item><label colour="red"/><label colour="blue"/></item>"#);
        let at = one(&tree, tree.element, "label[2]/@colour").expect("the attribute");
        assert_eq!(
            tree.spell_relative(at, tree.element).as_deref(),
            Some("label[2]/@colour")
        );
    }
}
