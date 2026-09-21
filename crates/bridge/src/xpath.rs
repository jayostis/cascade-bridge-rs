// Following an address, rather than only writing one. A finding's address is
// an XPath, and what the finding is about is the node that XPath selects, so
// an address selecting no node, or several, is reported.
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
use crate::rdf::{OA_REFINED_BY, RDF_VALUE};
use oxrdf::{BlankNode, NamedOrBlankNode, Quad, Term};
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use xpath_eval::{
    evaluate, parse, Document, EvaluationContext, ExpandedName, Node, NodeKind, Value,
};

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
        let mut open: Vec<usize> = vec![0];
        // The text node still taking characters. Adjacent character data is one
        // text node however many events it arrived in, and a comment or a
        // processing instruction between two runs of it begins no second one.
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
                    let raw = comment.into_inner();
                    let text = std::str::from_utf8(&raw).ok()?.to_owned();
                    let parent = *open.last()?;
                    let at = nodes.len();
                    nodes.push(Data::of(NodeKind::Comment, Some(parent)).saying(text));
                    nodes[parent].children.push(at);
                }
                Event::PI(instruction) => {
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
        let mut elements = nodes[0]
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
}

/// Characters said inside the element the walk is in, which the text node
/// already taking them takes where there is one. Character data outside the
/// document element is XML's own whitespace and no node.
fn say(nodes: &mut Vec<Data>, taking: &mut Option<usize>, open: &[usize], characters: String) {
    let Some(&parent) = open.last().filter(|&&node| node != 0) else {
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
        self.at(0)
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

/// A report for each address these findings carry that does not select exactly
/// one node of the record they are about.
pub(crate) fn unresolved(record: &Record, xml: &str, findings: &[Quad]) -> Result<Vec<Quad>> {
    let addresses = refinements(findings);
    if addresses.is_empty() {
        return Ok(Vec::new());
    }
    let Some(tree) = Tree::of(xml) else {
        return Ok(Vec::new());
    };
    let mut reports = Vec::new();
    for written in addresses {
        if one(&tree, tree.element, &written).is_none() {
            reports.extend(annotation::address(record, &written)?);
        }
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::Tree;
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
    fn reads_character_data_either_side_of_a_section_or_a_comment_as_one_text_node() {
        let tree = tree("<c>Re<![CDATA[ti]]>r<!-- said twice -->&#101;d</c>");
        assert_eq!(kinds(&tree), [NodeKind::Text, NodeKind::Comment]);
        assert_eq!(said(&tree), ["Retired", " said twice "]);
    }

    #[test]
    fn counts_a_text_node_made_only_of_space_as_a_node() {
        let tree = tree("<c>\n  <a/>\n</c>");
        assert_eq!(
            kinds(&tree),
            [NodeKind::Text, NodeKind::Element, NodeKind::Text]
        );
    }
}
