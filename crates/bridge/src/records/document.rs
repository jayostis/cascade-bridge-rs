use super::{ni_name, normalised_base_url, Versioned};
use crate::error::{Error, Result};
use crate::terms::{
    BRIDGE_ARRIVED_AS, BRIDGE_SELECTOR, BRIDGE_SERVER_BASE_URL, BRIDGE_SHA256,
    BRIDGE_THIS_DOCUMENT, BRIDGE_THIS_IMPORT, BRIDGE_THIS_RECORD, PAV_LAST_UPDATE_ON, PAV_VERSION,
    PROV_ACTIVITY, PROV_AGENT, PROV_ENTITY, PROV_HAD_PLAN, PROV_PLAN, PROV_QUALIFIED_ASSOCIATION,
    PROV_SOFTWARE_AGENT, PROV_USED, PROV_WAS_DERIVED_FROM, PROV_WAS_GENERATED_BY, RDFS_LABEL,
    RDF_TYPE,
};
use oxigraph::store::Store;
use oxrdf::vocab::xsd;
use oxrdf::{BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfParser};
use oxsdatatypes::DateTime;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

/// A Turtle file of the facts a caller supplies with a document.
pub(crate) struct Supplied<'a> {
    pub(crate) iri: &'a str,
    pub(crate) turtle: &'a [u8],
}

/// What an adapter release or a Bridge release is named by.
pub(crate) struct Release<'a> {
    pub(crate) label: Option<&'a str>,
    pub(crate) version: Option<&'a str>,
}

pub(crate) struct Document {
    name: NamedNode,
    import: BlankNode,
    supplied: Vec<Quad>,
    table: Vec<Quad>,
}

fn named(iri: &str) -> NamedNode {
    NamedNode::new_unchecked(iri)
}

fn quad(subject: impl Into<NamedOrBlankNode>, predicate: &str, object: impl Into<Term>) -> Quad {
    Quad::new(subject, named(predicate), object, GraphName::DefaultGraph)
}

/// A blank node the same parts name alike in every run, so a graph is written alike byte for byte.
fn stable(parts: &[&str]) -> BlankNode {
    let digest = Sha256::digest(parts.join("|").as_bytes());
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    BlankNode::new_from_unique_id(u128::from_be_bytes(id))
}

fn supplied(facts: Option<Supplied<'_>>, document: &str) -> Result<Vec<Quad>> {
    let Some(facts) = facts else {
        return Ok(Vec::new());
    };
    let mut relabelled: HashMap<BlankNode, BlankNode> = HashMap::new();
    let mut relabel = |node: &BlankNode| {
        let next = relabelled.len().to_string();
        relabelled
            .entry(node.clone())
            .or_insert_with(|| stable(&[document, "supplied", &next]))
            .clone()
    };
    let mut supplied = Vec::new();
    for fact in RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(facts.iri)?
        .for_slice(facts.turtle)
    {
        let mut fact = fact.map_err(|e| Error::facts(format!("{}: {e}", facts.iri)))?;
        if let NamedOrBlankNode::BlankNode(node) = &fact.subject {
            fact.subject = relabel(node).into();
        }
        if let Term::BlankNode(node) = &fact.object {
            fact.object = relabel(node).into();
        }
        if fact.predicate.as_str() == BRIDGE_SERVER_BASE_URL {
            if let Term::Literal(url) = &fact.object {
                fact.object = Literal::new_simple_literal(normalised_base_url(url.value())).into();
            }
        }
        supplied.push(fact);
    }
    Ok(supplied)
}

/// Each arrival's `pav:lastUpdateOn` in the text of the record's dataset a store canonicalised it from.
fn last_updates_as_the_source_wrote(
    graph: &mut [Quad],
    arrivals: &HashSet<NamedOrBlankNode>,
    source: &Store,
) -> Result<()> {
    let date_time = |quad: &Quad| match &quad.object {
        Term::Literal(literal)
            if quad.predicate.as_str() == PAV_LAST_UPDATE_ON
                && arrivals.contains(&quad.subject)
                && literal.datatype() == xsd::DATE_TIME =>
        {
            Some(literal.value().to_owned())
        }
        _ => None,
    };
    if !graph.iter().any(|quad| date_time(quad).is_some()) {
        return Ok(());
    }
    let mut written: HashMap<String, String> = HashMap::new();
    for quad in source.quads_for_pattern(None, None, None, None) {
        if let Term::Literal(literal) = quad?.object {
            if literal.datatype() != xsd::STRING {
                continue;
            }
            if let Ok(value) = DateTime::from_str(literal.value()) {
                written
                    .entry(value.to_string())
                    .or_insert_with(|| literal.value().to_owned());
            }
        }
    }
    for quad in graph.iter_mut() {
        if let Some(text) = date_time(quad).and_then(|canonical| written.get(&canonical)) {
            quad.object = Literal::new_typed_literal(text, xsd::DATE_TIME).into();
        }
    }
    Ok(())
}

impl Document {
    pub(crate) fn new(bytes: &[u8], facts: Option<Supplied<'_>>) -> Result<Self> {
        let name = ni_name(bytes);
        Ok(Self {
            import: stable(&[&name, "import"]),
            supplied: supplied(facts, &name)?,
            name: named(&name),
            table: Vec::new(),
        })
    }

    pub(crate) fn with_table(self, table: Vec<Quad>) -> Self {
        Self { table, ..self }
    }

    /// Loaded into a record's dataset beside its lift and the adapter's tables.
    pub(crate) fn dataset(&self, selector: &str) -> Vec<Quad> {
        let mut dataset = self.supplied.clone();
        dataset.extend(self.table.iter().cloned());
        dataset.push(quad(
            named(BRIDGE_THIS_DOCUMENT),
            BRIDGE_SHA256,
            Literal::new_simple_literal(self.name.as_str()),
        ));
        dataset.push(quad(
            named(BRIDGE_THIS_RECORD),
            BRIDGE_SELECTOR,
            Literal::new_simple_literal(selector),
        ));
        dataset
    }

    /// The record's graph with an arrival for each of its versions.
    pub(crate) fn arrived(
        &self,
        versioned: Versioned,
        selector: &str,
        source: &Store,
    ) -> Result<Vec<Quad>> {
        let arrivals: HashMap<&str, BlankNode> = versioned
            .versions
            .iter()
            .map(|version| {
                (
                    version.name.as_str(),
                    stable(&[self.name.as_str(), "arrival", &version.name, selector]),
                )
            })
            .collect();
        let arrival_of: HashMap<NamedOrBlankNode, BlankNode> = versioned
            .graph
            .iter()
            .filter(|quad| quad.predicate.as_str() == BRIDGE_ARRIVED_AS)
            .filter_map(|quad| match &quad.object {
                Term::NamedNode(version) => Some((
                    quad.subject.clone(),
                    arrivals.get(version.as_str())?.clone(),
                )),
                _ => None,
            })
            .collect();
        let mut graph: Vec<Quad> = versioned
            .graph
            .iter()
            .map(|quad| {
                let mut quad = quad.clone();
                if let Some(arrival) = arrival_of.get(&quad.subject) {
                    quad.subject = arrival.clone().into();
                }
                quad
            })
            .collect();
        let mapping_selected: HashSet<NamedOrBlankNode> = graph
            .iter()
            .filter(|quad| quad.predicate.as_str() == BRIDGE_SELECTOR)
            .map(|quad| quad.subject.clone())
            .collect();
        for version in &versioned.versions {
            let arrival = arrivals[version.name.as_str()].clone();
            if !mapping_selected.contains(&NamedOrBlankNode::from(arrival.clone())) {
                graph.push(quad(
                    arrival.clone(),
                    BRIDGE_SELECTOR,
                    Literal::new_simple_literal(selector),
                ));
            }
            graph.extend([
                quad(arrival.clone(), BRIDGE_ARRIVED_AS, named(&version.name)),
                quad(arrival.clone(), PROV_WAS_DERIVED_FROM, self.name.clone()),
                quad(arrival, PROV_WAS_GENERATED_BY, self.import.clone()),
            ]);
        }
        let arrivals = arrivals.into_values().map(NamedOrBlankNode::from).collect();
        last_updates_as_the_source_wrote(&mut graph, &arrivals, source)?;
        Ok(graph)
    }

    fn in_place(&self, term: Term) -> Term {
        match term {
            Term::NamedNode(node) if node.as_str() == BRIDGE_THIS_DOCUMENT => {
                self.name.clone().into()
            }
            Term::NamedNode(node) if node.as_str() == BRIDGE_THIS_IMPORT => {
                self.import.clone().into()
            }
            other => other,
        }
    }

    /// The document and the import, said once for every record in it.
    pub(crate) fn described(&self, plan: &Release<'_>, bridge: &Release<'_>) -> Vec<Quad> {
        let mut described: Vec<Quad> = self
            .supplied
            .iter()
            .map(|fact| {
                let subject = match self.in_place(Term::from(fact.subject.clone())) {
                    Term::NamedNode(node) => NamedOrBlankNode::from(node),
                    Term::BlankNode(node) => NamedOrBlankNode::from(node),
                    _ => fact.subject.clone(),
                };
                Quad::new(
                    subject,
                    fact.predicate.clone(),
                    self.in_place(fact.object.clone()),
                    GraphName::DefaultGraph,
                )
            })
            .collect();
        let association = stable(&[self.name.as_str(), "association"]);
        described.extend([
            quad(self.name.clone(), RDF_TYPE, named(PROV_ENTITY)),
            quad(self.import.clone(), RDF_TYPE, named(PROV_ACTIVITY)),
            quad(self.import.clone(), PROV_USED, self.name.clone()),
            quad(
                self.import.clone(),
                PROV_QUALIFIED_ASSOCIATION,
                association.clone(),
            ),
        ]);
        for (release, predicate, class) in [
            (plan, PROV_HAD_PLAN, PROV_PLAN),
            (bridge, PROV_AGENT, PROV_SOFTWARE_AGENT),
        ] {
            let node = stable(&[self.name.as_str(), class]);
            described.push(quad(association.clone(), predicate, node.clone()));
            described.push(quad(node.clone(), RDF_TYPE, named(class)));
            for (said, predicate) in [(release.label, RDFS_LABEL), (release.version, PAV_VERSION)] {
                if let Some(said) = said {
                    described.push(quad(
                        node.clone(),
                        predicate,
                        Literal::new_simple_literal(said),
                    ));
                }
            }
        }
        described
    }
}
