use crate::error::{Error, Result};
use crate::rdf::canonical_lines;
use crate::terms::PROV_SPECIALIZATION_OF;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use oxrdf::{NamedNode, NamedOrBlankNode, Quad, Term};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod document;

pub(crate) use document::{Document, Release, Supplied};

pub(crate) const PLACEHOLDER: &str = "urn:cascade:this-version";

pub(crate) fn ni_name(octets: &[u8]) -> String {
    format!(
        "ni:///sha-256;{}",
        URL_SAFE_NO_PAD.encode(Sha256::digest(octets))
    )
}

pub(crate) fn normalised_base_url(url: &str) -> String {
    let normal = match url.split_once("://") {
        Some((scheme, rest)) => {
            let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            let (authority, path) = rest.split_at(end);
            let (userinfo, host) = match authority.rsplit_once('@') {
                Some((userinfo, host)) => (format!("{userinfo}@"), host),
                None => (String::new(), authority),
            };
            format!(
                "{}://{userinfo}{}{path}",
                scheme.to_ascii_lowercase(),
                host.to_ascii_lowercase()
            )
        }
        None => url.to_owned(),
    };
    normal.trim_end_matches('/').to_owned()
}

pub(crate) struct Version {
    pub(crate) name: String,
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) canonical: String,
}

pub(crate) struct Versioned {
    pub(crate) graph: Vec<Quad>,
    pub(crate) versions: Vec<Version>,
}

/// The IRI the version's own IRI begins, where `iri` is the version or a node nested in it.
fn version_of(iri: &str) -> &str {
    iri.split_once('#').map_or(iri, |(version, _)| version)
}

fn renamed(node: &NamedNode, names: &BTreeMap<String, String>) -> NamedNode {
    let iri = node.as_str();
    match names.get(version_of(iri)) {
        Some(name) => NamedNode::new_unchecked(format!("{name}{}", &iri[version_of(iri).len()..])),
        None => node.clone(),
    }
}

fn renamed_quad(quad: &Quad, names: &BTreeMap<String, String>) -> Quad {
    Quad::new(
        match &quad.subject {
            NamedOrBlankNode::NamedNode(node) => renamed(node, names).into(),
            blank => blank.clone(),
        },
        renamed(&quad.predicate, names),
        match &quad.object {
            Term::NamedNode(node) => renamed(node, names).into(),
            other => other.clone(),
        },
        quad.graph_name.clone(),
    )
}

fn drafts(mapped: &[Quad]) -> Result<BTreeSet<String>> {
    let mut drafts: BTreeMap<String, &Term> = BTreeMap::new();
    for quad in mapped
        .iter()
        .filter(|quad| quad.predicate.as_str() == PROV_SPECIALIZATION_OF)
    {
        let NamedOrBlankNode::NamedNode(version) = &quad.subject else {
            return Err(Error::msg(format!(
                "a mapping wrote a version as the blank node {}; a version is an IRI",
                quad.subject
            )));
        };
        if version.as_str().contains('#') {
            return Err(Error::msg(format!(
                "a mapping wrote the version {version} with a fragment; a fragment names a node nested in a version"
            )));
        }
        if drafts
            .insert(version.as_str().to_owned(), &quad.object)
            .is_some_and(|record| *record != quad.object)
        {
            return Err(Error::msg(format!(
                "a mapping wrote the version {version} as a specialization of more than one record"
            )));
        }
    }
    Ok(drafts.into_keys().collect())
}

fn content(mapped: &[Quad], version: &str, drafts: &BTreeSet<String>) -> Result<Vec<Quad>> {
    let placeholder = BTreeMap::from([(version.to_owned(), PLACEHOLDER.to_owned())]);
    let mut content = Vec::new();
    for quad in mapped {
        let NamedOrBlankNode::NamedNode(subject) = &quad.subject else {
            continue;
        };
        if version_of(subject.as_str()) != version {
            continue;
        }
        match &quad.object {
            Term::BlankNode(node) => {
                return Err(Error::msg(format!(
                    "the version {version} holds the blank node {node}; a node nested in a version is named by the version's IRI, # and its position"
                )))
            }
            Term::NamedNode(node)
                if version_of(node.as_str()) != version
                    && drafts.contains(version_of(node.as_str())) =>
            {
                return Err(Error::msg(format!(
                    "the version {version} names the version {node}; a version names no other version"
                )))
            }
            _ => {}
        }
        content.push(renamed_quad(quad, &placeholder));
    }
    Ok(content)
}

fn canonical_nquads(content: Vec<Quad>) -> Result<String> {
    Ok(canonical_lines(content)?
        .into_iter()
        .map(|line| format!("{line} .\n"))
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect())
}

/// Hashes what the mapping constructed, before any store holds it.
pub(crate) fn versioned(mapped: Vec<Quad>) -> Result<Versioned> {
    let drafts = drafts(&mapped)?;
    let mut names = BTreeMap::new();
    let mut versions = Vec::new();
    for version in &drafts {
        let canonical = canonical_nquads(content(&mapped, version, &drafts)?)?;
        let name = ni_name(canonical.as_bytes());
        names.insert(version.clone(), name.clone());
        versions.push(Version { name, canonical });
    }
    let graph = mapped
        .iter()
        .map(|quad| renamed_quad(quad, &names))
        .collect();
    Ok(Versioned { graph, versions })
}

#[cfg(test)]
mod vectors;
#[cfg(test)]
mod written;
#[cfg(test)]
mod tests {
    use super::{ni_name, versioned};
    use crate::fixtures::{converted, tiny, Variant};
    use oxrdf::Quad;
    use oxrdfio::{RdfFormat, RdfParser};

    fn refusal(ntriples: &str) -> String {
        let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(ntriples.as_bytes())
            .map(|quad| quad.expect("N-Triples"))
            .collect();
        match versioned(quads) {
            Ok(_) => panic!("named the versions of {ntriples}"),
            Err(refusal) => refusal.to_string(),
        }
    }

    const OF: &str = "<http://www.w3.org/ns/prov#specializationOf>";

    #[test]
    fn refuses_a_version_whose_iri_carries_a_fragment() {
        let said = refusal(&format!("<urn:example:v#one> {OF} <urn:example:record> ."));
        assert!(said.contains("with a fragment"), "{said}");
    }

    #[test]
    fn refuses_a_version_of_two_records() {
        let said = refusal(&format!(
            "<urn:example:v> {OF} <urn:example:one> .\n<urn:example:v> {OF} <urn:example:two> ."
        ));
        assert!(said.contains("more than one record"), "{said}");
    }

    #[test]
    fn names_a_version_every_mapping_writes_as_a_specialization_of_the_same_record() {
        let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(
                format!("<urn:example:v> {OF} <urn:example:record> .\n<urn:example:v> {OF} <urn:example:record> .\n")
                    .as_bytes(),
            )
            .map(|quad| quad.expect("N-Triples"))
            .collect();
        let versioned = versioned(quads).expect("versioned");
        assert_eq!(versioned.versions.len(), 1);
    }

    #[test]
    fn refuses_a_version_holding_a_blank_node() {
        let said = refusal(&format!(
            "<urn:example:v> {OF} <urn:example:record> .\n<urn:example:v> <urn:example:reaction> _:r ."
        ));
        assert!(said.contains("blank node"), "{said}");
    }

    #[test]
    fn refuses_a_version_naming_another_version() {
        let said = refusal(&format!(
            "<urn:example:v> {OF} <urn:example:one> .\n<urn:example:w> {OF} <urn:example:two> .\n<urn:example:v> <urn:example:see> <urn:example:w#part> ."
        ));
        assert!(said.contains("names no other version"), "{said}");
    }

    #[test]
    fn names_each_version_a_mapping_constructs_from_its_literals_as_written() {
        let variant = Variant::of(tiny())
            .replacing(
                "mapping/item.rq",
                "CONSTRUCT { ?s a ex:Item ; ex:title ?title . }",
                "CONSTRUCT { ?s a ex:Item ; ex:title ?title . ?v <http://www.w3.org/ns/prov#specializationOf> ?s ; ex:dose \"2.00\"^^<http://www.w3.org/2001/XMLSchema#decimal> . }",
            )
            .replacing(
                "mapping/item.rq",
                "BIND(IRI(CONCAT(\"urn:example:item:\", ?id)) AS ?s)",
                "BIND(IRI(CONCAT(\"urn:example:item:\", ?id)) AS ?s) BIND(IRI(CONCAT(\"urn:example:draft:\", ?id)) AS ?v)",
            );
        let quads = converted(&variant, "two.xml").quads;
        let content = concat!(
            "<urn:cascade:this-version> <http://www.w3.org/ns/prov#specializationOf> <urn:example:item:1> .\n",
            "<urn:cascade:this-version> <urn:example:catalog#dose> \"2.00\"^^<http://www.w3.org/2001/XMLSchema#decimal> .\n",
        );
        let name = ni_name(content.as_bytes());
        let dose: Vec<String> = quads
            .iter()
            .filter(|quad| quad.predicate.as_str() == "urn:example:catalog#dose")
            .map(|quad| format!("{} {}", quad.subject, quad.object))
            .collect();
        assert!(
            dose.contains(&format!(
                "<{name}> \"2.00\"^^<http://www.w3.org/2001/XMLSchema#decimal>"
            )),
            "{dose:?}"
        );
        assert!(!quads
            .iter()
            .any(|quad| quad.subject.to_string().contains("urn:example:draft:")));
    }

    #[test]
    fn leaves_a_graph_without_a_version_as_the_mapping_wrote_it() {
        let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(
                b"_:r <urn:example:code> \"2.00\"^^<http://www.w3.org/2001/XMLSchema#decimal> .\n",
            )
            .map(|quad| quad.expect("N-Triples"))
            .collect();
        let versioned = versioned(quads.clone()).expect("versioned");
        assert_eq!(versioned.graph, quads);
        assert!(versioned.versions.is_empty());
    }
}
