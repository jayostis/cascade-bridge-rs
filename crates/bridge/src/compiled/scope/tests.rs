use super::analysis::{Binding, Filter};
use super::*;
use crate::compiled::compile;
use spargebra::algebra::GraphPattern;
use spargebra::{Query, SparqlParser};

const PREFIXES: &str =
    "PREFIX : <urn:t#> PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> ";

/// Analyzes the patterns, without the projection the parser puts around them, as a group.
fn with_scope<T>(body: &str, check: impl FnOnce(&Scoped, &Names, &Frame) -> T) -> T {
    let text = format!("{PREFIXES} ASK {{ {body} }}");
    let query = SparqlParser::new().parse_query(&text).expect("a query");
    let Query::Ask { pattern, .. } = &query else {
        panic!("an ASK");
    };
    let pattern = match pattern {
        GraphPattern::Project { inner, .. } => &**inner,
        other => other,
    };
    let mut names = Names::default();
    let mut frame = Frame::default();
    let scoped = analyze(&mut names, pattern, &mut frame);
    check(&scoped, &names, &frame)
}

/// The variables of the outermost group in a set, by name.
fn named<Role>(names: &Names, slots: &Slots<Role>) -> Vec<String> {
    let mut named: Vec<String> = names.scopes[0]
        .iter()
        .filter(|(_, slot)| slots.contains(**slot))
        .map(|(name, _)| name.clone())
        .collect();
    named.sort();
    named
}

fn strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn parts<'a, 'q>(scoped: &'a Scoped<'q>, into: &mut Vec<&'a Scoped<'q>>) {
    into.push(scoped);
    match scoped {
        Scoped::Declined(_) | Scoped::Triples(_) | Scoped::Values(_) | Scoped::Path(_) => {}
        Scoped::Join(left, right) => {
            parts(left, into);
            parts(right, into);
        }
        Scoped::Filter(filter) => parts(&filter.inner, into),
        Scoped::Extend(extend) => parts(&extend.inner, into),
        Scoped::Union(union) => {
            for branch in &union.branches {
                parts(&branch.scoped, into);
            }
        }
        Scoped::Optional(optional) => {
            parts(&optional.left, into);
            parts(&optional.right, into);
        }
        Scoped::Subquery(subquery) => parts(&subquery.inner, into),
        Scoped::Group(group) => parts(&group.inner, into),
    }
}

fn extends<'a, 'q>(scoped: &'a Scoped<'q>) -> Vec<&'a Binding<'q>> {
    let mut all = Vec::new();
    parts(scoped, &mut all);
    all.into_iter()
        .filter_map(|part| match part {
            Scoped::Extend(extend) => Some(&**extend),
            _ => None,
        })
        .collect()
}

fn filters<'a, 'q>(scoped: &'a Scoped<'q>) -> Vec<&'a Filter<'q>> {
    let mut all = Vec::new();
    parts(scoped, &mut all);
    all.into_iter()
        .filter_map(|part| match part {
            Scoped::Filter(filter) => Some(&**filter),
            _ => None,
        })
        .collect()
}

/// (construct, patterns, bound in every row, bound in some row)
const BOUND: &[(&str, &str, &[&str], &[&str])] = &[
    (
        "a UNION is certain of what every branch binds and maybe of what any does",
        "{ ?i :name ?n } UNION { ?i :id ?m }",
        &["i"],
        &["i", "m", "n"],
    ),
    (
        "an OPTIONAL's right side binds in some rows only",
        "?i a :Item . OPTIONAL { ?i :name ?n }",
        &["i"],
        &["i", "n"],
    ),
    (
        "a BIND of an expression binds in some rows, of a constant in every row",
        "?i a :Item . BIND(STR(?i) AS ?s) BIND(:flag AS ?f)",
        &["f", "i"],
        &["f", "i", "s"],
    ),
    (
        "a VALUES column with a gap binds in some rows",
        "VALUES (?a ?b) { (1 UNDEF) (2 3) }",
        &["a"],
        &["a", "b"],
    ),
    (
        "a subquery shares only the variables it projects",
        "{ SELECT ?i WHERE { ?i :a ?x } }",
        &["i"],
        &["i"],
    ),
    (
        "a SELECT DISTINCT binds its keys as its rows do",
        "{ SELECT DISTINCT ?i ?x WHERE { ?i :a ?x } }",
        &["i", "x"],
        &["i", "x"],
    ),
    (
        "a group binds its keys in every row and its aggregates in some",
        "{ SELECT ?i (COUNT(?x) AS ?c) WHERE { ?i :a ?x } GROUP BY ?i }",
        &["i"],
        &["c", "i"],
    ),
    (
        "a walk binds the node it ends at",
        ":p (!rdf:type)* ?m",
        &["m"],
        &["m"],
    ),
    (
        "a nested group binds what its patterns bind",
        "?i a :Item . { ?i :name ?n }",
        &["i", "n"],
        &["i", "n"],
    ),
];

#[test]
fn works_out_what_a_pattern_binds_in_every_row_and_in_some_row() {
    let wrong: Vec<_> = BOUND
        .iter()
        .filter_map(|&(construct, patterns, certain, maybe)| {
            let (found_certain, found_maybe) = with_scope(patterns, |_, names, frame| {
                (
                    named(names, &frame.bound.certain),
                    named(names, &frame.bound.maybe),
                )
            });
            (found_certain != strings(certain) || found_maybe != strings(maybe)).then_some((
                construct,
                found_certain,
                found_maybe,
            ))
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn declines_nothing_where_a_group_reads_what_it_binds_or_what_only_the_groups_around_it_bind_and_it_does_not(
) {
    for patterns in [
        "?i :id ?v . { ?x :key ?k FILTER(?v = \"1\") }",
        "?i a :Item . { ?x :key ?k FILTER NOT EXISTS { ?i :name \"alpha\" } }",
        "?i :id ?v . { ?i :id ?v FILTER(?v = \"1\") }",
        "?i a :Item . OPTIONAL { ?i :name ?n FILTER(?n != \"x\") }",
    ] {
        let text = format!("{PREFIXES} CONSTRUCT {{ ?i :p ?i }} WHERE {{ {patterns} }}");
        let query = SparqlParser::new().parse_query(&text).expect("a query");
        assert_eq!(compile(&query).err(), None, "{patterns}");
    }
}

/// (construct, patterns, what the first filter reads as unbound)
const HIDDEN: &[(&str, &str, &[&str])] = &[
    (
        "a NOT EXISTS over a variable only the groups around it bind",
        "?i a :Item . { ?x :key ?k FILTER NOT EXISTS { ?i :name \"alpha\" } }",
        &["i"],
    ),
    (
        "a comparison with a variable only the groups around it bind",
        "?i :id ?v . { ?x :key ?k FILTER(?v = \"1\") }",
        &["v"],
    ),
    (
        "a variable the group binds itself, though a group around it binds it too",
        "?i :id ?v . { ?i :id ?v FILTER(?v = \"1\") }",
        &[],
    ),
    (
        "a variable of the group's own patterns",
        "?i :id ?v FILTER(?v = \"1\")",
        &[],
    ),
];

#[test]
fn works_out_what_a_filter_reads_as_unbound() {
    let wrong: Vec<_> = HIDDEN
        .iter()
        .filter_map(|&(construct, patterns, hidden)| {
            let found = with_scope(patterns, |scoped, names, _| {
                let filter = *filters(scoped).first().expect("a filter");
                let conjunct = filter.conjuncts.first().expect("a conjunct");
                named(names, conjunct.hidden.as_ref().expect("not declined"))
            });
            (found != strings(hidden)).then_some((construct, found))
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

type Bind = (bool, bool);

/// (construct, patterns, for each BIND: a join on a variable a group around binds, and
/// bound in every row)
const BINDS: &[(&str, &str, &[Bind])] = &[
    (
        "a BIND of a variable a group around it binds",
        "?i :id ?n . { ?i :id ?id BIND(?id AS ?n) }",
        &[(true, true)],
    ),
    (
        "a BIND of an expression of what the group binds",
        "?i :id ?id BIND(CONCAT(?id, \"\") AS ?k)",
        &[(false, false)],
    ),
    (
        "a BIND of a constant",
        "?i a :Item BIND(:flag AS ?f)",
        &[(false, true)],
    ),
    (
        "an aggregate named for a variable the group around it binds, which spargebra binds after the group",
        "?i a :Item . BIND(2 AS ?c) { SELECT ?i (COUNT(?x) AS ?c) WHERE { ?i ?p ?x } GROUP BY ?i }",
        &[(false, true), (true, true)],
    ),
    (
        "a BIND of a constant the group around it binds too, in each branch",
        "?i a :Item BIND(\"p\" AS ?c) { { ?i :name ?n BIND(\"p\" AS ?c) } UNION { ?i :id ?n BIND(\"q\" AS ?c) } }",
        &[(false, true), (true, true), (true, true)],
    ),
];

#[test]
fn works_out_where_a_bind_joins_and_where_it_binds_in_every_row() {
    let wrong: Vec<_> = BINDS
        .iter()
        .filter_map(|&(construct, patterns, expected)| {
            let found = with_scope(patterns, |scoped, _, _| {
                extends(scoped)
                    .iter()
                    .map(|extend| (extend.joins, extend.certainly))
                    .collect::<Vec<_>>()
            });
            (found != expected).then_some((construct, found))
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn starts_an_exists_body_with_the_row_it_runs_on_less_what_is_hidden() {
    let slots = <[usize]>::to_vec;
    let frame = Frame::within_exists(&Scope {
        certain: [0, 1].into_iter().collect(),
        maybe: [0, 1, 2].into_iter().collect(),
        hidden: [1].into_iter().collect(),
    });
    assert_eq!(frame.bound.certain.iter().collect::<Vec<_>>(), slots(&[0]));
    assert_eq!(frame.bound.maybe.iter().collect::<Vec<_>>(), slots(&[0, 2]));
    assert_eq!(frame.row.iter().collect::<Vec<_>>(), slots(&[0, 2]));
    assert!(frame.bound.own.iter().next().is_none());
}
