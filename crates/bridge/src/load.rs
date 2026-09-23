// Loading an adapter: the crate and its test manifest as one graph, each
// parsed with its own location as base, so every link between them is a walk
// through the graph rather than a match on a file name.
use crate::error::{Error, Result};
use crate::rdf::{
    BRIDGE_ADAPTER, BRIDGE_DETECT_QUERY, BRIDGE_DOCUMENT_SCHEMA, BRIDGE_DOC_ROOT_ELEMENT_NAME,
    BRIDGE_ELEMENT_NAME_OF_EACH_RECORD, BRIDGE_ENVELOPE, BRIDGE_FINDINGS_QUERY, BRIDGE_GAP_SCHEME,
    BRIDGE_MAPPING, BRIDGE_REQUIRES_PROFILE, BRIDGE_SOURCE_ACCOUNTING, BRIDGE_SOURCE_SCHEMA,
    BRIDGE_TABLE, BRIDGE_TEST_MANIFEST, BRIDGE_VOCABULARY_FILE, RDF_FIRST, RDF_NIL, RDF_REST,
    RDF_TYPE, SCHEMA_ABOUT, SCHEMA_IDENTIFIER, SCHEMA_NAME,
};
use crate::resolver::Resolver;
use oxrdf::{Graph, NamedNode, NamedOrBlankNode, Term, Triple};
use oxrdfio::{JsonLdProfile, JsonLdProfileSet, LoadedDocument, RdfFormat, RdfParser};
use std::collections::HashSet;

/// The contexts a crate may name, bundled: loading an adapter fetches nothing.
const RO_CRATE_1_2: &str = "https://w3id.org/ro/crate/1.2/context";
const RO_CRATE_1_2_BODY: &[u8] = include_bytes!("contexts/ro-crate-1.2.json");

type LoaderResult = std::result::Result<LoadedDocument, Box<dyn std::error::Error + Send + Sync>>;

fn context(url: &str) -> LoaderResult {
    if url == RO_CRATE_1_2 {
        return Ok(LoadedDocument {
            url: url.to_owned(),
            content: RO_CRATE_1_2_BODY.to_vec(),
            format: RdfFormat::JsonLd {
                profile: JsonLdProfile::Context.into(),
            },
        });
    }
    Err(format!(
        "the crate names a context this Bridge does not bundle, and nothing is fetched: {url}"
    )
    .into())
}

#[derive(Debug, Clone)]
pub struct Envelope {
    pub iri: String,
    pub name: Option<String>,
    pub doc_root_element_name: Option<String>,
    pub document_schema: Option<String>,
}

pub struct Adapter {
    /// The crate's root entity, the adapter.
    pub root: String,
    pub crate_iri: String,
    pub graph: Graph,
    pub identifier: Option<String>,
    pub element_name_of_each_record: Option<String>,
    pub source_schema: Option<String>,
    /// The vocabulary files the produced graph is read against, as paths in the
    /// checkout the engine command is given rather than files of the crate.
    pub vocabulary_files: Vec<String>,
    pub source_accounting: Option<String>,
    pub gap_scheme: Option<String>,
    pub required_profiles: Vec<String>,
    pub mappings: Vec<String>,
    pub findings_queries: Vec<String>,
    pub detect_query: Option<String>,
    pub tables: Vec<String>,
    pub envelopes: Vec<Envelope>,
    pub manifest: String,
}

pub fn subject(iri: &str) -> Result<NamedOrBlankNode> {
    Ok(NamedOrBlankNode::from(NamedNode::new(iri)?))
}

pub fn objects(graph: &Graph, s: &NamedOrBlankNode, predicate: &str) -> Result<Vec<Term>> {
    let predicate = NamedNode::new(predicate)?;
    let mut terms: Vec<Term> = graph
        .objects_for_subject_predicate(s.as_ref(), predicate.as_ref())
        .map(Term::from)
        .collect();
    // A graph has no insertion order to preserve, so a run's output does not
    // depend on which way a hash happened to fall.
    terms.sort_by_cached_key(ToString::to_string);
    Ok(terms)
}

pub fn values(graph: &Graph, s: &NamedOrBlankNode, predicate: &str) -> Result<Vec<String>> {
    Ok(objects(graph, s, predicate)?
        .iter()
        .map(term_value)
        .collect())
}

pub fn value(graph: &Graph, s: &NamedOrBlankNode, predicate: &str) -> Result<Option<String>> {
    Ok(values(graph, s, predicate)?.into_iter().next())
}

pub fn term_value(term: &Term) -> String {
    match term {
        Term::NamedNode(n) => n.as_str().to_owned(),
        Term::BlankNode(b) => b.as_str().to_owned(),
        Term::Literal(l) => l.value().to_owned(),
        Term::Triple(t) => t.to_string(),
    }
}

/// A term as a subject, for walking on from an object.
pub fn as_subject(term: &Term) -> Option<NamedOrBlankNode> {
    match term {
        Term::NamedNode(n) => Some(n.clone().into()),
        Term::BlankNode(b) => Some(b.clone().into()),
        _ => None,
    }
}

/// The members of an RDF list, in order. A list that comes back to a cell it
/// has already passed is refused, since walking it would never end.
pub fn list(graph: &Graph, head: Option<&Term>) -> Result<Vec<Term>> {
    let mut out = Vec::new();
    let mut passed = HashSet::new();
    let mut node = head.and_then(as_subject);
    while let Some(current) = node {
        if matches!(&current, NamedOrBlankNode::NamedNode(n) if n.as_str() == RDF_NIL) {
            break;
        }
        if !passed.insert(current.clone()) {
            return Err(Error::msg(format!(
                "the RDF list loops back on itself at {current}"
            )));
        }
        if let Some(first) = objects(graph, &current, RDF_FIRST)?.into_iter().next() {
            out.push(first);
        }
        node = objects(graph, &current, RDF_REST)?
            .first()
            .and_then(as_subject);
    }
    Ok(out)
}

fn parse_into(graph: &mut Graph, bytes: &[u8], base: &str, format: RdfFormat) -> Result<()> {
    let parser = RdfParser::from_format(format)
        .with_base_iri(base)?
        // Two files parsed into one graph must not share a blank node label by
        // accident; a walk from an entry to its input would then cross into
        // the other file.
        .rename_blank_nodes();
    for quad in parser.for_slice(bytes).with_document_loader(context) {
        let quad = quad?;
        graph.insert(&Triple::new(quad.subject, quad.predicate, quad.object));
    }
    Ok(())
}

pub fn load_adapter(resolver: &dyn Resolver) -> Result<Adapter> {
    let crate_iri = format!("{}ro-crate-metadata.json", resolver.root());
    let mut graph = Graph::new();
    parse_into(
        &mut graph,
        &resolver.read(&crate_iri)?,
        &crate_iri,
        RdfFormat::JsonLd {
            profile: JsonLdProfileSet::empty(),
        },
    )?;

    let descriptor = subject(&crate_iri)?;
    let root = value(&graph, &descriptor, SCHEMA_ABOUT)?.ok_or_else(|| {
        Error::msg("the crate's metadata descriptor names no root entity (about)")
    })?;
    let root_subject = subject(&root)?;
    if !values(&graph, &root_subject, RDF_TYPE)?
        .iter()
        .any(|t| t == BRIDGE_ADAPTER)
    {
        return Err(Error::msg(format!(
            "the crate's root entity {root} is not a bridge:Adapter"
        )));
    }

    let manifest = value(&graph, &root_subject, BRIDGE_TEST_MANIFEST)?
        .ok_or_else(|| Error::msg("the adapter names no bridge:testManifest"))?;
    parse_into(
        &mut graph,
        &resolver.read(&manifest)?,
        &manifest,
        RdfFormat::Turtle,
    )?;

    let mut envelopes = Vec::new();
    for iri in values(&graph, &root_subject, BRIDGE_ENVELOPE)? {
        let s = subject(&iri)?;
        envelopes.push(Envelope {
            name: value(&graph, &s, SCHEMA_NAME)?,
            doc_root_element_name: value(&graph, &s, BRIDGE_DOC_ROOT_ELEMENT_NAME)?,
            document_schema: value(&graph, &s, BRIDGE_DOCUMENT_SCHEMA)?,
            iri,
        });
    }

    Ok(Adapter {
        identifier: value(&graph, &root_subject, SCHEMA_IDENTIFIER)?,
        element_name_of_each_record: value(
            &graph,
            &root_subject,
            BRIDGE_ELEMENT_NAME_OF_EACH_RECORD,
        )?,
        source_schema: value(&graph, &root_subject, BRIDGE_SOURCE_SCHEMA)?,
        vocabulary_files: values(&graph, &root_subject, BRIDGE_VOCABULARY_FILE)?,
        source_accounting: value(&graph, &root_subject, BRIDGE_SOURCE_ACCOUNTING)?,
        gap_scheme: value(&graph, &root_subject, BRIDGE_GAP_SCHEME)?,
        required_profiles: values(&graph, &root_subject, BRIDGE_REQUIRES_PROFILE)?,
        mappings: values(&graph, &root_subject, BRIDGE_MAPPING)?,
        findings_queries: values(&graph, &root_subject, BRIDGE_FINDINGS_QUERY)?,
        detect_query: value(&graph, &root_subject, BRIDGE_DETECT_QUERY)?,
        tables: values(&graph, &root_subject, BRIDGE_TABLE)?,
        envelopes,
        manifest,
        root,
        crate_iri,
        graph,
    })
}
