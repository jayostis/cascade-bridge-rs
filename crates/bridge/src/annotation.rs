// A finding about the source document, as the Web Annotation Data Model
// writes it. The adapter's query names what inside the record the finding is
// about; the Bridge names the document and where in it the record stood, and
// moves the query's selector under the record's own as its oa:refinedBy.
//
// Every annotation carries a record selector of its own: selectors on one node
// are alternative ways of selecting the same thing, so annotations sharing one
// would claim to be alternatives of each other.
use crate::error::{Error, Result};
use crate::rdf::{
    BRIDGE_THIS_RECORD, OA_ANNOTATION, OA_CLASSIFYING, OA_HAS_BODY, OA_HAS_SELECTOR, OA_HAS_SOURCE,
    OA_HAS_TARGET, OA_MOTIVATED_BY, OA_REFINED_BY, OA_XPATH_SELECTOR, RDF_TYPE, RDF_VALUE,
    SH_RESULT_SEVERITY, SH_VIOLATION,
};
use oxrdf::{BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use std::collections::{HashMap, HashSet};

/// Where a record stood: the document it was read from, and the XPath that
/// selects it there.
pub struct Record<'a> {
    pub source: &'a str,
    pub selector: &'a str,
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

/// What each blank node carries, by that node. A walk that instead scanned the
/// graph per node it reached would cost the square of what one query produced
/// for one record, and a record draws findings in the thousands.
fn described(quads: &[Quad]) -> HashMap<&BlankNode, Vec<&Quad>> {
    let mut description: HashMap<&BlankNode, Vec<&Quad>> = HashMap::new();
    for quad in quads {
        if let NamedOrBlankNode::BlankNode(node) = &quad.subject {
            description.entry(node).or_default().push(quad);
        }
    }
    description
}

/// A blank node's description made over as a node of its own: what the query
/// hung on it, and on every blank node reached from it. Each node copied maps
/// to the node standing for it.
fn copy(
    node: &BlankNode,
    from: &HashMap<&BlankNode, Vec<&Quad>>,
    into: &mut Vec<Quad>,
) -> HashMap<BlankNode, BlankNode> {
    let mut made: HashMap<BlankNode, BlankNode> =
        HashMap::from([(node.clone(), BlankNode::default())]);
    let mut pending = vec![node.clone()];
    while let Some(was) = pending.pop() {
        let subject = made[&was].clone();
        for quad in from.get(&was).into_iter().flatten() {
            let object = match &quad.object {
                Term::BlankNode(reached) => {
                    if !made.contains_key(reached) {
                        made.insert(reached.clone(), BlankNode::default());
                        pending.push(reached.clone());
                    }
                    Term::from(made[reached].clone())
                }
                other => other.clone(),
            };
            into.push(Quad::new(
                subject.clone(),
                quad.predicate.clone(),
                object,
                GraphName::DefaultGraph,
            ));
        }
    }
    made
}

/// The finding a Bridge stage made itself: the rule it names as its body, and
/// the element inside the record the rule was broken on, where that is not the
/// record itself.
pub fn violation(record: &Record, body: &str, within: Option<&str>) -> Result<Vec<Quad>> {
    let mut quads = Vec::new();
    let annotation = BlankNode::default();
    let target = BlankNode::default();
    let selector = record_selector(record, &mut quads)?;
    if let Some(within) = within {
        let refinement = BlankNode::default();
        quads.push(triple(
            refinement.clone(),
            RDF_TYPE,
            named(OA_XPATH_SELECTOR)?,
        )?);
        quads.push(triple(
            refinement.clone(),
            RDF_VALUE,
            Literal::new_simple_literal(within),
        )?);
        quads.push(triple(selector.clone(), OA_REFINED_BY, refinement)?);
    }
    quads.push(triple(annotation.clone(), RDF_TYPE, named(OA_ANNOTATION)?)?);
    quads.push(triple(annotation.clone(), OA_HAS_TARGET, target.clone())?);
    quads.push(triple(
        target.clone(),
        OA_HAS_SOURCE,
        named(record.source)?,
    )?);
    quads.push(triple(target, OA_HAS_SELECTOR, selector)?);
    quads.push(triple(annotation.clone(), OA_HAS_BODY, named(body)?)?);
    quads.push(triple(
        annotation.clone(),
        OA_MOTIVATED_BY,
        named(OA_CLASSIFYING)?,
    )?);
    quads.push(triple(
        annotation,
        SH_RESULT_SEVERITY,
        named(SH_VIOLATION)?,
    )?);
    Ok(quads)
}

/// What a findings query constructed, made about this record: bridge:thisRecord
/// becomes the document, and each annotation's target becomes a node of its own
/// carrying a record selector, the query's selector under it.
pub fn about(record: &Record, query: &str, constructed: Vec<Quad>) -> Result<Vec<Quad>> {
    let source = NamedNode::new(record.source)?;
    let this_record = NamedNode::new(BRIDGE_THIS_RECORD)?;
    let named_record = |term: Term| match term {
        Term::NamedNode(n) if n == this_record => Term::from(source.clone()),
        other => other,
    };
    // Read before the substitution below, so this names the term the query's
    // author wrote rather than the document it became.
    if let Some(named) = constructed
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == OA_ANNOTATION))
        .find_map(|q| match &q.subject {
            NamedOrBlankNode::NamedNode(node) => Some(node),
            NamedOrBlankNode::BlankNode(_) => None,
        })
    {
        return Err(Error::msg(format!(
            "findings query {query} constructs {named} as an oa:Annotation; a finding is a blank \
             node written for that one finding: one name is one node for every finding the query \
             produces, and which target, body and severity standing on it belong to which finding \
             is then unrecoverable"
        )));
    }
    // Every pass below is keyed on a node of this graph, and a query may name
    // the record where a node of its own would do, so the substitution comes
    // first: keyed on the constructed graph they would part company over the
    // node an annotation is.
    let quads: Vec<Quad> = constructed
        .iter()
        .map(|quad| {
            let subject = match named_record(Term::from(quad.subject.clone())) {
                Term::NamedNode(n) => NamedOrBlankNode::from(n),
                Term::BlankNode(b) => NamedOrBlankNode::from(b),
                Term::Literal(_) => unreachable!("a literal is not a subject"),
            };
            Quad::new(
                subject,
                quad.predicate.clone(),
                named_record(quad.object.clone()),
                GraphName::DefaultGraph,
            )
        })
        .collect();

    let annotations: HashSet<NamedOrBlankNode> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == OA_ANNOTATION))
        .map(|q| q.subject.clone())
        .collect();
    let sourced: HashSet<&BlankNode> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == OA_HAS_SOURCE)
        .filter_map(|q| match &q.subject {
            NamedOrBlankNode::BlankNode(node) => Some(node),
            NamedOrBlankNode::NamedNode(_) => None,
        })
        .collect();
    let mut targets: Vec<(NamedOrBlankNode, BlankNode)> = Vec::new();
    for (quad, written) in quads.iter().zip(&constructed) {
        if quad.predicate.as_str() != OA_HAS_TARGET || !annotations.contains(&quad.subject) {
            continue;
        }
        let Term::BlankNode(target) = &quad.object else {
            // The written term, not the substituted one: a query naming
            // bridge:thisRecord here must read its own name back.
            return Err(Error::msg(format!(
                "findings query {query} targets {}; a findings query's oa:hasTarget is a blank node",
                written.object
            )));
        };
        if !sourced.contains(target) {
            return Err(Error::msg(format!(
                "findings query {query} targets a node with no oa:hasSource; a finding names the \
                 document it is about, by targeting [ oa:hasSource bridge:thisRecord ]"
            )));
        }
        targets.push((quad.subject.clone(), target.clone()));
    }
    let targeted: HashSet<&NamedOrBlankNode> =
        targets.iter().map(|(annotation, _)| annotation).collect();
    if !annotations.iter().all(|a| targeted.contains(a)) {
        return Err(Error::msg(format!(
            "findings query {query} constructs an annotation with no oa:hasTarget; a finding names \
             the document it is about, by targeting [ oa:hasSource bridge:thisRecord ]"
        )));
    }

    let description = described(&quads);
    let mut findings = Vec::with_capacity(quads.len());
    let mut copied: HashSet<BlankNode> = HashSet::new();
    for (annotation, target) in targets {
        let mut made_over = Vec::new();
        let made = copy(&target, &description, &mut made_over);
        let target = made[&target].clone();
        copied.extend(made.into_keys());
        let selector = record_selector(record, &mut findings)?;
        for quad in made_over {
            if quad.predicate.as_str() == OA_HAS_SELECTOR
                && matches!(&quad.subject, NamedOrBlankNode::BlankNode(b) if *b == target)
            {
                findings.push(triple(selector.clone(), OA_REFINED_BY, quad.object)?);
            } else {
                findings.push(quad);
            }
        }
        findings.push(triple(target.clone(), OA_HAS_SELECTOR, selector)?);
        findings.push(triple(annotation, OA_HAS_TARGET, target)?);
    }

    let retargeted = |quad: &Quad| {
        quad.predicate.as_str() == OA_HAS_TARGET && annotations.contains(&quad.subject)
    };
    // The copy of a node inside a target is a node nothing outside that target
    // names, so dropping the original leaves every other pointer at it on a
    // node with nothing on it.
    let mut kept: HashSet<BlankNode> = HashSet::new();
    let mut pending: Vec<BlankNode> = Vec::new();
    for quad in &quads {
        let inside =
            matches!(&quad.subject, NamedOrBlankNode::BlankNode(node) if copied.contains(node));
        if !inside && !retargeted(quad) {
            keep(quad, &copied, &mut kept, &mut pending);
        }
    }
    while let Some(node) = pending.pop() {
        for quad in description.get(&node).into_iter().flatten() {
            keep(quad, &copied, &mut kept, &mut pending);
        }
    }

    for quad in quads {
        let moved = matches!(&quad.subject, NamedOrBlankNode::BlankNode(node)
            if copied.contains(node) && !kept.contains(node))
            || retargeted(&quad);
        if !moved {
            findings.push(quad);
        }
    }
    Ok(findings)
}

/// A copied node this quad names, noted as one whose own description stays.
fn keep(
    quad: &Quad,
    copied: &HashSet<BlankNode>,
    kept: &mut HashSet<BlankNode>,
    pending: &mut Vec<BlankNode>,
) {
    if let Term::BlankNode(node) = &quad.object {
        if copied.contains(node) && kept.insert(node.clone()) {
            pending.push(node.clone());
        }
    }
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
    use super::Minted;
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
