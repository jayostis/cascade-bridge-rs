// tests/naming/ is a copy of tests/vectors/naming/ in jayostis/cascade-vocabulary and
// tests/versioning/ of fixtures/versioning/ in jayostis/cascade-bridge-spec, the authorities.
use super::{normalised_base_url, versioned};
use crate::error::ErrorKind;
use crate::fixtures::fixture;
use crate::load::{as_subject, list, objects, one, subjects, term_value, turtle, values};
use crate::rdf::canonical_lines;
use crate::terms::{
    BRIDGE_BASE_URL_NORMALISATION_TEST, BRIDGE_BASE_URL_REFUSAL_TEST, BRIDGE_CANONICAL_CONTENT,
    BRIDGE_EXPECTED_GRAPH, BRIDGE_EXPECTED_NAME, BRIDGE_EXPECTED_VERSION, BRIDGE_INPUT,
    BRIDGE_NAME_INPUTS, BRIDGE_NAMING_RULE, BRIDGE_NAMING_TEST, BRIDGE_VERSIONING_TEST,
    CASCADE_SERVER_BASE_URL, MF_ACTION, MF_ENTRIES, MF_NAME, MF_RESULT, QT_QUERY, RDF_TYPE,
};
use oxigraph::model::{Literal, Term};
use oxigraph::sparql::{QueryResults, SparqlEvaluator, Variable};
use oxigraph::store::Store;
use oxrdf::{Graph, NamedOrBlankNode, Quad};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;

/// Stands for the crate's directory, so a manifest's relative IRI names a fixture.
const HERE: &str = "file:///";

fn manifest(directory: &str) -> Graph {
    let iri = format!("{HERE}{directory}/manifest.ttl");
    turtle(&fixture(&format!("{directory}/manifest.ttl")), &iri)
        .expect("the manifest")
        .0
}

fn file(iri: &str) -> Vec<u8> {
    fixture(
        iri.strip_prefix(HERE)
            .expect("a fixture beside the manifest"),
    )
}

fn node(graph: &Graph, entry: &NamedOrBlankNode, predicate: &str) -> NamedOrBlankNode {
    objects(graph, entry, predicate)
        .expect(predicate)
        .first()
        .and_then(as_subject)
        .unwrap_or_else(|| panic!("{entry} has no {predicate}"))
}

fn named(graph: &Graph, entry: &NamedOrBlankNode) -> String {
    one(graph, entry, MF_NAME).expect("a name").expect("a name")
}

fn only(graph: &Graph, subject: &NamedOrBlankNode, predicate: &str) -> String {
    one(graph, subject, predicate)
        .expect(predicate)
        .unwrap_or_else(|| panic!("{subject} has no {predicate}"))
}

/// The rules whose names a vector gives without a query: a list or content rule, the runtime's.
const WITHOUT_QUERY: [&str; 7] = [
    "ClaimNamingRule",
    "SnapshotNamingRule",
    "VersionNamingRule",
    "RevisionNamingRule",
    "FindingNamingRule",
    "TableVersionNamingRule",
    "DocumentNamingRule",
];

/// What `mf:entries` lists of one class; every entry listed is of a class this runner knows.
fn entries(graph: &Graph, class: &str) -> Vec<NamedOrBlankNode> {
    let root = subjects(graph, MF_ENTRIES).expect("the manifest").remove(0);
    let listed = list(
        graph,
        objects(graph, &root, MF_ENTRIES).expect("entries").first(),
    )
    .expect("a list");
    let known = [
        BRIDGE_NAMING_TEST,
        BRIDGE_BASE_URL_NORMALISATION_TEST,
        BRIDGE_BASE_URL_REFUSAL_TEST,
        BRIDGE_VERSIONING_TEST,
    ];
    let mut of_class = Vec::new();
    for entry in listed.iter().filter_map(as_subject) {
        let types = values(graph, &entry, RDF_TYPE).expect("types");
        assert!(
            types.iter().any(|kind| known.contains(&kind.as_str())),
            "{entry} is of no class this runner knows: {types:?}"
        );
        if types.iter().any(|kind| kind == class) {
            of_class.push(entry);
        }
    }
    assert!(!of_class.is_empty(), "no {class}");
    of_class
}

fn names(query: &[u8], inputs: &[String]) -> Vec<Term> {
    let parsed = spargebra::SparqlParser::new()
        .parse_query(std::str::from_utf8(query).expect("UTF-8"))
        .expect("the query");
    let mut evaluator = SparqlEvaluator::new().for_query(parsed);
    let store = Store::new().expect("a store");
    for (position, input) in inputs.iter().enumerate() {
        evaluator = evaluator.substitute_variable(
            Variable::new_unchecked(format!("input{}", position + 1)),
            Literal::new_simple_literal(input),
        );
    }
    let QueryResults::Solutions(solutions) =
        evaluator.on_store(&store).execute().expect("evaluated")
    else {
        panic!("the naming query is not a SELECT");
    };
    solutions
        .map(|solution| {
            solution
                .expect("a solution")
                .get("name")
                .cloned()
                .expect("a ?name")
        })
        .collect()
}

#[test]
fn computes_every_naming_vector_that_names_a_query_with_that_query_run_by_oxigraph() {
    let graph = manifest("tests/naming");
    let listed = entries(&graph, BRIDGE_NAMING_TEST);
    let (mut run, mut skipped) = (0, 0);
    for entry in listed.iter() {
        let action = node(&graph, entry, MF_ACTION);
        let Some(query) = one(&graph, &action, QT_QUERY).expect(QT_QUERY) else {
            let rule = only(&graph, entry, BRIDGE_NAMING_RULE);
            assert!(
                WITHOUT_QUERY
                    .iter()
                    .any(|named| rule.ends_with(&format!("#{named}"))),
                "{} has no query but {rule} has one",
                named(&graph, entry)
            );
            skipped += 1;
            continue;
        };
        let head = objects(&graph, &action, BRIDGE_NAME_INPUTS).expect("inputs");
        let inputs: Vec<String> = list(&graph, head.first())
            .expect("a list")
            .iter()
            .map(term_value)
            .collect();
        let expected = objects(
            &graph,
            &node(&graph, entry, MF_RESULT),
            BRIDGE_EXPECTED_NAME,
        )
        .expect("a name");
        assert_eq!(
            names(&file(&query), &inputs),
            expected,
            "{}",
            named(&graph, entry)
        );
        run += 1;
    }
    assert!(run > 0, "no naming vector names a query");
    assert_eq!(run + skipped, listed.len());
}

#[test]
fn normalises_every_base_url_vector() {
    let graph = manifest("tests/naming");
    for entry in entries(&graph, BRIDGE_BASE_URL_NORMALISATION_TEST) {
        let given = only(
            &graph,
            &node(&graph, &entry, MF_ACTION),
            CASCADE_SERVER_BASE_URL,
        );
        let expected = only(
            &graph,
            &node(&graph, &entry, MF_RESULT),
            CASCADE_SERVER_BASE_URL,
        );
        assert_eq!(
            normalised_base_url(&given).expect(&given),
            expected,
            "{}",
            named(&graph, &entry)
        );
    }
}

#[test]
fn says_which_rule_refused_a_base_url_and_never_the_url() {
    let graph = manifest("tests/naming");
    let refusals = entries(&graph, BRIDGE_BASE_URL_REFUSAL_TEST);
    assert!(!refusals.is_empty(), "the manifest lists no refusal");
    for entry in refusals {
        let name = named(&graph, &entry);
        let given = only(
            &graph,
            &node(&graph, &entry, MF_ACTION),
            CASCADE_SERVER_BASE_URL,
        );
        let refusal = normalised_base_url(&given).expect_err(&name);
        assert_eq!(refusal.kind(), &ErrorKind::Facts, "{name}");
        let said = refusal.to_string();
        assert!(said.starts_with("a server base URL "), "{name}: {said}");
        for part in std::iter::once(given.as_str()).chain(secret_parts(&given)) {
            assert!(!echoes(&said, part), "{name} echoes {part:?}: {said}");
        }
    }
}

/// A URL's userinfo, host, query and fragment, each where present, split as RFC 3986's appendix B does.
fn secret_parts(url: &str) -> Vec<&str> {
    let (rest, fragment) = url
        .split_once('#')
        .map_or((url, None), |(a, b)| (a, Some(b)));
    let (rest, query) = rest
        .split_once('?')
        .map_or((rest, None), |(a, b)| (a, Some(b)));
    let scheme_end = rest
        .find([':', '/'])
        .filter(|&at| rest[at..].starts_with(':') && at > 0);
    let after_scheme = scheme_end.map_or(rest, |at| &rest[at + 1..]);
    let authority = after_scheme
        .strip_prefix("//")
        .map(|tail| &tail[..tail.find('/').unwrap_or(tail.len())]);
    let (userinfo, host_and_port) = match authority {
        Some(authority) => match authority.rfind('@') {
            Some(at) => (Some(&authority[..at]), &authority[at + 1..]),
            None => (None, authority),
        },
        None => (None, ""),
    };
    let host = if host_and_port.starts_with('[') {
        &host_and_port[..host_and_port
            .find(']')
            .map_or(host_and_port.len(), |at| at + 1)]
    } else {
        &host_and_port[..host_and_port.find(':').unwrap_or(host_and_port.len())]
    };
    [userinfo, Some(host), query, fragment]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect()
}

/// Whether the text holds the part as written, not inside a longer run of letters and digits: "userinfo" does not
/// echo "user".
fn echoes(text: &str, part: &str) -> bool {
    !part.is_empty()
        && text.match_indices(part).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + part.len()..].chars().next();
            ![before, after]
                .into_iter()
                .flatten()
                .any(|c| c.is_ascii_alphanumeric())
        })
}

fn ntriples(bytes: &[u8]) -> Vec<Quad> {
    RdfParser::from_format(RdfFormat::NTriples)
        .for_slice(bytes)
        .map(|quad| quad.expect("N-Triples"))
        .collect()
}

fn lines(quads: &[Quad]) -> BTreeSet<String> {
    canonical_lines(quads.iter().cloned()).expect("canonical")
}

#[test]
fn names_every_versioning_vector_of_the_specification() {
    let graph = manifest("tests/versioning");
    for entry in entries(&graph, BRIDGE_VERSIONING_TEST) {
        let name = named(&graph, &entry);
        let input = only(&graph, &node(&graph, &entry, MF_ACTION), BRIDGE_INPUT);
        let result = node(&graph, &entry, MF_RESULT);
        let produced = versioned(ntriples(&file(&input))).expect(&name);

        let expected = ntriples(&file(&only(&graph, &result, BRIDGE_EXPECTED_GRAPH)));
        assert_eq!(lines(&produced.graph), lines(&expected), "{name}");

        let expected_versions: Vec<(String, String)> =
            objects(&graph, &result, BRIDGE_EXPECTED_VERSION)
                .expect("versions")
                .iter()
                .filter_map(as_subject)
                .map(|version| {
                    let content = file(&only(&graph, &version, BRIDGE_CANONICAL_CONTENT));
                    (
                        only(&graph, &version, BRIDGE_EXPECTED_NAME),
                        String::from_utf8(content).expect("UTF-8"),
                    )
                })
                .collect();
        let produced_versions: Vec<(String, String)> = produced
            .versions
            .into_iter()
            .map(|version| (version.name, version.canonical))
            .collect();
        let mut expected_versions = expected_versions;
        let mut produced_versions = produced_versions;
        expected_versions.sort();
        produced_versions.sort();
        assert_eq!(produced_versions, expected_versions, "{name}");
    }
}
