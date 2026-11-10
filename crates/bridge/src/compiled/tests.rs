use super::builtin::BUILTINS;
use super::once::Answer;
use super::{agree, compile, compile_explained, Dataset, Executor, Fail, Shared, Tree};
use crate::budget::{Budgets, Watch};
use crate::error::Result;
use crate::fixtures::{conversion_under, profiled_under, tiny, tiny_json, Variant};
use crate::query::{Form, Profile, Query, QueryProfile};
use crate::rdf::canonical_parts;
use crate::resolver::Resolver;
use oxigraph::model::{GraphName, NamedNode, Quad, Term};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::HashSet;
use std::rc::Rc;

const PREFIXES: &str =
    "PREFIX : <urn:t#> PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> \
                        PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> ";

const DATA: &str = r#"
@prefix : <urn:t#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:root a :Root ; rdf:_1 :a ; rdf:_2 :b ; rdf:_3 :c .
:s1 :system "urn:x:s"^^xsd:anyURI .
:s2 :system "urn:x:s" .
:s3 :system "urn:x:s"^^xsd:token .
:x1 :e :x2 ; :f :x2 .
:x2 :e :x1 .
:x3 :e :x3 .
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
    (
        "a GROUP_CONCAT over integers",
        "CONSTRUCT { :root :sizes ?c } WHERE { SELECT (GROUP_CONCAT(?s; separator=\"|\") AS ?c) WHERE { { SELECT ?s WHERE { ?m :size ?s } ORDER BY ?m } } }",
    ),
    (
        "a GROUP_CONCAT of a variable some rows leave unbound",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT (GROUP_CONCAT(?n; separator=\"|\") AS ?c) WHERE { { SELECT ?m ?n WHERE { ?m a :Item OPTIONAL { ?m :name ?n } } ORDER BY ?m } } }",
    ),
    (
        "a GROUP_CONCAT whose ORDER BY ties on rows that carry different values",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT (GROUP_CONCAT(?n; separator=\"|\") AS ?c) WHERE { { SELECT ?g ?n WHERE { ?g :m ?n } ORDER BY ?g } } }",
    ),
    (
        "a GROUP_CONCAT whose ORDER BY ties rows of two keys, one of which carries different values",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT ?k (GROUP_CONCAT(?n; separator=\"|\") AS ?c) WHERE { { SELECT ?k ?n WHERE { { ?g :m ?n BIND(1 AS ?k) } UNION { ?i :name ?n BIND(2 AS ?k) } BIND(\"t\" AS ?r) } ORDER BY ?r } } GROUP BY ?k }",
    ),
    (
        "a GROUP_CONCAT in the order of integers",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT (GROUP_CONCAT(?n; separator=\"|\") AS ?c) WHERE { { SELECT ?n WHERE { ?m :name ?n ; :size ?s } ORDER BY ?s } } }",
    ),
    (
        "a GROUP_CONCAT in the order of language-tagged strings",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT (GROUP_CONCAT(STR(?v); separator=\"|\") AS ?c) WHERE { { SELECT ?v WHERE { ?m :v ?v } ORDER BY ?v } } }",
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
        "a projection joins the variable an aggregate is named for, which the group around it binds",
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
        "a GROUP_CONCAT in the order of an ORDER BY beneath it, descending",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT (GROUP_CONCAT(?n; separator=\"|\") AS ?c) WHERE { { SELECT ?n WHERE { ?m a :Item ; :name ?n } ORDER BY DESC(?n) } } }",
    ),
    (
        "a GROUP_CONCAT whose ORDER BY ties on rows that carry the same value",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT (GROUP_CONCAT(?n; separator=\"|\") AS ?c) WHERE { { SELECT ?g ?n WHERE { ?g :m ?x . BIND(\"k\" AS ?n) } ORDER BY ?g } } }",
    ),
    (
        "a GROUP_CONCAT for each key whose ORDER BY ties on rows of different keys that carry different values",
        "CONSTRUCT { ?i :names ?c } WHERE { SELECT ?i (GROUP_CONCAT(?n; separator=\"|\") AS ?c) WHERE { { SELECT ?i ?n WHERE { ?i a :Item ; :name ?n . BIND(\"t\" AS ?r) } ORDER BY ?r } } GROUP BY ?i }",
    ),
    (
        "a GROUP_CONCAT for each key, in the order of two variables",
        "CONSTRUCT { ?g :letters ?c } WHERE { SELECT ?g (GROUP_CONCAT(?l; separator=\"\") AS ?c) WHERE { { SELECT ?g ?l WHERE { ?g :m ?l } ORDER BY ?g ?l } } GROUP BY ?g }",
    ),
    (
        "a GROUP_CONCAT in the order of a variable the subquery does not project, with the default separator",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT (GROUP_CONCAT(?n) AS ?c) WHERE { { SELECT ?n WHERE { ?m :id ?id ; :name ?n } ORDER BY DESC(?id) } } }",
    ),
    (
        "a GROUP_CONCAT over no rows",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT (GROUP_CONCAT(?n; separator=\"|\") AS ?c) WHERE { { SELECT ?n WHERE { ?m :absent ?n } ORDER BY ?n } } }",
    ),
    (
        "a GROUP_CONCAT in the order of a variable some rows leave unbound",
        "CONSTRUCT { :root :names ?c } WHERE { SELECT (GROUP_CONCAT(?id; separator=\"|\") AS ?c) WHERE { { SELECT ?id WHERE { ?m a :Item OPTIONAL { ?m :name ?n } ?m :id ?id } ORDER BY DESC(?n) ?id } } }",
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
        "a walk backwards over all but one edge, from the start",
        "CONSTRUCT { ?n :reached ?m } WHERE { :p ^(!rdf:type)* ?m . BIND(:p AS ?n) }",
    ),
    (
        "a walk backwards over all but one edge, not from the start",
        "CONSTRUCT { ?n :reached ?m } WHERE { :r ^(!rdf:type)+ ?m . BIND(:r AS ?n) }",
    ),
    (
        "a walk written forwards to a node that is known, which is taken backwards from it",
        "CONSTRUCT { ?m :reaches :r } WHERE { ?m (!rdf:type)* :r }",
    ),
    (
        "a walk written forwards to a node an earlier pattern bound",
        "CONSTRUCT { ?m :above ?q } WHERE { ?q a :Node . ?m (!rdf:type)+ ?q }",
    ),
    (
        "a choice of a walk and the same walk backwards, which takes the start once from each",
        "CONSTRUCT { :root :count ?c } WHERE { SELECT (COUNT(*) AS ?c) WHERE { :p ((!rdf:type)*|^(!rdf:type)*) ?m } }",
    ),
    (
        "a choice of a walk and the same walk backwards, to a node that is known",
        "CONSTRUCT { ?m :related :q } WHERE { ?m ((!rdf:type)*|^(!rdf:type)*) :q }",
    ),
    (
        "a pattern with a constant object",
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item ; :name \"alpha\" }",
    ),
    (
        "a language-tagged literal in a pattern",
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i :v \"naïve café\"@fr }",
    ),
    (
        "a literal in a pattern of a datatype a store keeps as written",
        "CONSTRUCT { ?s :seen :yes } WHERE { ?s :system \"urn:x:s\"^^xsd:anyURI }",
    ),
    (
        "a literal in VALUES of a datatype a store keeps as written",
        "CONSTRUCT { ?s :seen :yes } WHERE { ?s :system ?u . VALUES ?u { \"urn:x:s\"^^xsd:anyURI } }",
    ),
    (
        "a choice of two edges",
        "CONSTRUCT { ?i :named ?n } WHERE { ?i a :Item ; :name|:id ?n }",
    ),
    (
        "a choice of edges from a node not yet bound",
        "CONSTRUCT { ?i :named ?n } WHERE { :one :key ?n . ?i :name|:id ?n }",
    ),
    (
        "a choice of an edge and an edge followed backwards",
        "CONSTRUCT { ?i :near ?n } WHERE { ?i a :Item . ?i rdf:_1|^rdf:_2 ?n }",
    ),
    (
        "an edge followed backwards from a choice",
        "CONSTRUCT { ?i :held ?r } WHERE { ?i a :Item . ?i ^(rdf:_1|rdf:_2) ?r }",
    ),
    (
        "a choice of two edges that both link one pair, which it takes once",
        "CONSTRUCT { :root :count ?c } WHERE { SELECT (COUNT(*) AS ?c) WHERE { ?s :e|:f ?o } }",
    ),
    (
        "a choice of one edge twice, which it takes once",
        "CONSTRUCT { :root :count ?c } WHERE { SELECT (COUNT(*) AS ?c) WHERE { ?s :e|:e ?o } }",
    ),
    (
        "a choice of an edge and the same edge backwards, which links a pair both ways once",
        "CONSTRUCT { :root :count ?c } WHERE { SELECT (COUNT(*) AS ?c) WHERE { ?s :e|^:e ?o } }",
    ),
    (
        "a choice of two edges between ends already bound, which holds once",
        "CONSTRUCT { :root :count ?c } WHERE { SELECT (COUNT(*) AS ?c) WHERE { :x1 :e|:f :x2 } }",
    ),
    (
        "BNODE() in a BIND, a node for each row of the patterns before it",
        "CONSTRUCT { :root :count ?c } WHERE { SELECT (COUNT(DISTINCT ?b) AS ?c) WHERE { ?s :e ?o BIND(BNODE() AS ?b) } }",
    ),
    (
        "BNODE() under a condition bound before the last pattern, a node for each row",
        "CONSTRUCT { :root :count ?c } WHERE { SELECT (COUNT(DISTINCT ?b) AS ?c) WHERE { ?s :e ?m . ?m ?edge ?o BIND(IF(BOUND(?s), BNODE(), 1/0) AS ?b) } }",
    ),
    (
        "BNODE() in a BIND, a node for each row where a condition holds",
        "CONSTRUCT { ?i :has ?b . ?b :named ?n } WHERE { ?i a :Item . OPTIONAL { ?i :name ?n } BIND(IF(BOUND(?n), BNODE(), 1/0) AS ?b) }",
    ),
    (
        "an OPTIONAL of a SELECT DISTINCT that names its key by a BIND, some rows unmatched",
        "CONSTRUCT { ?i :named ?m } WHERE { ?i a :Item . OPTIONAL { SELECT DISTINCT (?x AS ?i) (true AS ?m) WHERE { ?x :name ?n } } }",
    ),
    (
        "an OPTIONAL of an aggregate that names its key by a BIND, some rows unmatched",
        "CONSTRUCT { ?i :children ?c } WHERE { ?i a :Item . OPTIONAL { SELECT ?i (COUNT(?y) AS ?c) WHERE { ?x rdf:_1 ?y BIND(?x AS ?i) } GROUP BY ?i } }",
    ),
    (
        "an OPTIONAL of a walk from a constant that names its key by a BIND",
        "CONSTRUCT { ?i :under ?m } WHERE { ?i a :Item . OPTIONAL { SELECT DISTINCT (?node AS ?i) (true AS ?m) WHERE { :a (!rdf:type)* ?node } } }",
    ),
    (
        "a SELECT DISTINCT that names its key by a BIND, joined to the rows it matches",
        "CONSTRUCT { ?i :named :yes } WHERE { ?i a :Item . { SELECT DISTINCT (?x AS ?i) WHERE { ?x :name ?n } } }",
    ),
    (
        "an aggregate that shares no variable with the rows it joins",
        "CONSTRUCT { ?i :of ?c } WHERE { ?i a :Item . { SELECT (COUNT(*) AS ?c) WHERE { ?x :name ?n } } }",
    ),
    (
        "an aggregate that shares no variable with the rows it joins, over nothing",
        "CONSTRUCT { ?i :of ?c } WHERE { ?i a :Item . { SELECT (COUNT(*) AS ?c) WHERE { ?x :absent ?n } } }",
    ),
    (
        "a SELECT DISTINCT that names its key by a BIND, in an OPTIONAL after rows that do not bind the key",
        "CONSTRUCT { :root :seen ?i } WHERE { :root a :Root . OPTIONAL { SELECT DISTINCT (?x AS ?i) WHERE { ?x :name ?n } } }",
    ),
    (
        "a SELECT DISTINCT that names its key by a BIND, in an aggregate that scans the key",
        "CONSTRUCT { ?i :of ?c } WHERE { SELECT ?i (COUNT(?m) AS ?c) WHERE { ?i a :Item OPTIONAL { SELECT DISTINCT (?w AS ?i) (true AS ?m) WHERE { ?w :name ?n } } } GROUP BY ?i }",
    ),
];

/// A subquery that reads nothing from the rows it joins: the construct, the rows
/// it joins to, and the subquery alone.
const ONCE: &[(&str, &str, &str, &str)] = &[
    (
        "a SELECT DISTINCT in an OPTIONAL",
        "CONSTRUCT { ?i :named ?m } WHERE { ?i a :Item . OPTIONAL { SELECT DISTINCT (?x AS ?i) (true AS ?m) WHERE { ?x :name ?n } } }",
        "CONSTRUCT { ?i :named :no } WHERE { ?i a :Item }",
        "CONSTRUCT { ?i :named ?m } WHERE { SELECT DISTINCT (?x AS ?i) (true AS ?m) WHERE { ?x :name ?n } }",
    ),
    (
        "an aggregate in an OPTIONAL",
        "CONSTRUCT { ?i :children ?c } WHERE { ?i a :Item . OPTIONAL { SELECT ?i (COUNT(?y) AS ?c) WHERE { ?x rdf:_1 ?y BIND(?x AS ?i) } GROUP BY ?i } }",
        "CONSTRUCT { ?i :children :no } WHERE { ?i a :Item }",
        "CONSTRUCT { ?i :children ?c } WHERE { SELECT ?i (COUNT(?y) AS ?c) WHERE { ?x rdf:_1 ?y BIND(?x AS ?i) } GROUP BY ?i }",
    ),
    (
        "a walk in a SELECT DISTINCT in an OPTIONAL",
        "CONSTRUCT { ?i :under ?m } WHERE { ?i a :Item . OPTIONAL { SELECT DISTINCT (?node AS ?i) (true AS ?m) WHERE { :a (!rdf:type)* ?node } } }",
        "CONSTRUCT { ?i :under :no } WHERE { ?i a :Item }",
        "CONSTRUCT { ?i :under ?m } WHERE { SELECT DISTINCT (?node AS ?i) (true AS ?m) WHERE { :a (!rdf:type)* ?node } }",
    ),
    (
        "a SELECT DISTINCT whose other key is a plain projected variable the rows do not bind",
        "CONSTRUCT { ?i :named ?n } WHERE { ?i a :Item . OPTIONAL { SELECT DISTINCT (?x AS ?i) ?n WHERE { ?x :name ?n } } }",
        "CONSTRUCT { ?i :named :no } WHERE { ?i a :Item }",
        "CONSTRUCT { ?i :named ?n } WHERE { SELECT DISTINCT (?x AS ?i) ?n WHERE { ?x :name ?n } }",
    ),
    (
        "an aggregate grouped by a plain variable the rows do not bind",
        "CONSTRUCT { ?i :of ?c } WHERE { ?i a :Item . OPTIONAL { SELECT ?n (COUNT(?x) AS ?c) WHERE { ?x :name ?n } GROUP BY ?n } }",
        "CONSTRUCT { ?i :of :no } WHERE { ?i a :Item }",
        "CONSTRUCT { ?n :of ?c } WHERE { SELECT ?n (COUNT(?x) AS ?c) WHERE { ?x :name ?n } GROUP BY ?n }",
    ),
    (
        "an aggregate with no variable in common, in a join",
        "CONSTRUCT { ?i :of ?c } WHERE { ?i a :Item . { SELECT (COUNT(*) AS ?c) WHERE { ?x :name ?n } } }",
        "CONSTRUCT { ?i :of :no } WHERE { ?i a :Item }",
        "CONSTRUCT { :root :of ?c } WHERE { SELECT (COUNT(*) AS ?c) WHERE { ?x :name ?n } }",
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
        "a property path that is an edge or none",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item ; rdf:_1? ?o }",
        "a property path other than a choice of edges or a walk",
    ),
    (
        "a choice of a walk and an edge",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item ; (!rdf:type)*|rdf:_1 ?o }",
        "a property path other than a choice of edges or a walk",
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
        "a subquery that makes blank nodes, which stock Oxigraph evaluates once for every row around it",
        "CONSTRUCT { ?i :has ?b } WHERE { ?r a :Root ; ?slot ?i . { SELECT ?i ?b WHERE { ?i a :Item BIND(BNODE() AS ?b) } } }",
        "a subquery that makes blank nodes",
    ),
    (
        "a subquery that makes blank nodes after a UNION of two empty groups, which yields two rows",
        "CONSTRUCT { ?i :has ?b } WHERE { { } UNION { } { SELECT ?i ?b WHERE { ?i a :Item BIND(BNODE() AS ?b) } } }",
        "a subquery that makes blank nodes",
    ),
    (
        "a subquery that makes blank nodes after VALUES of two empty rows",
        "CONSTRUCT { ?i :has ?b } WHERE { VALUES () { () () } { SELECT ?i ?b WHERE { ?i a :Item BIND(BNODE() AS ?b) } } }",
        "a subquery that makes blank nodes",
    ),
    (
        "BNODE of a label",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s a :Item . BIND(BNODE(\"x\") AS ?o) }",
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
        "CONSTRUCT { ?s :p ?o } WHERE { SELECT ?s (AVG(?z) AS ?o) WHERE { ?s :size ?z } GROUP BY ?s }",
        "an aggregate other than COUNT, SUM, MIN, MAX, SAMPLE or GROUP_CONCAT",
    ),
    (
        "a GROUP_CONCAT with no ORDER BY beneath it",
        "CONSTRUCT { ?s :p ?o } WHERE { SELECT ?s (GROUP_CONCAT(?i) AS ?o) WHERE { ?s :id ?i } GROUP BY ?s }",
        "GROUP_CONCAT with no ORDER BY directly beneath it",
    ),
    (
        "an ORDER BY beneath a GROUP_CONCAT that another pattern follows, whose order the general evaluator loses in the join",
        "CONSTRUCT { :root :p ?o } WHERE { SELECT (GROUP_CONCAT(?n) AS ?o) WHERE { { SELECT ?n WHERE { ?m :name ?n } ORDER BY ?n } ?x :id ?k } }",
        "ORDER BY",
    ),
    (
        "an ORDER BY in a subquery no GROUP_CONCAT reads",
        "CONSTRUCT { ?i :named ?n } WHERE { { SELECT ?i ?n WHERE { ?i :name ?n } ORDER BY DESC(?n) } }",
        "ORDER BY",
    ),
    (
        "an ORDER BY beneath a COUNT",
        "CONSTRUCT { :root :count ?c } WHERE { SELECT (COUNT(?n) AS ?c) WHERE { { SELECT ?n WHERE { ?m :name ?n } ORDER BY ?n } } }",
        "ORDER BY",
    ),
    (
        "a GROUP_CONCAT in the order of an expression",
        "CONSTRUCT { :root :p ?o } WHERE { SELECT (GROUP_CONCAT(?n) AS ?o) WHERE { { SELECT ?n WHERE { ?m :name ?n } ORDER BY STRLEN(?n) } } }",
        "ORDER BY on an expression",
    ),
    (
        "an ORDER BY of a SELECT DISTINCT",
        "CONSTRUCT { :root :p ?o } WHERE { SELECT (GROUP_CONCAT(?n) AS ?o) WHERE { { SELECT DISTINCT ?n WHERE { ?m :name ?n } ORDER BY ?n } } }",
        "ORDER BY",
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
        "a BIND over a variable only the groups around it bind",
        "CONSTRUCT { ?i :named ?s } WHERE { ?i :size ?n . { ?i :id ?id BIND(STR(?n) AS ?s) } }",
        "a group reads a variable only the groups around it bind",
    ),
    (
        "a key that reads a variable a filter reads as unbound",
        "CONSTRUCT { ?o :label ?l } WHERE { ?i :id ?d . { ?o :key ?k . ?o :label ?l FILTER(?k = ?d) } }",
        "a key reads a variable a filter reads as unbound",
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
        "a group in an EXISTS reading a variable of the row the EXISTS runs on, in a filter",
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . FILTER EXISTS { ?z :key ?k . { ?y :label ?l FILTER(?i = :a) } } }",
        "a group reads a variable only the groups around it bind",
    ),
    (
        "a group in a NOT EXISTS reading a variable of the row the NOT EXISTS runs on, in a filter",
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . FILTER NOT EXISTS { ?z :key ?k . { ?y :label ?l FILTER(?i = :a) } } }",
        "a group reads a variable only the groups around it bind",
    ),
    (
        "a group in an EXISTS reading a variable of the row the EXISTS runs on, in a NOT EXISTS",
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . FILTER EXISTS { ?z :key ?k . { ?y :label ?l FILTER NOT EXISTS { ?i :name \"alpha\" } } } }",
        "a group reads a variable only the groups around it bind",
    ),
    (
        "an integer in a pattern",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s :id ?o . ?s :size 07 }",
        "a literal in a pattern of a datatype a store matches by value",
    ),
    (
        "a date in a pattern",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s :id ?o . ?s :size \"2020-01-01\"^^xsd:date }",
        "a literal in a pattern of a datatype a store matches by value",
    ),
    (
        "an integer in VALUES",
        "CONSTRUCT { ?s :p ?o } WHERE { ?s :id ?o . VALUES ?o { 07 } }",
        "a literal in VALUES of a datatype a join matches by value",
    ),
];

const KEYED_PLAIN: &str = r#"
@prefix : <urn:t#> .
:a :p 1 .
:a2 :p 2 ; :name "z" .
:b :q "y" ; :r "y" .
:b2 :q "z" ; :r "z" .
"#;

const KEYED_MIXED: &str = r#"
@prefix : <urn:t#> .
:b :q "y" ; :r "y" .
:c :q "y"@en ; :r "y" .
:d :q 7 ; :r "7" .
:e :q [ :s :t ] ; :r "x" .
"#;

const KEYED: &[(&str, &str, &str)] = &[
    (
        "a key read from a variable an OPTIONAL binds in some rows only",
        KEYED_PLAIN,
        "CONSTRUCT { ?b :got ?k } WHERE { ?a :p ?x . OPTIONAL { ?a :name ?m } ?b :q ?k . ?b :r ?m . FILTER(?k = STR(?m)) }",
    ),
    (
        "a key read from a variable a UNION binds in some rows only",
        KEYED_PLAIN,
        "CONSTRUCT { ?b :got ?k } WHERE { { ?a :p ?x } UNION { ?a :p ?x ; :name ?m } ?b :q ?k . ?b :r ?m . FILTER(?k = STR(?m)) }",
    ),
    (
        "a key against plain, language-tagged, typed and blank-node objects",
        KEYED_MIXED,
        "CONSTRUCT { ?b :got ?k } WHERE { ?w :r ?m . ?b :q ?k . FILTER(?k = STR(?m)) }",
    ),
    (
        "an equality inside a disjunction, which keys nothing",
        KEYED_PLAIN,
        "CONSTRUCT { ?b :got ?k } WHERE { ?w :r ?m . ?b :q ?k . FILTER(?k = STR(?m) || ?k = \"z\") }",
    ),
];

/// Each with the data that tells the general evaluator from a plan that is wrong.
const DIVERGENT: &[(&str, &str, &str)] = &[
    (
        "a BIND joined with a dateTime of another offset",
        r#"@prefix : <urn:t#> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:i :when "2020-01-01T00:00:00Z"^^xsd:dateTime ; :at "2020-01-01T01:00:00+01:00"^^xsd:dateTime .
:j :when "2020-01-01T00:00:00Z"^^xsd:dateTime ; :at "2020-01-01T00:00:00Z"^^xsd:dateTime ."#,
        "CONSTRUCT { ?i :named ?n } WHERE { ?i :when ?n . { ?i :at ?d BIND(?d AS ?n) } }",
    ),
    (
        "an EXISTS in an EXISTS reading a variable an OPTIONAL binds in some rows only",
        r#"@prefix : <urn:t#> .
:a a :Item ; :name "alpha" . :b a :Item . :c a :Item ; :name "one" .
:one :key "1" ; :label "one" ."#,
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . OPTIONAL { ?i :name ?n } FILTER EXISTS { ?z :key ?k FILTER EXISTS { ?y :label ?n } } }",
    ),
    (
        "an EXISTS in an EXISTS reading a variable bound in every row",
        r#"@prefix : <urn:t#> .
:a a :Item ; :name "alpha" . :c a :Item ; :name "one" .
:one :key "1" ; :label "one" ."#,
        "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item ; :name ?n . FILTER EXISTS { ?z :key ?k FILTER EXISTS { ?y :label ?n } } }",
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

fn explained(query: &str) -> String {
    compile_explained(&parse(query))
        .expect("a query the compiler reads")
        .to_string()
}

/// Whether the query answered anything.
fn answers_alike(query: &str) -> bool {
    let store = graph();
    let theirs = evaluated(query, &store);
    assert_eq!(compiled(query, &store), Ok(theirs.clone()), "{query}");
    let (own, shared) = split(&store);
    let data = Dataset::over(&own, &shared);
    let plan = compile(&parse(query)).expect("a query the compiler reads");
    let over_shared = plan
        .construct(data.tree(&Watch::default()).expect("a tree"))
        .map(|quads| canonical_parts(quads).expect("canonical"));
    assert_eq!(over_shared, Ok(theirs.clone()), "{query} over shared quads");
    assert_eq!(
        evaluated(query, data.store().expect("filled")),
        theirs,
        "{query} evaluated over shared quads"
    );
    let (own, shared) = split(&store);
    let held = own.len().expect("a count");
    let data = Dataset::over(&own, &shared);
    data.store().expect("filled");
    let tree = data.tree(&Watch::default()).expect("a tree");
    assert_eq!(
        tree.own_triples(),
        held - 1,
        "{query}: the filled store's shared quads re-indexed"
    );
    let over_filled = plan
        .construct(tree)
        .map(|quads| canonical_parts(quads).expect("canonical"));
    assert_eq!(
        over_filled,
        Ok(theirs.clone()),
        "{query} over a filled store"
    );
    !theirs.is_empty()
}

/// A store's quads split between a record's own and a document's shared, one quad in both.
fn split(store: &Store) -> (Store, Shared) {
    let quads: Vec<Quad> = store.iter().map(|quad| quad.expect("a quad")).collect();
    let own = Store::new().expect("a store");
    let mut shared = vec![quads[0].clone()];
    for (at, quad) in quads.iter().enumerate() {
        if at % 2 == 0 {
            own.insert(quad).expect("inserted");
        } else {
            shared.push(quad.clone());
        }
    }
    (own, Shared::new(shared))
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

fn scans_of(query: &str, store: &Store) -> usize {
    let tree = Tree::of(store, Watch::default()).expect("a tree");
    compile(&parse(query))
        .expect("a query the compiler reads")
        .construct(&tree)
        .expect("constructed");
    tree.scans()
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

/// Choosing how queries are executed · Executor, Loaded::explain(), Fallback
///
/// The Bridge runs a query with its own compiled executor where the compiler reads it, and with
/// the general evaluator (Oxigraph's) where it does not. The graph is the same either way;
/// `Executor::Both` runs both and refuses a difference.
mod choosing_how_queries_are_executed {
    use super::*;

    mod given_a_query_the_compiler_compiles {
        use super::*;

        #[test]
        fn answers_each_key_as_the_general_evaluator_does() {
            let differing: Vec<_> = KEYED
                .iter()
                .filter_map(|(construct, data, query)| {
                    let store = graph_of(data);
                    let theirs = evaluated(query, &store);
                    assert!(
                        !theirs.is_empty(),
                        "{construct} answers nothing, so shows nothing"
                    );
                    let ours = compiled(query, &store);
                    (ours != Ok(theirs.clone())).then_some((*construct, ours, theirs))
                })
                .collect();
            assert!(differing.is_empty(), "{differing:#?}");
        }

        #[test]
        fn answers_as_the_general_evaluator_does_where_the_data_tells_a_wrong_plan_apart() {
            let differing: Vec<_> = DIVERGENT
                .iter()
                .filter_map(|(construct, data, query)| {
                    let store = graph_of(data);
                    let (ours, theirs) = (compiled(query, &store), evaluated(query, &store));
                    (ours != Ok(theirs.clone())).then_some((*construct, ours, theirs))
                })
                .collect();
            assert!(differing.is_empty(), "{differing:#?}");
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
            assert!(explained(query).contains("exists, remembered as memo 0:"));
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

        #[test]
        fn offers_every_tuple_of_an_answer_where_one_leaves_a_key_the_row_binds_unbound() {
            let term = |name: &str| {
                Rc::new(Term::from(NamedNode::new_unchecked(format!(
                    "urn:t#{name}"
                ))))
            };
            let (a, b) = (term("a"), term("b"));
            let bound = [(0, Rc::clone(&a))];
            let complete = Answer::of(vec![
                vec![Some(Rc::clone(&a))],
                vec![Some(Rc::clone(&b))],
                vec![Some(Rc::clone(&a))],
            ]);
            assert_eq!(complete.agreeing(&bound), [0, 2]);
            assert_eq!(complete.agreeing(&[(0, term("c"))]), Vec::<usize>::new());
            assert_eq!(complete.agreeing(&[]), [0, 1, 2]);
            let incomplete = Answer::of(vec![vec![Some(a)], vec![None]]);
            assert_eq!(incomplete.agreeing(&bound), [0, 1]);
        }

        #[test]
        fn runs_a_subquery_that_reads_nothing_from_the_rows_it_joins_once_for_the_tree() {
            let store = graph();
            for &(construct, query, rows, subquery) in ONCE {
                assert!(
                    evaluated(rows, &store).len() > 1,
                    "{construct}: one row would not tell once from each"
                );
                assert_eq!(
                    scans_of(query, &store),
                    scans_of(rows, &store) + scans_of(subquery, &store),
                    "{construct}"
                );
            }
        }

        #[test]
        fn answers_two_plans_that_each_remember_an_exists_alike_on_one_tree() {
            let store = graph();
            let tree = Tree::of(&store, Watch::default()).expect("a tree");
            let found =
                "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . FILTER EXISTS { ?z :key \"1\" } }";
            let missing =
                "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . FILTER EXISTS { ?z :key \"9\" } }";
            assert_ne!(evaluated(found, &store), evaluated(missing, &store));
            for query in [found, missing, found] {
                let answered = compile(&parse(query))
                    .expect("compiled")
                    .construct(&tree)
                    .expect("constructed");
                assert_eq!(
                    canonical_parts(answered).expect("canonical"),
                    evaluated(query, &store),
                    "{query}"
                );
            }
        }
    }

    mod given_a_query_the_compiler_declines {
        use super::*;

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

        /// converts with a mapping the compiler declines as the general evaluator does, and names the construct in `QueryProfile::fallback`
        #[test]
        fn converts_with_a_mapping_the_compiler_declines_as_the_general_evaluator_does_and_names_the_construct_in_queryprofile_fallback(
        ) {
            let ordered = Variant::of(tiny()).replacing_exactly(
                "mapping/item.rq",
                "?title . }\n}",
                "?title . }\n} ORDER BY ?id",
                1,
            );
            let (compiled, profiled) =
                profiled_under(&ordered, "two.xml", Executor::Compiled).expect("conversion");
            let general =
                conversion_under(&ordered, "two.xml", Executor::Spareval).expect("conversion");
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
    }

    mod given_a_query_a_plan_gives_up_on {
        use super::*;

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
    }

    /// given `Executor::Both`
    mod given_executor_both {
        use super::*;

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
    }

    /// `explain()`
    mod explain {
        use super::*;

        #[test]
        fn prints_a_plan_by_its_steps_with_variables_by_name_and_the_exists_it_remembers() {
            let query = "CONSTRUCT { ?i :seen ?n } WHERE { ?r a :Root ; ?slot ?i . ?i :name ?n . FILTER EXISTS { ?i rdf:_1 ?child } FILTER EXISTS { ?z :key \"1\" } }";
            let other =
                "CONSTRUCT { ?i :seen :yes } WHERE { ?i a :Item . FILTER EXISTS { ?z :key \"1\" } }";
            let first = explained(query);
            explained(other);
            assert_eq!(explained(query), first);
            assert_eq!(
                first,
                concat!(
                    "filter EXISTS { ?z <urn:t#key> \"1\" . }\n",
                    "  exists, remembered as memo 0:\n",
                    "    scan ?z <urn:t#key> \"1\"\n",
                    "scan ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:t#Root>\n",
                    "scan ?r ?slot ?i\n",
                    "filter EXISTS { ?i <http://www.w3.org/1999/02/22-rdf-syntax-ns#_1> ?child . }\n",
                    "  exists, run for each row:\n",
                    "    scan ?i <http://www.w3.org/1999/02/22-rdf-syntax-ns#_1> ?child\n",
                    "scan ?i <urn:t#name> ?n\n",
                    "construct\n",
                    "  ?i <urn:t#seen> ?n\n",
                )
            );
        }

        #[test]
        fn prints_the_same_plan_each_time_a_query_is_parsed_and_tells_shadowed_variables_apart() {
            let query = "CONSTRUCT { ?i :seen ?c } WHERE { ?i :name ?code . { SELECT ?i (COUNT(*) AS ?c) (SUM(?s) AS ?t) WHERE { ?i :size ?s . ?i :id ?code . FILTER(STRLEN(?code) > 0) } GROUP BY ?i } FILTER(STRLEN(?code) > 1) }";
            let plan = explained(query);
            assert_eq!(plan, explained(query));
            assert_eq!(
                plan,
                concat!(
                    "scan ?i <urn:t#name> ?code\n",
                    "filter (STRLEN(?code) > \"1\"^^<http://www.w3.org/2001/XMLSchema#integer>)\n",
                    "group by (?i)\n",
                    "  ?gen0 := count(*)\n",
                    "  ?gen1 := sum(?s)\n",
                    "  scan ?i <urn:t#size> ?s\n",
                    "  scan ?i <urn:t#id> ?code~5\n",
                    "  filter (STRLEN(?code~5) > \"0\"^^<http://www.w3.org/2001/XMLSchema#integer>)\n",
                    "bind ?c := ?gen0\n",
                    "bind ?t := ?gen1\n",
                    "construct\n",
                    "  ?i <urn:t#seen> ?c\n",
                )
            );
        }

        #[test]
        fn prints_a_subquery_run_once_and_one_that_reads_its_key_from_the_rows_apart() {
            let once = explained(ONCE[0].1);
            assert!(
                once.contains("group by (?i ?m), remembered as memo 0\n"),
                "{once}"
            );
            let seeded = explained(
                "CONSTRUCT { ?i :count ?c } WHERE { ?i a :Item . OPTIONAL { SELECT ?i (COUNT(?y) AS ?c) WHERE { ?i rdf:_1 ?y } GROUP BY ?i } }",
            );
            assert!(seeded.contains("group by (?i)\n"), "{seeded}");
        }
    }
}

/// Workings no caller sees · crates/bridge/src
mod workings_no_caller_sees {
    use super::*;

    /// `BUILTINS`
    mod builtins {
        use super::*;

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
    }
}
