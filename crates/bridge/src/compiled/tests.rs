use super::builtin::BUILTINS;
use super::{agree, compile, Dataset, Executor, Fail, Tree};
use crate::budget::{Budgets, Watch};
use crate::error::Result;
use crate::fixtures::{conversion_under, profiled_under, tiny, tiny_json, Variant};
use crate::query::{Form, Profile, Query, QueryProfile};
use crate::rdf::canonical_parts;
use crate::resolver::Resolver;
use oxigraph::model::{GraphName, Quad};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::HashSet;

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
:g a :Thing ; :m "z" , "a" , "m" .
"#;

const GIVEN_UP: &[(&str, &str)] = &[
    (
        "a value no builtin covers in a filter",
        "CONSTRUCT { ?i :big ?s } WHERE { ?i :size ?s . FILTER(?s > 4.5) }",
    ),
    (
        "a value no builtin covers in a BIND",
        "CONSTRUCT { :v2 :joined ?o } WHERE { :v2 :v ?v . BIND(CONCAT(?v, \"!\") AS ?o) }",
    ),
    (
        "a SUM over a value no builtin covers",
        "CONSTRUCT { ?i :total ?t } WHERE { SELECT ?i (SUM(?s) AS ?t) WHERE { ?i :size ?s } GROUP BY ?i }",
    ),
    (
        "a MIN over a string and an integer, tested by EXISTS",
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . FILTER EXISTS { { SELECT (MIN(?v) AS ?m) WHERE { ?x :size ?v } } } }",
    ),
    (
        "a MIN over a string and an integer, tested by a BIND of EXISTS",
        "CONSTRUCT { ?i :seen ?e } WHERE { ?i a :Item . BIND(EXISTS { { SELECT (MIN(?v) AS ?m) WHERE { ?x :size ?v } } } AS ?e) }",
    ),
    (
        "a MAX over a string and an integer",
        "CONSTRUCT { :root :largest ?m } WHERE { SELECT (MAX(?v) AS ?m) WHERE { ?x :size ?v } }",
    ),
    (
        "a SAMPLE of a group whose values differ",
        "CONSTRUCT { ?g :any ?s } WHERE { ?g a :Thing . { SELECT ?g (SAMPLE(?x) AS ?s) WHERE { { ?g :m ?x FILTER(?x != \"a\") } UNION { ?g :m ?x FILTER(?x = \"a\") } } GROUP BY ?g } }",
    ),
];

const LOWERED: &[(&str, &str)] = &[
    (
        "a seeded EXISTS",
        "CONSTRUCT { ?i :seen ?n } WHERE { ?r a :Root ; ?slot ?i . ?i a :Item ; :name ?n . FILTER EXISTS { ?i rdf:_1 ?child } }",
    ),
    (
        "a seeded NOT EXISTS",
        "CONSTRUCT { ?i :seen ?n } WHERE { ?r a :Root ; ?slot ?i . ?i :name ?n . FILTER NOT EXISTS { ?i rdf:_1 ?child } }",
    ),
    (
        "an OPTIONAL with a condition of its own",
        "CONSTRUCT { ?i :first ?t } WHERE { ?i a :Item . OPTIONAL { ?i rdf:_1 ?n . ?n rdf:_1 ?t . FILTER(STRSTARTS(?t, \"f\")) } }",
    ),
    (
        "a UNION of three branches",
        "CONSTRUCT { ?i :named ?n } WHERE { { ?i :name ?n } UNION { ?i :id ?n } UNION { ?i :name ?n } }",
    ),
    (
        "GROUP BY with COUNT, MIN and SAMPLE",
        "CONSTRUCT { ?i :count ?c ; :least ?m ; :any ?s } WHERE { SELECT ?i (COUNT(?x) AS ?c) (MIN(?x) AS ?m) (SAMPLE(?x) AS ?s) WHERE { ?i rdf:_1 ?x } GROUP BY ?i }",
    ),
    (
        "GROUP BY with MAX and COUNT(*)",
        "CONSTRUCT { ?i :last ?m ; :rows ?c } WHERE { SELECT ?i (MAX(?x) AS ?m) (COUNT(*) AS ?c) WHERE { ?i ?s ?x . ?i a :Note } GROUP BY ?i }",
    ),
    (
        "an aggregate with no key",
        "CONSTRUCT { :root :count ?c } WHERE { SELECT (COUNT(?x) AS ?c) WHERE { :a rdf:_1 ?x } }",
    ),
    (
        "COUNT(DISTINCT) with HAVING",
        "CONSTRUCT { ?i :count ?c } WHERE { SELECT ?i (COUNT(DISTINCT ?x) AS ?c) WHERE { ?i ?s ?x . ?i a :Note } GROUP BY ?i HAVING (COUNT(?x) > 1) }",
    ),
    (
        "SUM over a filtered group",
        "CONSTRUCT { ?x :total ?t } WHERE { SELECT ?x (SUM(?s) AS ?t) WHERE { ?x a :Item ; :size ?s . FILTER(isLiteral(?s) && ?s > 0) } GROUP BY ?x }",
    ),
    (
        "a VALUES table joined on a variable",
        "CONSTRUCT { ?i :kind ?k } WHERE { ?i :id ?id . VALUES (?id ?k) { (\"1\" :first) (\"2\" :second) (\"3\" :third) } }",
    ),
    (
        "a BIND of a constant",
        "CONSTRUCT { ?i :is ?f } WHERE { ?i a :Item . BIND(:flag AS ?f) }",
    ),
    (
        "a BIND of a cast and a division",
        "CONSTRUCT { ?i :ratio ?r } WHERE { ?i :id ?id . BIND(xsd:integer(?id) AS ?n) BIND(10 / ?n AS ?r) }",
    ),
    (
        "a filter equating two patterns' values",
        "CONSTRUCT { ?i :label ?l } WHERE { ?i a :Item ; :id ?id . ?row :key ?key ; :label ?l . FILTER(?key = ?id) }",
    ),
    (
        "a filter equating the result of builtins",
        "CONSTRUCT { ?i :label ?l } WHERE { ?i a :Item ; :id ?id . ?row :key ?k ; :label ?l . FILTER(CONCAT(\"\", ?k) = STR(?id)) }",
    ),
    (
        "a key from a BIND that follows the patterns it reads, ahead of the pattern it keys",
        "CONSTRUCT { ?i :label ?l } WHERE { ?i a :Item ; :id ?id . ?row :key ?key ; :label ?l . BIND(CONCAT(?id, \"\") AS ?k) FILTER(?key = ?k) }",
    ),
    (
        "a key under a conditional the branches' constants decide",
        "CONSTRUCT { ?i :label ?l } WHERE { ?i a :Item ; :id ?id . { { ?row :key ?key ; :label ?l BIND(:p AS ?gap) } UNION { ?row :key ?key ; :label ?l BIND(:q AS ?gap) } } FILTER(IF(?gap = :q, ?key != ?id, ?key = ?id)) }",
    ),
    (
        "a filter equating a value with an expression whose value is not a string",
        "CONSTRUCT { ?i :same ?j } WHERE { ?i a :Item ; :size ?s . ?j a :Item ; :size ?t . FILTER(?t = ?s) }",
    ),
    (
        "a filter equating a value with a string no language-tagged value equals",
        "CONSTRUCT { ?a :same ?b } WHERE { ?a :v ?x . ?b :v ?y . FILTER(?y = STR(?x)) }",
    ),
    (
        "a BIND of a variable a group around it binds",
        "CONSTRUCT { ?i :named ?n } WHERE { ?i :id ?n . { ?i :id ?id BIND(?id AS ?n) } }",
    ),
    (
        "a BIND of a constant, the group around it binding the same and another",
        "CONSTRUCT { ?i :named ?n } WHERE { ?i a :Item . BIND(\"p\" AS ?c) { { ?i :name ?n BIND(\"p\" AS ?c) } UNION { ?i :id ?n BIND(\"q\" AS ?c) } } }",
    ),
    (
        "an aggregate named for a variable the group around it binds",
        "CONSTRUCT { ?i :count ?c } WHERE { ?i a :Item . BIND(2 AS ?c) { SELECT ?i (COUNT(?x) AS ?c) WHERE { ?i ?p ?x . ?x a :Note } GROUP BY ?i } }",
    ),
    (
        "a filter in a group reading a variable only the groups around it bind, in an EXISTS",
        "CONSTRUCT { ?i :seen ?k } WHERE { ?i a :Item . { ?x :key ?k FILTER EXISTS { ?i :name \"alpha\" } } }",
    ),
    (
        "a filter in a group reading a variable only the groups around it bind, in BOUND",
        "CONSTRUCT { ?i :seen ?k } WHERE { ?i a :Item . { ?x :key ?k FILTER(!BOUND(?i)) } }",
    ),
    (
        "an EXISTS that shares no variable with its group",
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . FILTER NOT EXISTS { ?z :key \"3\" } }",
    ),
    (
        "a filter on the predicate and the kind of the object",
        "CONSTRUCT { ?o :text ?t } WHERE { ?o a :Note ; ?s ?t . FILTER(isLiteral(?t) && STRSTARTS(STR(?s), STR(rdf:_))) }",
    ),
    (
        "a filter comparing two integers",
        "CONSTRUCT { ?i :bigger ?j } WHERE { ?i :size ?a . ?j :size ?b . FILTER(?a < ?b && ?i != ?j) }",
    ),
    (
        "a SELECT DISTINCT subquery",
        "CONSTRUCT { ?i :pair ?j } WHERE { { SELECT DISTINCT ?i ?j WHERE { ?i ?s ?x . ?j ?t ?x . ?i a :Note . ?j a :Note } } }",
    ),
    (
        "a blank node in the template",
        "CONSTRUCT { [] :holds ?i ; :named ?n } WHERE { ?i a :Item ; :name ?n }",
    ),
    (
        "a walk over all but one edge, from the start",
        "CONSTRUCT { ?n :reaches ?m } WHERE { :p (!rdf:type)* ?m . BIND(:p AS ?n) }",
    ),
    (
        "a walk over all but one edge, not from the start",
        "CONSTRUCT { ?n :reaches ?m } WHERE { :p (!rdf:type)+ ?m . BIND(:p AS ?n) }",
    ),
    (
        "a pattern with a constant object",
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item ; :name \"alpha\" }",
    ),
    (
        "a language-tagged literal in a pattern",
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i :v \"naïve café\"@fr }",
    ),
];

const DECLINED: &[(&str, &str, &str)] = &[
    (
        "a SELECT",
        "SELECT ?s WHERE { ?s a :Item }",
        "it is not a CONSTRUCT",
    ),
    (
        "an ASK",
        "ASK { ?s a :Item }",
        "it is not a CONSTRUCT",
    ),
    (
        "a pattern with nothing bound to start from",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s ?p ?o }",
        "a triple pattern with nothing bound to start from",
    ),
    (
        "ORDER BY",
        "CONSTRUCT { ?s a :Item } WHERE { ?s a :Item } ORDER BY ?s",
        "ORDER BY",
    ),
    (
        "LIMIT",
        "CONSTRUCT { ?s a :Item } WHERE { ?s a :Item } LIMIT 1",
        "LIMIT or OFFSET",
    ),
    (
        "MINUS",
        "CONSTRUCT { ?s a :Item } WHERE { ?s a :Item MINUS { ?s :id \"1\" } }",
        "MINUS",
    ),
    (
        "GRAPH",
        "CONSTRUCT { ?s :p ?o } WHERE { GRAPH ?g { ?s :id ?o } }",
        "GRAPH",
    ),
    (
        "REDUCED",
        "CONSTRUCT { ?s :p ?o } WHERE { { SELECT REDUCED ?s ?o WHERE { ?s :id ?o } } }",
        "REDUCED",
    ),
    (
        "a property path that is an alternative",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s rdf:_1|rdf:_2 ?o }",
        "a property path that is not a walk",
    ),
    (
        "a property path that walks one edge",
        "CONSTRUCT { ?s :p ?o } WHERE { :p :next+ ?o . BIND(:p AS ?s) }",
        "a property path that is not a walk over all but some edges",
    ),
    (
        "a property path from a node not yet bound",
        "CONSTRUCT { ?s :p ?m } WHERE { ?s (!rdf:type)* ?m }",
        "a property path from a node not yet bound",
    ),
    (
        "BNODE()",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item . BIND(BNODE() AS ?o) }",
        "the function BNODE",
    ),
    (
        "a function the compiler has no builtin for",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item ; :id ?i . BIND(xsd:double(?i) AS ?o) }",
        "the function <http://www.w3.org/2001/XMLSchema#double>",
    ),
    (
        "a regular expression that is not written out",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item ; :id ?i . BIND(REGEX(?i, ?s) AS ?o) }",
        "a regular expression that is not written out",
    ),
    (
        "a regular expression the engine refuses",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item ; :id ?o . FILTER(REGEX(?o, \"(\")) }",
        "a regular expression the engine refuses",
    ),
    (
        "an aggregate the compiler has no accumulator for",
        "CONSTRUCT { ?s :p ?o } WHERE { SELECT ?s (GROUP_CONCAT(?i) AS ?o) WHERE { ?s :id ?i } GROUP BY ?s }",
        "an aggregate other than COUNT, SUM, MIN, MAX or SAMPLE",
    ),
    (
        "COUNT(DISTINCT *)",
        "CONSTRUCT { ?s :p ?c } WHERE { SELECT ?s (COUNT(DISTINCT *) AS ?c) WHERE { ?s :id ?i } GROUP BY ?s }",
        "COUNT(DISTINCT *)",
    ),
    (
        "DISTINCT in SUM",
        "CONSTRUCT { ?s :p ?c } WHERE { SELECT ?s (SUM(DISTINCT ?z) AS ?c) WHERE { ?s :size ?z } GROUP BY ?s }",
        "DISTINCT in an aggregate",
    ),
    (
        "an OPTIONAL over a variable another OPTIONAL may leave unbound",
        "CONSTRUCT { ?i :named ?n } WHERE { ?i a :Item . OPTIONAL { ?i :size ?n } { ?i :id ?id OPTIONAL { ?i :name ?n } } }",
        "a group reads a variable only the groups around it bind",
    ),
    (
        "a filter over a variable the group around it binds and it may bind",
        "CONSTRUCT { ?i :named ?n } WHERE { ?i :size ?n . { ?i :id ?id OPTIONAL { ?i :name ?n } FILTER(BOUND(?n)) } }",
        "a group reads a variable only the groups around it bind",
    ),
    (
        "a GROUP BY key only the groups around the subquery bind",
        "CONSTRUCT { ?k :count ?n } WHERE { ?k a :Item . { SELECT ?k (COUNT(*) AS ?n) WHERE { { ?k :id ?v } UNION { ?w :size ?v } } GROUP BY ?k } }",
        "a group reads a variable only the groups around it bind",
    ),
    (
        "a SELECT DISTINCT key only the groups around the subquery bind",
        "CONSTRUCT { [] :k ?k ; :v ?v } WHERE { ?k a :Item . { SELECT DISTINCT ?k ?v WHERE { { ?k :size ?v } UNION { ?w :size ?v } } } }",
        "a group reads a variable only the groups around it bind",
    ),
    (
        "a typed literal in a pattern",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s :id ?o . ?s :size 07 }",
        "a typed literal in a pattern, which a store matches by value",
    ),
    (
        "a typed literal in VALUES",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s :id ?o . VALUES ?o { 07 } }",
        "a typed literal in VALUES, which a join matches by value",
    ),
];

fn graph() -> Store {
    graph_of(DATA)
}

fn graph_of(data: &str) -> Store {
    let store = Store::new().expect("a store");
    for quad in RdfParser::from_format(RdfFormat::Turtle).for_slice(data) {
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
    let quads = plan.construct(&Tree::of(store, Watch::default()).expect("a tree"))?;
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
fn lowers_each_construct_a_mapping_query_uses_and_answers_as_the_general_evaluator_does() {
    let silent: Vec<_> = LOWERED
        .iter()
        .filter(|(_, query)| !answers_alike(query))
        .map(|(construct, _)| *construct)
        .collect();
    assert!(
        silent.is_empty(),
        "{silent:?} answer nothing, so show nothing"
    );
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
fn names_each_builtin_once() {
    let mut names = HashSet::new();
    let repeated: Vec<_> = BUILTINS
        .iter()
        .map(|builtin| builtin.name)
        .filter(|name| !names.insert(*name))
        .collect();
    assert!(repeated.is_empty(), "{repeated:?}");
}

#[test]
fn declines_each_construct_it_does_not_read_and_says_which() {
    let wrong: Vec<_> = DECLINED
        .iter()
        .filter_map(|(construct, query, reason)| {
            let declined = compile(&parse(query)).err();
            (declined.as_deref() != Some(*reason)).then_some((*construct, declined))
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn gives_a_query_up_where_a_value_is_one_no_builtin_covers() {
    let store = graph();
    for (construct, query) in GIVEN_UP {
        assert_eq!(
            compiled(query, &store),
            Err(Fail::GiveUp),
            "{construct}: {query}"
        );
    }
}

/// Where a plan that reads a group's variable from the row around it, instead of
/// as unbound, would answer something.
const NOTHING: &[(&str, &str)] = &[
    (
        "a NOT EXISTS over a variable only the groups around it bind",
        "CONSTRUCT { ?i :seen ?k } WHERE { ?i a :Item . { ?x :key ?k FILTER NOT EXISTS { ?i :name \"alpha\" } } }",
    ),
    (
        "a filter comparing a variable only the groups around it bind",
        "CONSTRUCT { ?i :seen ?k } WHERE { ?i :id ?v . { ?x :key ?k FILTER(?v = \"1\") } }",
    ),
    (
        "a BIND that disagrees with the group around it",
        "CONSTRUCT { ?i :named ?n } WHERE { ?i :name ?n . { ?i :id ?id BIND(?id AS ?n) } }",
    ),
    (
        "an aggregate that disagrees with the group around it",
        "CONSTRUCT { ?i :count ?c } WHERE { ?i a :Item . BIND(7 AS ?c) { SELECT ?i (COUNT(?x) AS ?c) WHERE { ?i ?p ?x . ?x a :Note } GROUP BY ?i } }",
    ),
];

#[test]
fn answers_nothing_where_the_general_evaluator_answers_nothing() {
    let store = graph();
    for (construct, query) in NOTHING {
        assert_eq!(
            evaluated(query, &store),
            Vec::<String>::new(),
            "{construct}"
        );
        assert_eq!(compiled(query, &store), Ok(Vec::new()), "{construct}");
    }
}

#[test]
fn answers_an_exists_that_shares_no_variable_afresh_for_each_tree() {
    let query =
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . FILTER EXISTS { ?z :key \"1\" } }";
    let plan = compile(&parse(query)).expect("compiled");
    let keyed = graph();
    let unkeyed = graph_of(&DATA.replace(":one :key \"1\" ; :label \"one\" .", ""));
    for store in [&keyed, &unkeyed, &keyed] {
        let answered = plan
            .construct(&Tree::of(store, Watch::default()).expect("a tree"))
            .expect("constructed");
        assert_eq!(
            canonical_parts(answered).expect("canonical"),
            evaluated(query, store)
        );
    }
    assert_ne!(evaluated(query, &keyed), evaluated(query, &unkeyed));
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

fn answered(query: &Query, executor: Executor) -> (Vec<String>, Vec<QueryProfile>) {
    let store = graph();
    let profile = Profile::new(executor, true, Budgets::default());
    let quads = query
        .graph(&Dataset::new(&store), &profile)
        .expect("constructed");
    (
        canonical_parts(quads).expect("canonical"),
        profile.queries(),
    )
}

#[test]
fn answers_each_declined_construct_as_the_general_evaluator_and_says_why() {
    for &(construct, query, reason) in DECLINED {
        if !query.starts_with("CONSTRUCT") {
            continue;
        }
        let query = read(query);
        let (answer, profiled) = answered(&query, Executor::Compiled);
        let (general, general_profiled) = answered(&query, Executor::Spareval);
        assert_eq!(answer, general, "{construct}");
        assert_eq!(
            profiled
                .iter()
                .map(|q| (q.fallback.as_deref(), q.give_ups))
                .collect::<Vec<_>>(),
            [(Some(reason), 0)],
            "{construct}"
        );
        assert_eq!(
            general_profiled
                .iter()
                .map(|q| q.fallback.as_deref())
                .collect::<Vec<_>>(),
            [None],
            "{construct}: the compiler did not run"
        );
    }
}

#[test]
fn answers_each_given_up_query_as_the_general_evaluator_and_counts_it() {
    for &(construct, query) in GIVEN_UP {
        let query = read(query);
        let (answer, profiled) = answered(&query, Executor::Compiled);
        assert_eq!(
            profiled
                .iter()
                .map(|q| (q.fallback.as_deref(), q.give_ups))
                .collect::<Vec<_>>(),
            [(None, 1)],
            "{construct}"
        );
        assert_eq!(
            answer,
            answered(&query, Executor::Spareval).0,
            "{construct}"
        );
    }
}

#[test]
fn runs_a_query_both_ways_and_answers_where_they_agree() {
    let query = read("CONSTRUCT { ?i :named ?n } WHERE { ?i a :Item ; :name ?n }");
    let (answer, profiled) = answered(&query, Executor::Both);
    assert_eq!(answer.len(), 2);
    assert_eq!(profiled.iter().map(|q| q.give_ups).sum::<usize>(), 0);
}

#[test]
fn names_the_query_and_the_triple_where_the_two_executors_differ() {
    let store = graph();
    let theirs = compile(&parse(
        "CONSTRUCT { ?i :named ?n } WHERE { ?i a :Item ; :name ?n }",
    ))
    .expect("compiled")
    .construct(&Tree::of(&store, Watch::default()).expect("a tree"))
    .expect("constructed");
    let refused = agree("urn:t:query.rq", &theirs[1..], &theirs).expect_err("a difference");
    assert!(
        refused.message().contains("urn:t:query.rq")
            && refused.message().contains("general evaluator")
            && refused.message().contains("urn:t#named"),
        "{refused}"
    );
}

fn converted_under(
    adapter: &dyn Resolver,
    input: &str,
    executor: Executor,
) -> (Vec<String>, Vec<String>) {
    let conversion = conversion_under(adapter, input, executor).expect("conversion");
    (
        canonical_parts(conversion.quads).expect("canonical"),
        canonical_parts(conversion.findings).expect("canonical"),
    )
}

#[test]
fn converts_each_tiny_adapter_alike_under_every_executor() {
    for (adapter, input) in [
        (&tiny() as &dyn Resolver, "two.xml"),
        (&tiny_json(), "item.json"),
    ] {
        let general = converted_under(adapter, input, Executor::Spareval);
        assert!(!general.0.is_empty(), "{input}");
        assert_eq!(
            converted_under(adapter, input, Executor::Compiled),
            general,
            "{input}"
        );
        assert_eq!(
            converted_under(adapter, input, Executor::Both),
            general,
            "{input}"
        );
    }
}

#[test]
fn converts_the_tiny_adapter_with_a_mapping_the_compiler_declines_as_the_general_evaluator_does() {
    let ordered = Variant::of(tiny()).replacing_exactly(
        "mapping/item.rq",
        "?title . }\n}",
        "?title . }\n} ORDER BY ?id",
        1,
    );
    let (compiled, profiled) =
        profiled_under(&ordered, "two.xml", Executor::Compiled).expect("conversion");
    let general = conversion_under(&ordered, "two.xml", Executor::Spareval).expect("conversion");
    assert_eq!(
        canonical_parts(compiled.quads).expect("canonical"),
        canonical_parts(general.quads).expect("canonical")
    );
    let item = profiled
        .iter()
        .find(|q| q.iri.ends_with("/mapping/item.rq"))
        .expect("the item mapping is profiled");
    assert_eq!(item.fallback.as_deref(), Some("ORDER BY"));
    assert_eq!(item.give_ups, 0);
}
