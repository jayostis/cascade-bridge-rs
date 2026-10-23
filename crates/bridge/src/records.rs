use crate::error::{Error, Result};
use crate::rdf::canonical_lines;
use crate::terms::{BRIDGE_ARRIVED_AS, OA_HAS_SELECTOR, PROV_SPECIALIZATION_OF, RDF_VALUE};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use oxrdf::{BlankNode, NamedNode, NamedOrBlankNode, Quad, Term};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::net::Ipv6Addr;

mod document;

pub(crate) use document::{Document, Release, Supplied};

pub(crate) const PLACEHOLDER: &str = "urn:cascade:this-version";

pub(crate) fn ni_name(octets: &[u8]) -> String {
    format!(
        "ni:///sha-256;{}",
        URL_SAFE_NO_PAD.encode(Sha256::digest(octets))
    )
}

const DEFAULT_PORTS: [(&str, &str); 2] = [("http", "80"), ("https", "443")];

fn unreserved(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '_' | '~')
}

/// Each percent-encoded unreserved character decoded and every other encoding's hex digits upper
/// case; with `lower`, every other character lower case too (RFC 3986 section 6.2.2.1).
fn percent_normalised(text: &str, lower: bool) -> String {
    let case = |character: char, into: &mut String| {
        if lower {
            into.extend(character.to_lowercase());
        } else {
            into.push(character);
        }
    };
    let mut normal = String::new();
    let mut rest = text;
    while let Some(character) = rest.chars().next() {
        let hex = rest
            .strip_prefix('%')
            .and_then(|after| after.get(..2))
            .filter(|digits| digits.chars().all(|digit| digit.is_ascii_hexdigit()));
        match hex {
            Some(digits) => {
                let decoded = char::from(u8::from_str_radix(digits, 16).expect("hex digits"));
                if unreserved(decoded) {
                    case(decoded, &mut normal);
                } else {
                    normal.push('%');
                    normal.push_str(&digits.to_ascii_uppercase());
                }
                rest = &rest[3..];
            }
            None => {
                case(character, &mut normal);
                rest = &rest[character.len_utf8()..];
            }
        }
    }
    normal
}

/// RFC 3986 section 5.2.4.
fn without_dot_segments(path: &str) -> String {
    let mut path = path;
    let mut output: Vec<&str> = Vec::new();
    while !path.is_empty() {
        if let Some(rest) = path.strip_prefix("../") {
            path = rest;
        } else if let Some(rest) = path.strip_prefix("./") {
            path = rest;
        } else if path.starts_with("/./") {
            path = &path[2..];
        } else if path == "/." {
            path = "/";
        } else if path.starts_with("/../") {
            path = &path[3..];
            output.pop();
        } else if path == "/.." {
            path = "/";
            output.pop();
        } else if path == "." || path == ".." {
            path = "";
        } else {
            let start = usize::from(path.starts_with('/'));
            let end = path[start..].find('/').map_or(path.len(), |at| at + start);
            output.push(&path[..end]);
            path = &path[end..];
        }
    }
    output.concat()
}

/// RFC 3986 sections 6.2.2 and 6.2.3, then every trailing `/` removed. Refused: userinfo, a control
/// character or space, a query or a fragment (empty ones too), no scheme, no host, a host that is
/// not ASCII, a registered name or a bracketed IPv6 address or IPvFuture (lower-cased), and a port that
/// is not ASCII digits or is above 65535 (leading zeros dropped). A refusal says which rule, never the URL: it may carry a credential.
pub(crate) fn normalised_base_url(url: &str) -> Result<String> {
    let refused = |why: &str| Error::facts(format!("a server base URL {why}"));
    if url
        .chars()
        .any(|character| character <= ' ' || character == '\x7f')
    {
        return Err(refused("holds a control character or space"));
    }
    if url.contains(['?', '#']) {
        return Err(refused("carries a query or a fragment"));
    }
    let (scheme, rest) = url
        .split_once(':')
        .filter(|(scheme, _)| {
            scheme.starts_with(|first: char| first.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        })
        .ok_or_else(|| refused("has no scheme"))?;
    let scheme = scheme.to_ascii_lowercase();
    let rest = rest
        .strip_prefix("//")
        .ok_or_else(|| refused("has no host"))?;
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    if authority.contains('@') {
        return Err(refused("carries userinfo"));
    }
    let (host, port) = if authority.starts_with('[') {
        let close = authority
            .find(']')
            .ok_or_else(|| refused("has a bracket that is not closed"))?;
        authority.split_at(close + 1)
    } else {
        authority.split_at(authority.find(':').unwrap_or(authority.len()))
    };
    if !host.is_ascii() {
        return Err(refused("has a host that is not ASCII"));
    }
    if host.starts_with('[') {
        let literal = &host[1..host.len() - 1];
        if !(literal.parse::<Ipv6Addr>().is_ok() || ip_vfuture(literal)) {
            return Err(refused("has a bracketed host that is not an IP literal"));
        }
    } else if !reg_name(host) {
        return Err(refused("has a host that is not a registered name"));
    }
    let port = match port.strip_prefix(':') {
        Some(digits) => digits,
        None if port.is_empty() => "",
        None => return Err(refused("has a port that is not digits")),
    };
    if !port.chars().all(|digit| digit.is_ascii_digit()) {
        return Err(refused("has a port that is not digits"));
    }
    let significant = port.trim_start_matches('0');
    let port = if significant.is_empty() && !port.is_empty() {
        "0"
    } else {
        significant
    };
    if port.len() > 5 || port.parse::<u32>().is_ok_and(|number| number > 65535) {
        return Err(refused("has a port above 65535"));
    }
    if host.is_empty() {
        return Err(refused("has no host"));
    }
    let host = percent_normalised(host, true);
    let default = DEFAULT_PORTS
        .iter()
        .find_map(|(named, port)| (*named == scheme).then_some(*port));
    let authority = if port.is_empty() || Some(port) == default {
        host
    } else {
        format!("{host}:{port}")
    };
    let path = without_dot_segments(&percent_normalised(path, false));
    Ok(format!(
        "{scheme}://{authority}{}",
        path.trim_end_matches('/')
    ))
}

/// RFC 3986 section 3.2.2's reg-name: unreserved and sub-delimiter characters and `%` before two hex digits.
fn reg_name(host: &str) -> bool {
    let mut rest = host;
    while let Some(character) = rest.chars().next() {
        if character == '%' {
            if !rest
                .get(1..3)
                .is_some_and(|digits| digits.chars().all(|d| d.is_ascii_hexdigit()))
            {
                return false;
            }
            rest = &rest[3..];
        } else if unreserved(character) || "!$&'()*+,;=".contains(character) {
            rest = &rest[1..];
        } else {
            return false;
        }
    }
    true
}

/// RFC 3986 section 3.2.2: `v`, hex digits, `.`, then unreserved, sub-delimiter and `:` characters.
fn ip_vfuture(literal: &str) -> bool {
    literal
        .strip_prefix(['v', 'V'])
        .and_then(|rest| rest.split_once('.'))
        .is_some_and(|(version, address)| {
            !version.is_empty()
                && version.chars().all(|c| c.is_ascii_hexdigit())
                && !address.is_empty()
                && address
                    .chars()
                    .all(|c| unreserved(c) || ":!$&'()*+,;=".contains(c))
        })
}

pub(crate) struct Version {
    pub(crate) name: String,
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) canonical: String,
}

pub(crate) struct Versioned {
    pub(crate) graph: Vec<Quad>,
    pub(crate) versions: Vec<Version>,
    pub(crate) dropped: Vec<Dropped>,
}

/// A version not kept, beside the one of its record that was.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Dropped {
    pub(crate) kept: String,
    /// The least selector the mapping wrote on what arrived as it.
    pub(crate) selector: Option<String>,
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

/// Each version the mapping wrote, with the record it is a version of.
fn drafts(mapped: &[Quad]) -> Result<BTreeMap<String, String>> {
    let mut drafts: BTreeMap<String, String> = BTreeMap::new();
    for quad in mapped
        .iter()
        .filter(|quad| quad.predicate.as_str() == PROV_SPECIALIZATION_OF)
    {
        let NamedOrBlankNode::NamedNode(version) = &quad.subject else {
            return Err(Error::adapter(format!(
                "a mapping wrote a version as the blank node {}; a version is an IRI",
                quad.subject
            )));
        };
        if version.as_str().contains('#') {
            return Err(Error::adapter(format!(
                "a mapping wrote the version {version} with a fragment; a fragment names a node nested in a version"
            )));
        }
        let record = quad.object.to_string();
        if drafts
            .insert(version.as_str().to_owned(), record.clone())
            .is_some_and(|held| held != record)
        {
            return Err(Error::adapter(format!(
                "a mapping wrote the version {version} as a specialization of more than one record"
            )));
        }
    }
    Ok(drafts)
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
                return Err(Error::adapter(format!(
                    "the version {version} holds the blank node {node}; a node nested in a version is named by the version's IRI, # and its position"
                )))
            }
            Term::NamedNode(node)
                if version_of(node.as_str()) != version
                    && drafts.contains(version_of(node.as_str())) =>
            {
                return Err(Error::adapter(format!(
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
    let all: BTreeSet<String> = drafts.keys().cloned().collect();
    let mut kept: BTreeMap<&str, (String, &str, String)> = BTreeMap::new();
    for (version, record) in &drafts {
        let canonical = canonical_nquads(content(&mapped, version, &all)?)?;
        let name = ni_name(canonical.as_bytes());
        let least = kept.get(record.as_str()).is_none_or(|(held, draft, _)| {
            (name.as_str(), version.as_str()) < (held.as_str(), *draft)
        });
        if least {
            kept.insert(record, (name, version, canonical));
        }
    }
    let names: BTreeMap<String, String> = kept
        .values()
        .map(|(name, version, _)| ((*version).to_owned(), name.clone()))
        .collect();
    let dropped: BTreeSet<&str> = all
        .iter()
        .filter(|version| !names.contains_key(*version))
        .map(String::as_str)
        .collect();
    let names_dropped = |node: &NamedNode| dropped.contains(version_of(node.as_str()));

    let mut arrivals: BTreeMap<&str, Vec<&NamedOrBlankNode>> = BTreeMap::new();
    let mut selected: HashMap<&NamedOrBlankNode, Vec<NamedOrBlankNode>> = HashMap::new();
    let mut selecting: HashMap<&NamedOrBlankNode, Vec<&str>> = HashMap::new();
    let mut referrers: HashMap<&BlankNode, Vec<&NamedOrBlankNode>> = HashMap::new();
    for quad in &mapped {
        match &quad.object {
            Term::NamedNode(version)
                if quad.predicate.as_str() == BRIDGE_ARRIVED_AS
                    && dropped.contains(version.as_str()) =>
            {
                arrivals
                    .entry(version.as_str())
                    .or_default()
                    .push(&quad.subject);
            }
            Term::Literal(selector) if quad.predicate.as_str() == RDF_VALUE => {
                selecting
                    .entry(&quad.subject)
                    .or_default()
                    .push(selector.value());
            }
            Term::NamedNode(node) if quad.predicate.as_str() == OA_HAS_SELECTOR => {
                selected
                    .entry(&quad.subject)
                    .or_default()
                    .push(node.clone().into());
            }
            Term::BlankNode(node) => {
                if quad.predicate.as_str() == OA_HAS_SELECTOR {
                    selected
                        .entry(&quad.subject)
                        .or_default()
                        .push(node.clone().into());
                }
                referrers.entry(node).or_default().push(&quad.subject);
            }
            _ => {}
        }
    }

    let dropped_versions: Vec<Dropped> = dropped
        .iter()
        .map(|version| Dropped {
            kept: kept[drafts[*version].as_str()].0.clone(),
            selector: arrivals
                .get(version)
                .into_iter()
                .flatten()
                .filter_map(|arrival| selected.get(arrival))
                .flatten()
                .filter_map(|selector| selecting.get(selector))
                .flatten()
                .min()
                .map(|selector| (*selector).to_owned()),
        })
        .collect();

    let mut gone: HashSet<NamedOrBlankNode> = arrivals
        .values()
        .flatten()
        .map(|arrival| (*arrival).clone())
        .collect();
    let is_gone = |subject: &NamedOrBlankNode, gone: &HashSet<NamedOrBlankNode>| {
        gone.contains(subject)
            || matches!(subject, NamedOrBlankNode::NamedNode(node) if names_dropped(node))
    };
    loop {
        let orphaned: Vec<NamedOrBlankNode> = referrers
            .iter()
            .map(|(node, from)| (NamedOrBlankNode::from((*node).clone()), from))
            .filter(|(node, from)| {
                !gone.contains(node)
                    && from
                        .iter()
                        .any(|subject| *subject != node && is_gone(subject, &gone))
                    && from
                        .iter()
                        .all(|subject| *subject == node || is_gone(subject, &gone))
            })
            .map(|(node, _)| node)
            .collect();
        if orphaned.is_empty() {
            break;
        }
        gone.extend(orphaned);
    }

    let graph = mapped
        .iter()
        .filter(|quad| !is_gone(&quad.subject, &gone))
        .filter(|quad| !matches!(&quad.object, Term::NamedNode(node) if names_dropped(node)))
        .map(|quad| renamed_quad(quad, &names))
        .collect();
    let versions = kept
        .into_values()
        .map(|(name, _, canonical)| Version { name, canonical })
        .collect();
    Ok(Versioned {
        graph,
        versions,
        dropped: dropped_versions,
    })
}

#[cfg(test)]
mod base_url;
#[cfg(test)]
mod vectors;
#[cfg(test)]
mod written;
#[cfg(test)]
mod tests {
    use super::{ni_name, versioned};
    use crate::fixtures::{converted, tiny, Variant};
    use crate::terms::BRIDGE_ARRIVED_AS as ARRIVED_AS;
    use oxrdf::Quad;
    use oxrdfio::{RdfFormat, RdfParser};
    use std::collections::BTreeSet;

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
    fn says_of_each_version_it_drops_which_version_was_kept_and_where_the_dropped_one_arrived_from()
    {
        let mapped: Vec<Quad> = RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(&crate::fixtures::fixture(
                "tests/versioning/one-version-a-record.mapped.nt",
            ))
            .map(|quad| quad.expect("N-Triples"))
            .collect();
        let dropped = versioned(mapped).expect("versioned").dropped;
        assert_eq!(dropped.len(), 1);
        assert_eq!(
            dropped[0].kept,
            "ni:///sha-256;NE41csq6GnQ0o4E6uBjw3zQq45yL4XhCXNUwAyC8Mm8"
        );
        assert!(dropped[0]
            .selector
            .as_deref()
            .is_some_and(|selector| selector
                .contains("[local-name()='entry' and namespace-uri()='urn:hl7-org:v3'][1]")));
    }

    #[test]
    fn finds_a_dropped_arrival_s_selector_when_the_selector_node_is_named() {
        let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(
                format!(
                    "<urn:example:v1> {OF} <urn:example:record> .
<urn:example:v1> <urn:example:t> \"1\" .
<urn:example:v2> {OF} <urn:example:record> .
<urn:example:v2> <urn:example:t> \"2\" .
<urn:example:a1> <{ARRIVED_AS}> <urn:example:v1> .
<urn:example:a1> <http://www.w3.org/ns/oa#hasSelector> <urn:example:s1> .
<urn:example:s1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> \"/one\" .
<urn:example:a2> <{ARRIVED_AS}> <urn:example:v2> .
<urn:example:a2> <http://www.w3.org/ns/oa#hasSelector> <urn:example:s2> .
<urn:example:s2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> \"/two\" .
"
                )
                .as_bytes(),
            )
            .map(|quad| quad.expect("N-Triples"))
            .collect();
        let dropped = versioned(quads).expect("versioned").dropped;
        assert_eq!(dropped.len(), 1);
        assert!(
            matches!(dropped[0].selector.as_deref(), Some("/one" | "/two")),
            "{:?}",
            dropped[0].selector
        );
    }

    #[test]
    fn leaves_nothing_of_a_dropped_version_behind_neither_a_reference_to_it_nor_what_hung_off_its_arrival(
    ) {
        let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(
                format!(
                    "<urn:example:v1> {OF} <urn:example:record> .
<urn:example:v2> {OF} <urn:example:record> .
_:kept <{ARRIVED_AS}> <urn:example:v1> .
_:kept <urn:example:by> _:keptBy .
_:keptBy <urn:example:label> \"kept\" .
_:gone <{ARRIVED_AS}> <urn:example:v2> .
_:gone <urn:example:by> _:goneBy .
_:goneBy <urn:example:label> \"dropped\" .
_:about <urn:example:about> <urn:example:v2> .
_:about <urn:example:label> \"about\" .
"
                )
                .as_bytes(),
            )
            .map(|quad| quad.expect("N-Triples"))
            .collect();
        let graph = versioned(quads).expect("versioned").graph;
        let written: Vec<String> = graph.iter().map(ToString::to_string).collect();
        assert!(
            !written.iter().any(|quad| quad.contains("urn:example:v2")),
            "{written:#?}"
        );
        let labels: BTreeSet<String> = graph
            .iter()
            .filter(|quad| quad.predicate.as_str() == "urn:example:label")
            .map(|quad| quad.object.to_string())
            .collect();
        assert_eq!(
            labels,
            BTreeSet::from(["\"kept\"".to_owned(), "\"about\"".to_owned()])
        );
    }

    #[test]
    fn keeps_a_blank_node_only_itself_refers_to_where_no_version_is_dropped() {
        let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::NTriples)
            .for_slice(
                format!(
                    "<urn:example:v> {OF} <urn:example:record> .
_:x <urn:example:next> _:x .
"
                )
                .as_bytes(),
            )
            .map(|quad| quad.expect("N-Triples"))
            .collect();
        let graph = versioned(quads).expect("versioned").graph;
        assert!(
            graph
                .iter()
                .any(|quad| quad.predicate.as_str() == "urn:example:next"),
            "{graph:?}"
        );
    }

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
