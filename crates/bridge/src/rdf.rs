// The RDF plumbing every stage shares: the terms it names, and the one
// canonical form graphs are compared in.
use crate::error::Result;
use oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use oxrdf::{Dataset, Quad};
use std::collections::BTreeSet;

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
    BRIDGE_UNIT = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "unit";
    BRIDGE_PROFILE_REQUIRED = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "profileRequired";
    BRIDGE_MAPPING = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "mapping";
    BRIDGE_FINDINGS_QUERY = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "findingsQuery";
    BRIDGE_DETECT_QUERY = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "detectQuery";
    BRIDGE_TABLE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "table";
    BRIDGE_ENVELOPE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "envelope";
    BRIDGE_ROOT_ELEMENT = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "rootElement";
    BRIDGE_INPUT = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "input";
    BRIDGE_GRAPH = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "graph";
    BRIDGE_FINDINGS = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "findings";
    BRIDGE_IGNORE_PREDICATE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "ignorePredicate";
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
