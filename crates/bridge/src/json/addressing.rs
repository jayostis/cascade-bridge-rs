use super::{parse, pointer, selected, tokens_at, Node};
use crate::annotation::{self, Record};
use crate::error::Result;
use crate::syntax::{refinements, Addresses, Respelled, Respelling};
use crate::terms::{OA_HAS_SELECTOR, OA_REFINED_BY, RDF_VALUE};
use oxrdf::{BlankNode, Literal, NamedOrBlankNode, Quad, Term};
use std::cell::OnceCell;
use std::collections::{BTreeSet, HashMap, HashSet};

pub(crate) struct Followed;

impl Addresses for Followed {
    fn unresolved(&self, document: &Record, record: &str, findings: &[Quad]) -> Result<Vec<Quad>> {
        let addresses = refinements(findings);
        if addresses.is_empty() {
            return Ok(Vec::new());
        }
        let Ok(record) = parse(record) else {
            return Ok(Vec::new());
        };
        let mut reports = Vec::new();
        for written in addresses {
            if selected(&record, &written).len() != 1 {
                reports.extend(annotation::address(document, &written)?);
            }
        }
        Ok(reports)
    }
}

pub(crate) struct Spelled<'a> {
    text: &'a str,
    value: OnceCell<Option<Node>>,
}

impl<'a> Spelled<'a> {
    pub(crate) fn of(text: &'a str) -> Self {
        Self {
            text,
            value: OnceCell::new(),
        }
    }
}

/// The one node a pointer selects, spelled as this Bridge writes it; otherwise why not.
fn one<'a>(root: &'a Node, written: &str) -> std::result::Result<(String, &'a Node), String> {
    match selected(root, written).as_slice() {
        [(positions, node)] => Ok((pointer(&tokens_at(root, positions)), *node)),
        [] => Err(format!("{written:?} selects no node")),
        several => Err(format!("{written:?} selects {} nodes", several.len())),
    }
}

impl Respelling for Spelled<'_> {
    fn respelled(&self, findings: Vec<Quad>) -> Respelled {
        let Some(root) = self.value.get_or_init(|| parse(self.text).ok()).as_ref() else {
            return Respelled {
                findings,
                missed: BTreeSet::new(),
            };
        };
        let mut refining: HashMap<&BlankNode, Vec<&BlankNode>> = HashMap::new();
        let mut refinement: HashSet<&BlankNode> = HashSet::new();
        let mut written: HashMap<&BlankNode, &str> = HashMap::new();
        for quad in &findings {
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
        let mut missed = BTreeSet::new();
        for quad in &findings {
            let Term::BlankNode(selector) = &quad.object else {
                continue;
            };
            if quad.predicate.as_str() != OA_HAS_SELECTOR || refinement.contains(selector) {
                continue;
            }
            let Some(address) = written.get(selector) else {
                continue;
            };
            let record = match one(root, address) {
                Ok((spelled, record)) => {
                    respell.insert(selector.clone(), spelled);
                    record
                }
                Err(why) => {
                    missed.insert(why);
                    continue;
                }
            };
            for refined in refining.get(selector).into_iter().flatten() {
                let Some(address) = written.get(refined) else {
                    continue;
                };
                match one(record, address) {
                    Ok((spelled, _)) => {
                        respell.insert((*refined).clone(), spelled);
                    }
                    Err(why) => {
                        missed.insert(why);
                    }
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::{Followed, Spelled};
    use crate::annotation::{self, Record, SelectorType};
    use crate::syntax::{Addresses, Respelling};
    use crate::terms::RDF_VALUE;
    use oxrdf::Term;

    const SOURCE: &str = "urn:example:document";

    fn finding(record: &str, within: &str) -> Vec<oxrdf::Quad> {
        let record = Record {
            source: SOURCE,
            selector: record,
            selector_type: SelectorType::JsonPointer,
        };
        annotation::violation(&record, "urn:example:body", Some(within)).expect("a finding")
    }

    fn values(findings: &[oxrdf::Quad]) -> Vec<String> {
        let mut values: Vec<String> = findings
            .iter()
            .filter(|quad| quad.predicate.as_str() == RDF_VALUE)
            .filter_map(|quad| match &quad.object {
                Term::Literal(literal) => Some(literal.value().to_owned()),
                _ => None,
            })
            .collect();
        values.sort();
        values
    }

    #[test]
    fn reports_a_refinement_that_selects_other_than_one_node_of_the_record() {
        let document = Record {
            source: SOURCE,
            selector: "",
            selector_type: SelectorType::JsonPointer,
        };
        let record = r#"{"a": 1, "b": 2, "b": 3, "n": null}"#;
        for (within, reported) in [("/a", false), ("/b", true), ("/n", true), ("/z", true)] {
            let reports = Followed
                .unresolved(&document, record, &finding("/0", within))
                .expect("the reports");
            assert_eq!(!reports.is_empty(), reported, "{within}");
        }
    }

    #[test]
    fn spells_alike_two_pointers_that_select_one_node() {
        let document = r#"{"records": [{"given name": "Ann"}]}"#;
        let spelled = Spelled::of(document);
        let respelled = spelled.respelled(finding("/%72ecords/0", "/given name"));
        assert!(respelled.missed.is_empty(), "{:?}", respelled.missed);
        assert_eq!(values(&respelled.findings), ["/given%20name", "/records/0"]);
        let missed = spelled.respelled(finding("/records/1", "/x")).missed;
        assert_eq!(missed.len(), 1, "{missed:?}");
    }
}
