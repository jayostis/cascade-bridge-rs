// tests/naming/ and tests/versioning/ are copies of fixtures/naming/ and
// fixtures/versioning/ in jayostis/cascade-bridge-spec, the authority.
use super::{normalised_base_url, versioned};
use crate::fixtures::fixture;
use crate::load::{as_subject, instances, list, objects, one, term_value, turtle};
use crate::terms::{
    BRIDGE_BASE_URL_NORMALISATION_TEST, BRIDGE_CANONICAL_CONTENT, BRIDGE_EXPECTED_GRAPH,
    BRIDGE_EXPECTED_NAME, BRIDGE_EXPECTED_VERSION, BRIDGE_INPUT, BRIDGE_NAME_INPUTS,
    BRIDGE_NAMING_TEST, BRIDGE_SERVER_BASE_URL, BRIDGE_VERSIONING_TEST, MF_ACTION, MF_NAME,
    MF_RESULT, QT_QUERY,
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

fn entries(graph: &Graph, class: &str) -> Vec<NamedOrBlankNode> {
    let entries = instances(graph, class).expect("entries");
    assert!(!entries.is_empty(), "no {class}");
    entries
}

fn names(query: &[u8], input: &str) -> Vec<Term> {
    let parsed = spargebra::SparqlParser::new()
        .parse_query(std::str::from_utf8(query).expect("UTF-8"))
        .expect("the query");
    let QueryResults::Solutions(solutions) = SparqlEvaluator::new()
        .for_query(parsed)
        .on_store(&Store::new().expect("a store"))
        .substitute_variable(
            Variable::new_unchecked("input"),
            Literal::new_simple_literal(input),
        )
        .execute()
        .expect("evaluated")
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
fn computes_every_naming_vector_of_the_specification_with_its_query_run_by_oxigraph() {
    let graph = manifest("tests/naming");
    for entry in entries(&graph, BRIDGE_NAMING_TEST) {
        let action = node(&graph, &entry, MF_ACTION);
        let query = file(&only(&graph, &action, QT_QUERY));
        let head = objects(&graph, &action, BRIDGE_NAME_INPUTS).expect("inputs");
        let inputs: Vec<String> = list(&graph, head.first())
            .expect("a list")
            .iter()
            .map(term_value)
            .collect();
        let expected = only(
            &graph,
            &node(&graph, &entry, MF_RESULT),
            BRIDGE_EXPECTED_NAME,
        );
        let names = names(&query, &inputs.join("|"));
        assert_eq!(
            names.iter().map(term_value).collect::<Vec<_>>(),
            [expected],
            "{}",
            named(&graph, &entry)
        );
    }
}

#[test]
fn normalises_every_base_url_vector_of_the_specification() {
    let graph = manifest("tests/naming");
    for entry in entries(&graph, BRIDGE_BASE_URL_NORMALISATION_TEST) {
        let given = only(
            &graph,
            &node(&graph, &entry, MF_ACTION),
            BRIDGE_SERVER_BASE_URL,
        );
        let expected = only(
            &graph,
            &node(&graph, &entry, MF_RESULT),
            BRIDGE_SERVER_BASE_URL,
        );
        assert_eq!(
            normalised_base_url(&given),
            expected,
            "{}",
            named(&graph, &entry)
        );
    }
}

fn ntriples(bytes: &[u8]) -> Vec<Quad> {
    RdfParser::from_format(RdfFormat::NTriples)
        .for_slice(bytes)
        .map(|quad| quad.expect("N-Triples"))
        .collect()
}

fn lines(quads: &[Quad]) -> BTreeSet<String> {
    quads.iter().map(ToString::to_string).collect()
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
