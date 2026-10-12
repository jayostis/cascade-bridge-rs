use super::{agree, compile, Dataset, Executor, Fail, Tree};
use crate::error::Result;
use crate::fixtures::{conversion_under, tiny};
use crate::query::{Form, Profile, Query};
use crate::rdf::canonical_parts;
use crate::resolver::Resolver;
use oxigraph::model::{GraphName, Quad};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use oxrdfio::{RdfFormat, RdfParser};

const PREFIXES: &str =
    "PREFIX : <urn:t#> PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> \
                        PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> ";

const DATA: &str = r#"
@prefix : <urn:t#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
:root a :Root ; rdf:_1 :a ; rdf:_2 :b ; rdf:_3 :c .
:a a :Item ; :id "1" ; :name "alpha" ; :size 3 ; rdf:_1 :a1 ; rdf:_2 :a2 .
:b a :Item ; :id "2" ; :name "beta" ; :size 5 .
:c a :Item ; :id "x" ; :size "many" .
:a1 a :Note ; rdf:_1 "first" ; rdf:_2 "second" .
:a2 a :Note ; rdf:_1 "first" .
:one :key "1" ; :label "one" .
:two :key "2" ; :label "two" .
:v1 :v "Hello, World" ; :n "17" ; :w "  padded  " .
:v2 :v "naïve café"@fr .
:p :next :q ; :other :r .
:q :next :p ; a :Node .
"#;

fn graph() -> Store {
    let store = Store::new().expect("a store");
    for quad in RdfParser::from_format(RdfFormat::Turtle).for_slice(DATA) {
        let quad = quad.expect("data");
        let quad = Quad::new(
            quad.subject,
            quad.predicate,
            quad.object,
            GraphName::DefaultGraph,
        );
        store.insert(&quad).expect("inserted");
    }
    store
}

fn parse(query: &str) -> spargebra::Query {
    spargebra::SparqlParser::new()
        .parse_query(&format!("{PREFIXES}{query}"))
        .expect("a query")
}

fn evaluated(query: &str, store: &Store) -> Vec<String> {
    let QueryResults::Graph(triples) = SparqlEvaluator::new()
        .for_query(parse(query))
        .on_store(store)
        .execute()
        .expect("evaluated")
    else {
        panic!("a CONSTRUCT");
    };
    let quads = triples.map(|t| {
        let t = t.expect("a triple");
        Quad::new(t.subject, t.predicate, t.object, GraphName::DefaultGraph)
    });
    canonical_parts(quads).expect("canonical")
}

fn compiled(query: &str, store: &Store) -> std::result::Result<Vec<String>, Fail> {
    let plan = compile(&parse(query)).expect("a query the compiler reads");
    let quads = plan.construct(&Tree::of(store).expect("a tree"))?;
    Ok(canonical_parts(quads).expect("canonical"))
}

/// Whether the query answered anything.
fn answers_alike(query: &str) -> bool {
    let store = graph();
    let theirs = evaluated(query, &store);
    assert_eq!(compiled(query, &store), Ok(theirs.clone()), "{query}");
    !theirs.is_empty()
}

#[test]
fn answers_each_shape_of_the_mapping_queries_as_the_general_evaluator_does() {
    for query in [
        "CONSTRUCT { ?i :seen ?n } WHERE { ?r a :Root ; ?slot ?i . ?i a :Item ; :name ?n . FILTER EXISTS { ?i rdf:_1 ?child } }",
        "CONSTRUCT { ?i :seen ?n } WHERE { ?r a :Root ; ?slot ?i . ?i :name ?n . FILTER NOT EXISTS { ?i rdf:_1 ?child } }",
        "CONSTRUCT { ?i :first ?t } WHERE { ?i a :Item . OPTIONAL { ?i rdf:_1 ?n . ?n rdf:_1 ?t . FILTER(STRSTARTS(?t, \"f\")) } }",
        "CONSTRUCT { ?i :named ?n } WHERE { { ?i :name ?n } UNION { ?i :id ?n } UNION { ?i :name ?n } }",
        "CONSTRUCT { ?i :count ?c ; :least ?m ; :any ?s } WHERE { SELECT ?i (COUNT(?x) AS ?c) (MIN(?x) AS ?m) (SAMPLE(?x) AS ?s) WHERE { ?i rdf:_1 ?x } GROUP BY ?i }",
        "CONSTRUCT { ?i :count ?c } WHERE { SELECT ?i (COUNT(DISTINCT ?x) AS ?c) WHERE { ?i ?s ?x . ?i a :Note } GROUP BY ?i HAVING (COUNT(?x) > 1) }",
        "CONSTRUCT { ?i :kind ?k } WHERE { ?i :id ?id . VALUES (?id ?k) { (\"1\" :first) (\"2\" :second) (\"3\" :third) } }",
        "CONSTRUCT { ?i :ratio ?r } WHERE { ?i :id ?id . BIND(xsd:integer(?id) AS ?n) BIND(10 / ?n AS ?r) }",
        "CONSTRUCT { ?i :label ?l } WHERE { ?i a :Item ; :id ?id . ?row :key ?key ; :label ?l . FILTER(?key = ?id) }",
        "CONSTRUCT { ?i :label ?l } WHERE { ?i a :Item ; :id ?id . ?row :key ?k ; :label ?l . FILTER(CONCAT(\"\", ?k) = STR(?id)) }",
        "CONSTRUCT { ?o :text ?t } WHERE { ?o a :Note ; ?s ?t . FILTER(isLiteral(?t) && STRSTARTS(STR(?s), STR(rdf:_))) }",
        "CONSTRUCT { ?i :pair ?j } WHERE { { SELECT DISTINCT ?i ?j WHERE { ?i ?s ?x . ?j ?t ?x . ?i a :Note . ?j a :Note } } }",
        "CONSTRUCT { [] :holds ?i ; :named ?n } WHERE { ?i a :Item ; :name ?n }",
        "CONSTRUCT { ?n :reaches ?m } WHERE { :p (!rdf:type)* ?m . BIND(:p AS ?n) }",
        "CONSTRUCT { ?n :reaches ?m } WHERE { :p (!rdf:type)+ ?m . BIND(:p AS ?n) }",
        "CONSTRUCT { ?x :total ?t } WHERE { SELECT ?x (SUM(?s) AS ?t) WHERE { ?x a :Item ; :size ?s . FILTER(isLiteral(?s) && ?s > 0) } GROUP BY ?x }",
        "CONSTRUCT { ?i :bigger ?j } WHERE { ?i :size ?a . ?j :size ?b . FILTER(?a < ?b && ?i != ?j) }",
    ] {
        assert!(answers_alike(query), "{query} answers nothing, so it shows nothing");
    }
}

#[test]
fn evaluates_each_builtin_to_the_value_the_general_evaluator_gives() {
    let mut dropped = Vec::new();
    for expression in [
        "STR(?v)",
        "STRLEN(?v)",
        "CONCAT(?v, \"-\", ?n)",
        "SUBSTR(?v, 3, 4)",
        "SUBSTR(?v, 8)",
        "SUBSTR(?v, 0, 2)",
        "STRAFTER(?v, \", \")",
        "STRBEFORE(?v, \",\")",
        "STRAFTER(?v, \"no\")",
        "REPLACE(?w, \"^[ ]+|[ ]+$\", \"\")",
        "REPLACE(?v, \"(o)\", \"[$1]\", \"i\")",
        "LCASE(?v)",
        "UCASE(?v)",
        "ENCODE_FOR_URI(?v)",
        "SHA256(?v)",
        "IRI(CONCAT(\"urn:x:\", ENCODE_FOR_URI(?v)))",
        "STRDT(?n, xsd:integer)",
        "STRDT(\"2020-01-02\", xsd:date)",
        "STRDT(CONCAT(\"2020-01-02T03:04:05\", \"\"), xsd:dateTime)",
        "xsd:integer(?n) + 1",
        "xsd:integer(?n) * xsd:integer(?n) - 2",
        "xsd:integer(?n) / 4",
        "1 / 0",
        "xsd:integer(?v)",
        "IF(REGEX(?v, \"^hello\", \"i\"), \"yes\", \"no\")",
        "IF(CONTAINS(?v, \"World\") && !STRENDS(?v, \"x\"), 1, 2)",
        "COALESCE(?missing, STRAFTER(?v, \"!\"), ?n)",
        "IF(BOUND(?missing), \"bound\", \"unbound\")",
        "IF(?v IN (\"a\", \"Hello, World\"), \"in\", \"out\")",
        "IF(?v NOT IN (\"a\", \"b\"), \"out\", \"in\")",
        "IF(isIRI(?v) || isBlank(?v) || isLiteral(?v), \"term\", \"none\")",
        "IF(sameTerm(?v, ?v) && ?v = ?v && ?v != \"x\", \"same\", \"differ\")",
        "IF(STRLEN(?v) > 3 && STRLEN(?v) >= 12 && STRLEN(?v) < 20 && STRLEN(?v) <= 12, \"ordered\", \"unordered\")",
        "IF(?v = 1, \"equal\", \"not\")",
    ] {
        let query = format!(
            "CONSTRUCT {{ :v1 :out ?o }} WHERE {{ :v1 :v ?v ; :n ?n ; :w ?w . BIND({expression} AS ?o) }}"
        );
        if !answers_alike(&query) {
            dropped.push(expression);
        }
    }
    assert_eq!(dropped, ["SUBSTR(?v, 0, 2)", "1 / 0", "xsd:integer(?v)"]);
}

#[test]
fn declines_each_construct_it_does_not_read_and_says_which() {
    for (query, reason) in [
        ("SELECT ?s WHERE { ?s a :Item }", "CONSTRUCT"),
        ("CONSTRUCT { ?s :p ?o } WHERE { ?s ?p ?o }", "nothing bound"),
        ("CONSTRUCT { ?s a :Item } WHERE { ?s a :Item } ORDER BY ?s", "ORDER BY"),
        ("CONSTRUCT { ?s a :Item } WHERE { ?s a :Item } LIMIT 1", "LIMIT"),
        ("CONSTRUCT { ?s a :Item } WHERE { ?s a :Item MINUS { ?s :id \"1\" } }", "MINUS"),
        ("CONSTRUCT { ?s :p ?o } WHERE { ?s rdf:_1|rdf:_2 ?o }", "property path"),
        ("CONSTRUCT { ?s :p ?o } WHERE { :p :next+ ?o . BIND(:p AS ?s) }", "property path"),
        ("CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item ; :id ?i . BIND(NOW() AS ?o) }", "NOW"),
        ("CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item ; :id ?i . BIND(REGEX(?i, ?s) AS ?o) }", "regular expression"),
        ("CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item ; :id ?i . BIND(xsd:double(?i) AS ?o) }", "function"),
        ("CONSTRUCT { ?s :p ?o } WHERE { SELECT ?s (GROUP_CONCAT(?i) AS ?o) WHERE { ?s :id ?i } GROUP BY ?s }", "aggregate"),
        ("CONSTRUCT { ?s :p ?o } WHERE { ?s :id ?i . { ?s :name ?o FILTER(?i = \"1\") } }", "reads a variable"),
        ("CONSTRUCT { ?s :p ?o } WHERE { ?s :id ?o . ?s :size 07 }", "typed literal"),
        ("CONSTRUCT { ?s :p ?o } WHERE { ?s :id ?o . VALUES ?o { 07 } }", "typed literal"),
        ("CONSTRUCT { ?s :p ?o } WHERE { GRAPH ?g { ?s :id ?o } }", "GRAPH"),
    ] {
        let declined = compile(&parse(query)).err().unwrap_or_default();
        assert!(declined.contains(reason), "{query}: {declined}");
    }
}

#[test]
fn gives_a_query_up_where_a_value_is_one_no_builtin_covers() {
    let store = graph();
    for query in [
        "CONSTRUCT { ?i :big ?s } WHERE { ?i :size ?s . FILTER(?s > 4.5) }",
        "CONSTRUCT { :v2 :joined ?o } WHERE { :v2 :v ?v . BIND(CONCAT(?v, \"!\") AS ?o) }",
        "CONSTRUCT { ?i :total ?t } WHERE { SELECT ?i (SUM(?s) AS ?t) WHERE { ?i :size ?s } GROUP BY ?i }",
    ] {
        assert_eq!(compiled(query, &store), Err(Fail::GiveUp), "{query}");
    }
}

struct Text(String);

impl Resolver for Text {
    fn root(&self) -> &str {
        "urn:t:"
    }

    fn read(&self, _: &str) -> Result<Vec<u8>> {
        Ok(self.0.clone().into_bytes())
    }
}

fn read(query: &str) -> Query {
    Query::read(
        &Text(format!("{PREFIXES}{query}")),
        "urn:t:query.rq",
        Form::Construct,
        "mapping",
    )
    .expect("a query")
}

fn answered(query: &Query, executor: Executor) -> (Vec<String>, usize) {
    let store = graph();
    let profile = Profile::new(executor, true);
    let quads = query
        .graph(&Dataset::new(&store), &profile)
        .expect("constructed");
    let give_ups = profile.queries().iter().map(|q| q.give_ups).sum();
    (canonical_parts(quads).expect("canonical"), give_ups)
}

#[test]
fn answers_a_given_up_query_as_the_general_evaluator_and_counts_it() {
    let query =
        read("CONSTRUCT { :v2 :joined ?o } WHERE { :v2 :v ?v . BIND(CONCAT(?v, \"!\") AS ?o) }");
    let (answer, give_ups) = answered(&query, Executor::Compiled);
    assert_eq!((answer.len(), give_ups), (1, 1));
    assert_eq!(answer, answered(&query, Executor::Spareval).0);
}

#[test]
fn runs_a_query_both_ways_and_answers_where_they_agree() {
    let query = read("CONSTRUCT { ?i :named ?n } WHERE { ?i a :Item ; :name ?n }");
    let (answer, give_ups) = answered(&query, Executor::Both);
    assert_eq!((answer.len(), give_ups), (2, 0));
}

#[test]
fn names_the_query_and_the_triple_where_the_two_executors_differ() {
    let store = graph();
    let theirs = compile(&parse(
        "CONSTRUCT { ?i :named ?n } WHERE { ?i a :Item ; :name ?n }",
    ))
    .expect("compiled")
    .construct(&Tree::of(&store).expect("a tree"))
    .expect("constructed");
    let refused = agree("urn:t:query.rq", &theirs[1..], &theirs).expect_err("a difference");
    assert!(
        refused.message().contains("urn:t:query.rq")
            && refused.message().contains("general evaluator")
            && refused.message().contains("urn:t#named"),
        "{refused}"
    );
}

#[test]
fn converts_the_tiny_adapter_alike_under_every_executor() {
    let of = |executor| {
        let conversion = conversion_under(&tiny(), "two.xml", executor).expect("conversion");
        (
            canonical_parts(conversion.quads).expect("canonical"),
            canonical_parts(conversion.findings).expect("canonical"),
        )
    };
    let general = of(Executor::Spareval);
    assert!(!general.0.is_empty() && !general.1.is_empty());
    assert_eq!(of(Executor::Compiled), general);
    assert_eq!(of(Executor::Both), general);
}
