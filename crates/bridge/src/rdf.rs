// The RDF plumbing every stage shares: the terms it names, the one canonical
// form graphs are compared in, and the text a produced graph is handed over as.
use crate::error::Result;
use oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use oxrdf::{Dataset, NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfSerializer};
use std::collections::{BTreeSet, HashSet};

macro_rules! terms {
    ($($name:ident = $namespace:expr, $local:expr;)*) => {
        $(pub const $name: &str = concat!($namespace, $local);)*
    };
}

terms! {
    RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#", "type";
    RDF_FIRST = "http://www.w3.org/1999/02/22-rdf-syntax-ns#", "first";
    RDF_REST = "http://www.w3.org/1999/02/22-rdf-syntax-ns#", "rest";
    RDF_NIL = "http://www.w3.org/1999/02/22-rdf-syntax-ns#", "nil";

    SCHEMA_ABOUT = "http://schema.org/", "about";
    SCHEMA_NAME = "http://schema.org/", "name";
    SCHEMA_IDENTIFIER = "http://schema.org/", "identifier";
    SCHEMA_ENCODING_FORMAT = "http://schema.org/", "encodingFormat";

    BRIDGE_ADAPTER = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "Adapter";
    BRIDGE_TEST_MANIFEST = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "testManifest";
    BRIDGE_ELEMENT_NAME_OF_EACH_RECORD = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "elementNameOfEachRecord";
    BRIDGE_REQUIRES_PROFILE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "requiresProfile";
    BRIDGE_MAPPING = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "mapping";
    BRIDGE_FINDINGS_QUERY = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "findingsQuery";
    BRIDGE_DETECT_QUERY = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "detectQuery";
    BRIDGE_TABLE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "table";
    BRIDGE_ENVELOPE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "envelope";
    BRIDGE_DOC_ROOT_ELEMENT_NAME = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "docRootElementName";
    BRIDGE_INPUT = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "input";
    BRIDGE_EXPECTED_GRAPH = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "expectedGraph";
    BRIDGE_EXPECTED_FINDINGS = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "expectedFindings";
    BRIDGE_STAMP_PREDICATE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "stampPredicate";
    BRIDGE_SPARQL_1_1 = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "sparql-1.1";
    BRIDGE_ISOMORPHIC = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "IsomorphicConversionTest";
    BRIDGE_INPUT_ONLY = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "InputOnlyTest";
    BRIDGE_DATASET = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "DatasetCompletionTest";

    MF_ENTRIES = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#", "entries";
    MF_ACTION = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#", "action";
    MF_RESULT = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#", "result";
    MF_NAME = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#", "name";
}

/// RDFC-1.0 canonical N-Quads, one line per quad, duplicates removed: two
/// graphs are isomorphic exactly when these are equal.
pub fn canonical_lines(quads: impl IntoIterator<Item = Quad>) -> Result<BTreeSet<String>> {
    let mut dataset = Dataset::new();
    for quad in quads {
        dataset.insert(&quad);
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
        hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
    });
    Ok(dataset.iter().map(|quad| quad.to_string()).collect())
}

/// The syntax a produced graph is written in: one to read, one to pipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphFormat {
    Turtle,
    NTriples,
}

impl GraphFormat {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "turtle" => Some(Self::Turtle),
            "ntriples" => Some(Self::NTriples),
            _ => None,
        }
    }

    fn format(self) -> RdfFormat {
        match self {
            Self::Turtle => RdfFormat::Turtle,
            Self::NTriples => RdfFormat::NTriples,
        }
    }
}

/// Every IRI a quad names, for deciding which of the offered prefixes the
/// graph can be spelled with.
fn iris(quad: &Quad) -> [Option<&str>; 3] {
    [
        match &quad.subject {
            NamedOrBlankNode::NamedNode(n) => Some(n.as_str()),
            NamedOrBlankNode::BlankNode(_) => None,
        },
        Some(quad.predicate.as_str()),
        match &quad.object {
            Term::NamedNode(n) => Some(n.as_str()),
            Term::Literal(l) => Some(l.datatype().as_str()),
            Term::BlankNode(_) => None,
        },
    ]
}

/// The namespace of every IRI in the graph, cut at its last "#" or "/".
fn namespaces(quads: &[Quad]) -> HashSet<&str> {
    let mut namespaces = HashSet::new();
    for iri in quads.iter().flat_map(iris).flatten() {
        for cut in [iri.rfind('#'), iri.rfind('/')].into_iter().flatten() {
            namespaces.insert(&iri[..=cut]);
        }
    }
    namespaces
}

/// The graph as text.
///
/// Each triple is written once: two mappings that construct the same triple,
/// or one constructed for every record, describe the graph no more than once.
/// A prefix is declared only where the graph uses it, so a mapping's lift
/// namespaces do not reach output they never appear in.
pub fn serialise(
    quads: &[Quad],
    format: GraphFormat,
    prefixes: &[(String, String)],
) -> Result<String> {
    let namespaces = namespaces(quads);
    let mut serializer = RdfSerializer::from_format(format.format());
    for (prefix, namespace) in prefixes {
        if namespaces.contains(namespace.as_str()) {
            serializer = serializer.with_prefix(prefix, namespace)?;
        }
    }
    let mut serializer = serializer.for_writer(Vec::new());
    let mut written: HashSet<&Quad> = HashSet::new();
    for quad in quads {
        if written.insert(quad) {
            serializer.serialize_quad(quad)?;
        }
    }
    Ok(String::from_utf8(serializer.finish()?)?)
}
