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
    RDF_PROPERTY = "http://www.w3.org/1999/02/22-rdf-syntax-ns#", "Property";

    OWL_DATATYPE_PROPERTY = "http://www.w3.org/2002/07/owl#", "DatatypeProperty";
    OWL_OBJECT_PROPERTY = "http://www.w3.org/2002/07/owl#", "ObjectProperty";
    OWL_ANNOTATION_PROPERTY = "http://www.w3.org/2002/07/owl#", "AnnotationProperty";

    OA_ANNOTATION = "http://www.w3.org/ns/oa#", "Annotation";
    OA_HAS_TARGET = "http://www.w3.org/ns/oa#", "hasTarget";
    OA_HAS_SOURCE = "http://www.w3.org/ns/oa#", "hasSource";
    OA_HAS_SELECTOR = "http://www.w3.org/ns/oa#", "hasSelector";
    OA_HAS_BODY = "http://www.w3.org/ns/oa#", "hasBody";
    OA_REFINED_BY = "http://www.w3.org/ns/oa#", "refinedBy";
    OA_XPATH_SELECTOR = "http://www.w3.org/ns/oa#", "XPathSelector";
    OA_MOTIVATED_BY = "http://www.w3.org/ns/oa#", "motivatedBy";
    OA_CLASSIFYING = "http://www.w3.org/ns/oa#", "classifying";

    SH_SEVERITY = "http://www.w3.org/ns/shacl#", "severity";
    SH_RESULT_SEVERITY = "http://www.w3.org/ns/shacl#", "resultSeverity";
    SH_RESULT_PATH = "http://www.w3.org/ns/shacl#", "resultPath";
    SH_FOCUS_NODE = "http://www.w3.org/ns/shacl#", "focusNode";
    SH_VALUE = "http://www.w3.org/ns/shacl#", "value";
    SH_WARNING = "http://www.w3.org/ns/shacl#", "Warning";
    SH_VIOLATION = "http://www.w3.org/ns/shacl#", "Violation";
    SH_INFO = "http://www.w3.org/ns/shacl#", "Info";

    SKOS_BROADER = "http://www.w3.org/2004/02/skos/core#", "broader";
    SKOS_CONCEPT_SCHEME = "http://www.w3.org/2004/02/skos/core#", "ConceptScheme";
    SKOS_NOTATION = "http://www.w3.org/2004/02/skos/core#", "notation";

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
    BRIDGE_VOCABULARY_FILE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "vocabularyFile";
    BRIDGE_PREDICATE_NOT_DECLARED = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "predicateNotDeclared";
    BRIDGE_DOCUMENT_SCHEMA = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "documentSchema";
    BRIDGE_THIS_RECORD = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "thisRecord";
    BRIDGE_SCHEMA_RULE_UNNAMED = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "schemaRuleUnnamed";
    BRIDGE_SOURCE_ACCOUNTING = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "sourceAccounting";
    BRIDGE_PATH_ENTRY = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "PathEntry";
    BRIDGE_SOURCE_PATH = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "sourcePath";
    BRIDGE_GAP_SCHEME = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "gapScheme";
    BRIDGE_VERDICT = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "verdict";
    BRIDGE_NAMES_GAP = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "namesGap";
    BRIDGE_LOOKUP_IN = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "lookupIn";
    BRIDGE_LOOKUP_NAMES_GAP = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "lookupNamesGap";
    BRIDGE_NO_HOME = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "noHome";
    BRIDGE_CARRIED_IN_PART = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "carriedInPart";
    BRIDGE_NO_PREDICATE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "noPredicate";
    BRIDGE_SOURCE_LACKS_REQUIRED = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "sourceLacksRequired";
    BRIDGE_VALUE_NOT_MAPPED = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "valueNotMapped";
    BRIDGE_OCCURRENCES = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "occurrences";
    BRIDGE_PATH_NOT_ACCOUNTED = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "pathNotAccounted";
    BRIDGE_ADDRESS_NOT_ONE_NODE = "https://ns.cascadeprotocol.org/bridge/v1-draft#", "addressNotOneNode";
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
