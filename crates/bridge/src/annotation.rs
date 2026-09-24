// Every annotation carries a record selector of its own: selectors on one node are
// alternatives of each other.
use crate::error::{Error, Result};
use crate::terms::{
    BRIDGE_ADDRESS_NOT_ONE_NODE, BRIDGE_OCCURRENCES, BRIDGE_PATH_NOT_ACCOUNTED, BRIDGE_THIS_RECORD,
    OA_ANNOTATION, OA_CLASSIFYING, OA_HAS_BODY, OA_HAS_SELECTOR, OA_HAS_SOURCE, OA_HAS_TARGET,
    OA_MOTIVATED_BY, OA_REFINED_BY, OA_XPATH_SELECTOR, RDF_TYPE, RDF_VALUE, SH_FOCUS_NODE, SH_INFO,
    SH_RESULT_PATH, SH_RESULT_SEVERITY, SH_VALUE, SH_VIOLATION,
};
use oxrdf::vocab::xsd;
use oxrdf::{BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use std::collections::{HashMap, HashSet};

pub(crate) struct Record<'a> {
    pub(crate) source: &'a str,
    pub(crate) selector: &'a str,
}

pub(crate) fn annotations(quads: &[Quad]) -> usize {
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

fn described(quads: &[Quad]) -> HashMap<&BlankNode, Vec<&Quad>> {
    let mut description: HashMap<&BlankNode, Vec<&Quad>> = HashMap::new();
    for quad in quads {
        if let NamedOrBlankNode::BlankNode(node) = &quad.subject {
            description.entry(node).or_default().push(quad);
        }
    }
    description
}

/// Each node copied, mapped to the node standing for it.
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

fn finding(
    record: &Record,
    body: &str,
    within: Option<&str>,
    severity: &str,
    value: Option<&str>,
    occurrences: usize,
    carries: &[(&str, &str)],
) -> Result<Vec<Quad>> {
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
    if let Some(value) = value {
        quads.push(triple(
            annotation.clone(),
            SH_VALUE,
            Literal::new_simple_literal(value),
        )?);
    }
    if occurrences > 1 {
        quads.push(triple(
            annotation.clone(),
            BRIDGE_OCCURRENCES,
            Literal::new_typed_literal(occurrences.to_string(), xsd::INTEGER),
        )?);
    }
    for (predicate, iri) in carries {
        quads.push(triple(annotation.clone(), predicate, named(iri)?)?);
    }
    quads.push(triple(annotation, SH_RESULT_SEVERITY, named(severity)?)?);
    Ok(quads)
}

pub(crate) fn violation(record: &Record, body: &str, within: Option<&str>) -> Result<Vec<Quad>> {
    finding(record, body, within, SH_VIOLATION, None, 1, &[])
}

/// Addressed no more finely than the record: which node of the source stands
/// behind a node of the graph is the mapping's to know.
pub(crate) fn drawn(
    record: &Record,
    body: &str,
    path: Option<&str>,
    focus: Option<&str>,
    severity: &str,
) -> Result<Vec<Quad>> {
    let mut carries: Vec<(&str, &str)> = Vec::new();
    if let Some(path) = path {
        carries.push((SH_RESULT_PATH, path));
    }
    if let Some(focus) = focus {
        carries.push((SH_FOCUS_NODE, focus));
    }
    finding(record, body, None, severity, None, 1, &carries)
}

/// Addressed to the document element, the one node found without following an address.
pub(crate) fn address(document: &Record, written: &str) -> Result<Vec<Quad>> {
    finding(
        document,
        BRIDGE_ADDRESS_NOT_ONE_NODE,
        None,
        SH_VIOLATION,
        Some(written),
        1,
        &[],
    )
}

pub(crate) fn unaccounted(
    record: &Record,
    path: &str,
    within: Option<&str>,
    occurrences: usize,
) -> Result<Vec<Quad>> {
    finding(
        record,
        BRIDGE_PATH_NOT_ACCOUNTED,
        within,
        SH_INFO,
        Some(path),
        occurrences,
        &[],
    )
}

pub(crate) fn gap(
    record: &Record,
    gap: &str,
    path: &str,
    within: Option<&str>,
    severity: &str,
    occurrences: usize,
) -> Result<Vec<Quad>> {
    finding(record, gap, within, severity, Some(path), occurrences, &[])
}

pub(crate) fn lookup(
    record: &Record,
    gap: &str,
    value: &str,
    within: Option<&str>,
    severity: &str,
    occurrences: usize,
) -> Result<Vec<Quad>> {
    finding(record, gap, within, severity, Some(value), occurrences, &[])
}

pub(crate) fn about(
    record: &Record,
    query: &str,
    constructed: Vec<Quad>,
    severities: &HashMap<String, String>,
) -> Result<Vec<Quad>> {
    let source = NamedNode::new(record.source)?;
    let this_record = NamedNode::new(BRIDGE_THIS_RECORD)?;
    let named_record = |term: Term| match term {
        Term::NamedNode(n) if n == this_record => Term::from(source.clone()),
        other => other,
    };
    // Before the substitution, so the error names the term the query wrote.
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
    let quads: Vec<Quad> = constructed
        .iter()
        .map(|quad| {
            let subject = match named_record(Term::from(quad.subject.clone())) {
                Term::NamedNode(n) => NamedOrBlankNode::from(n),
                Term::BlankNode(b) => NamedOrBlankNode::from(b),
                Term::Literal(_) => unreachable!("a literal is not a subject"),
                Term::Triple(_) => unreachable!("a triple term is not a subject"),
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
            // The written term: a query naming bridge:thisRecord reads its own name back.
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

    let mut severe: HashSet<&NamedOrBlankNode> = HashSet::new();
    let mut bodies: HashMap<&NamedOrBlankNode, Vec<&str>> = HashMap::new();
    for quad in &quads {
        if quad.predicate.as_str() == SH_RESULT_SEVERITY {
            severe.insert(&quad.subject);
        } else if quad.predicate.as_str() == OA_HAS_BODY {
            if let Term::NamedNode(body) = &quad.object {
                bodies.entry(&quad.subject).or_default().push(body.as_str());
            }
        }
    }
    // In the graph's order, not the set's, so a run's output is the same each run.
    let mut declared = Vec::new();
    for annotation in quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == OA_ANNOTATION))
        .map(|q| &q.subject)
        .filter(|annotation| !severe.contains(annotation))
    {
        let severity = bodies
            .get(annotation)
            .into_iter()
            .flatten()
            .find_map(|body| severities.get(*body))
            .map_or(SH_INFO, String::as_str);
        declared.push(triple(
            annotation.clone(),
            SH_RESULT_SEVERITY,
            named(severity)?,
        )?);
    }
    findings.extend(declared);

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

#[derive(Default)]
pub(crate) struct Minted(HashMap<String, BlankNode>);

impl Minted {
    pub(crate) fn apart(&mut self, quads: impl IntoIterator<Item = Quad>) -> Vec<Quad> {
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
