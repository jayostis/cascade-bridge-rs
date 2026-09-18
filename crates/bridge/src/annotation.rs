// A finding about the source document, as the Web Annotation Data Model
// writes it. The adapter's query names what inside the record the finding is
// about; the Bridge names the document and where in it the record stood, and
// moves the query's selector under the record's own as its oa:refinedBy.
//
// Every annotation carries a record selector of its own: selectors on one node
// are alternative ways of selecting the same thing, so annotations sharing one
// would claim to be alternatives of each other.
use crate::error::Result;
use crate::rdf::{
    BRIDGE_THIS_RECORD, OA_ANNOTATION, OA_HAS_BODY, OA_HAS_SELECTOR, OA_HAS_SOURCE, OA_HAS_TARGET,
    OA_REFINED_BY, OA_TEXTUAL_BODY, OA_XPATH_SELECTOR, RDF_TYPE, RDF_VALUE, SH_RESULT_SEVERITY,
    SH_VIOLATION,
};
use oxrdf::{BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use std::collections::HashMap;

/// Where a record stood: the document it was read from, and the XPath that
/// selects it there.
pub struct Record<'a> {
    pub source: &'a str,
    pub selector: &'a str,
}

/// A record's selector is its position: the envelope's document root element,
/// the record element, and the record's number among records of that name.
pub fn selector(root_element: &str, record_element: &str, position: usize) -> String {
    if root_element == record_element {
        format!("/{root_element}[{position}]")
    } else {
        format!("/{root_element}/{record_element}[{position}]")
    }
}

/// How many annotations a graph of findings holds.
pub fn annotations(quads: &[Quad]) -> usize {
    quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == OA_ANNOTATION))
        .count()
}

fn triple(
    subject: impl Into<NamedOrBlankNode>,
    predicate: &str,
    object: impl Into<Term>,
) -> Result<Quad> {
    Ok(Quad::new(
        subject.into(),
        NamedNode::new(predicate)?,
        object,
        GraphName::DefaultGraph,
    ))
}

fn named(iri: &str) -> Result<Term> {
    Ok(Term::from(NamedNode::new(iri)?))
}

/// The record's selector as a node of its own, refining nothing yet.
fn record_selector(record: &Record, into: &mut Vec<Quad>) -> Result<BlankNode> {
    let node = BlankNode::default();
    into.push(triple(node.clone(), RDF_TYPE, named(OA_XPATH_SELECTOR)?)?);
    into.push(triple(
        node.clone(),
        RDF_VALUE,
        Literal::new_simple_literal(record.selector),
    )?);
    Ok(node)
}

/// The finding a Bridge stage made itself, about the record as a whole.
pub fn violation(record: &Record, reason: &str) -> Result<Vec<Quad>> {
    let mut quads = Vec::new();
    let annotation = BlankNode::default();
    let target = BlankNode::default();
    let body = BlankNode::default();
    let selector = record_selector(record, &mut quads)?;
    quads.push(triple(annotation.clone(), RDF_TYPE, named(OA_ANNOTATION)?)?);
    quads.push(triple(annotation.clone(), OA_HAS_TARGET, target.clone())?);
    quads.push(triple(
        target.clone(),
        OA_HAS_SOURCE,
        named(record.source)?,
    )?);
    quads.push(triple(target, OA_HAS_SELECTOR, selector)?);
    quads.push(triple(annotation.clone(), OA_HAS_BODY, body.clone())?);
    quads.push(triple(body.clone(), RDF_TYPE, named(OA_TEXTUAL_BODY)?)?);
    quads.push(triple(
        body,
        RDF_VALUE,
        Literal::new_simple_literal(reason),
    )?);
    quads.push(triple(
        annotation,
        SH_RESULT_SEVERITY,
        named(SH_VIOLATION)?,
    )?);
    Ok(quads)
}

/// What a findings query constructed, made about this record: bridge:thisRecord
/// becomes the document, and each target's selector moves under a record
/// selector of its own.
pub fn about(record: &Record, constructed: Vec<Quad>) -> Result<Vec<Quad>> {
    let source = NamedNode::new(record.source)?;
    let this_record = NamedNode::new(BRIDGE_THIS_RECORD)?;
    let named_record = |term: Term| match term {
        Term::NamedNode(n) if n == this_record => Term::from(source.clone()),
        other => other,
    };
    let mut quads: Vec<Quad> = constructed
        .into_iter()
        .map(|quad| {
            let subject = match named_record(Term::from(quad.subject)) {
                Term::NamedNode(n) => NamedOrBlankNode::from(n),
                Term::BlankNode(b) => NamedOrBlankNode::from(b),
                Term::Literal(_) => unreachable!("a literal is not a subject"),
            };
            Quad::new(
                subject,
                quad.predicate,
                named_record(quad.object),
                GraphName::DefaultGraph,
            )
        })
        .collect();

    let targets: Vec<NamedOrBlankNode> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == OA_HAS_TARGET)
        .filter_map(|q| match &q.object {
            Term::NamedNode(n) => Some(NamedOrBlankNode::from(n.clone())),
            Term::BlankNode(b) => Some(NamedOrBlankNode::from(b.clone())),
            Term::Literal(_) => None,
        })
        .collect();

    let mut refined: HashMap<NamedOrBlankNode, BlankNode> = HashMap::new();
    let mut added = Vec::new();
    for target in targets {
        if refined.contains_key(&target) {
            continue;
        }
        let node = record_selector(record, &mut added)?;
        added.push(triple(target.clone(), OA_HAS_SELECTOR, node.clone())?);
        refined.insert(target, node);
    }

    for quad in &mut quads {
        if quad.predicate.as_str() != OA_HAS_SELECTOR {
            continue;
        }
        let Some(node) = refined.get(&quad.subject) else {
            continue;
        };
        *quad = triple(node.clone(), OA_REFINED_BY, quad.object.clone())?;
    }
    quads.extend(added);
    Ok(quads)
}

/// Blank nodes minted by one query execution, kept apart from every other
/// execution's. Two executions may label a blank node alike, and merging their
/// graphs would then fuse two findings into one and lose the count.
#[derive(Default)]
pub struct Minted(HashMap<String, BlankNode>);

impl Minted {
    pub fn apart(&mut self, quads: impl IntoIterator<Item = Quad>) -> Vec<Quad> {
        quads
            .into_iter()
            .map(|quad| {
                let subject = match quad.subject {
                    NamedOrBlankNode::BlankNode(b) => NamedOrBlankNode::from(self.node(&b)),
                    named => named,
                };
                let object = match quad.object {
                    Term::BlankNode(b) => Term::from(self.node(&b)),
                    other => other,
                };
                Quad::new(subject, quad.predicate, object, GraphName::DefaultGraph)
            })
            .collect()
    }

    fn node(&mut self, was: &BlankNode) -> BlankNode {
        self.0.entry(was.as_str().to_owned()).or_default().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{selector, Minted};
    use oxrdf::{BlankNode, GraphName, Literal, NamedNode, Quad};
    use std::collections::BTreeSet;

    fn labelled(label: &str) -> Quad {
        Quad::new(
            BlankNode::new_unchecked(label),
            NamedNode::new_unchecked("urn:example:p"),
            Literal::new_simple_literal("same"),
            GraphName::DefaultGraph,
        )
    }

    #[test]
    fn writes_a_record_that_is_the_document_element_as_one_step() {
        assert_eq!(selector("Record", "Record", 1), "/Record[1]");
        assert_eq!(selector("Set", "Record", 3), "/Set/Record[3]");
    }

    #[test]
    fn keeps_two_executions_that_labelled_a_blank_node_alike_apart() {
        let fused: BTreeSet<String> = [labelled("b0"), labelled("b0")]
            .into_iter()
            .map(|q| q.to_string())
            .collect();
        assert_eq!(fused.len(), 1, "the collision this guards against");

        let mut one = Minted::default();
        let mut two = Minted::default();
        let apart: BTreeSet<String> = one
            .apart([labelled("b0")])
            .into_iter()
            .chain(two.apart([labelled("b0")]))
            .map(|q| q.to_string())
            .collect();
        assert_eq!(apart.len(), 2);
    }

    #[test]
    fn keeps_one_execution_s_blank_node_one_node() {
        let mut execution = Minted::default();
        let renamed = execution.apart([labelled("b0"), labelled("b0")]);
        assert_eq!(renamed[0].subject, renamed[1].subject);
    }
}
