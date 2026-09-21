// The RDF plumbing every stage shares: the terms it names, the one canonical
// form graphs are compared in, and the text a produced graph is handed over as.
use crate::error::Result;
use oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use oxrdf::{Dataset, NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfSerializer};
use std::collections::{BTreeSet, HashMap, HashSet};

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
    RDF_VALUE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#", "value";

    OA_ANNOTATION = "http://www.w3.org/ns/oa#", "Annotation";
    OA_HAS_TARGET = "http://www.w3.org/ns/oa#", "hasTarget";
    OA_HAS_SOURCE = "http://www.w3.org/ns/oa#", "hasSource";
    OA_HAS_SELECTOR = "http://www.w3.org/ns/oa#", "hasSelector";
    OA_HAS_BODY = "http://www.w3.org/ns/oa#", "hasBody";
    OA_REFINED_BY = "http://www.w3.org/ns/oa#", "refinedBy";
    OA_XPATH_SELECTOR = "http://www.w3.org/ns/oa#", "XPathSelector";
    OA_MOTIVATED_BY = "http://www.w3.org/ns/oa#", "motivatedBy";
    OA_CLASSIFYING = "http://www.w3.org/ns/oa#", "classifying";

    SH_RESULT_SEVERITY = "http://www.w3.org/ns/shacl#", "resultSeverity";
    SH_VALUE = "http://www.w3.org/ns/shacl#", "value";
    SH_VIOLATION = "http://www.w3.org/ns/shacl#", "Violation";
    SH_INFO = "http://www.w3.org/ns/shacl#", "Info";

    SKOS_BROADER = "http://www.w3.org/2004/02/skos/core#", "broader";

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
    BRIDGE_SOURCE_SCHEMA = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "sourceSchema";
    BRIDGE_DOCUMENT_SCHEMA = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "documentSchema";
    BRIDGE_THIS_RECORD = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "thisRecord";
    BRIDGE_SCHEMA_RULE_UNNAMED = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "schemaRuleUnnamed";
    BRIDGE_SOURCE_ACCOUNTING = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "sourceAccounting";
    BRIDGE_PATH_ENTRY = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "PathEntry";
    BRIDGE_SOURCE_PATH = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "sourcePath";
    BRIDGE_GAP_SCHEME = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "gapScheme";
    BRIDGE_VERDICT = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "verdict";
    BRIDGE_NAMES_GAP = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "namesGap";
    BRIDGE_NO_HOME = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "noHome";
    BRIDGE_CARRIED_IN_PART = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "carriedInPart";
    BRIDGE_NO_PREDICATE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "noPredicate";
    BRIDGE_SOURCE_LACKS_REQUIRED = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "sourceLacksRequired";
    BRIDGE_OCCURRENCES = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "occurrences";
    BRIDGE_PATH_NOT_ACCOUNTED = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "pathNotAccounted";
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

/// The blank nodes a graph holds, each joined to every other one a quad of the
/// graph names beside it.
#[derive(Default)]
struct Joined(HashMap<String, String>);

impl Joined {
    fn root(&mut self, node: &str) -> String {
        let mut at = node.to_owned();
        while let Some(up) = self.0.get(&at) {
            if up == &at {
                break;
            }
            at = up.clone();
        }
        self.0.insert(node.to_owned(), at.clone());
        at
    }

    fn join(&mut self, one: &str, two: &str) {
        let (one, two) = (self.root(one), self.root(two));
        if one != two {
            self.0.insert(one, two);
        }
    }
}

/// The blank node a quad belongs to whatever else it names.
fn blank_of(quad: &Quad) -> Option<&str> {
    match (&quad.subject, &quad.object) {
        (NamedOrBlankNode::BlankNode(node), _) => Some(node.as_str()),
        (_, Term::BlankNode(node)) => Some(node.as_str()),
        _ => None,
    }
}

/// Each part of a graph no blank node reaches out of, canonicalised on its own
/// and written as one line. A blank node bijection maps such a part onto such a
/// part, so two graphs are isomorphic exactly when these multisets are equal —
/// and what differs is then one whole part, rather than every line a
/// relabelling moved.
pub fn canonical_parts(quads: impl IntoIterator<Item = Quad>) -> Result<Vec<String>> {
    let quads: HashSet<Quad> = quads.into_iter().collect();
    let mut joined = Joined::default();
    for quad in &quads {
        if let (NamedOrBlankNode::BlankNode(subject), Term::BlankNode(object)) =
            (&quad.subject, &quad.object)
        {
            joined.join(subject.as_str(), object.as_str());
        }
    }

    let mut parts: HashMap<String, Vec<Quad>> = HashMap::new();
    for quad in quads {
        let key = match blank_of(&quad) {
            Some(node) => format!("_:{}", joined.root(node)),
            None => quad.to_string(),
        };
        parts.entry(key).or_default().push(quad);
    }

    let mut written: Vec<String> = Vec::with_capacity(parts.len());
    for part in parts.into_values() {
        written.push(
            canonical_lines(part)?
                .into_iter()
                .collect::<Vec<String>>()
                .join(" "),
        );
    }
    written.sort();
    Ok(written)
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

/// The namespace of every IRI in the graph: what stands before its "#", or
/// before its last "/" where it has none. An IRI has the one namespace, so a
/// name a path of it merely starts with is not one the graph uses.
fn namespaces(quads: &[Quad]) -> HashSet<&str> {
    let mut namespaces = HashSet::new();
    for iri in quads.iter().flat_map(iris).flatten() {
        if let Some(cut) = iri.rfind('#').or_else(|| iri.rfind('/')) {
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
