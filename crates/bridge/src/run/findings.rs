// Every adapter here has its accounting struck out: what a findings query says is what
// this file is about.

use crate::fixtures::{address, annotations, one, tiny, unaccounted, Variant, OA, RDF_VALUE, SH};
use crate::load::load_adapter;
use crate::run::{convert, prepare, Conversion, Source};
use crate::Resolver;
use oxrdf::{NamedNode, Quad, Term};
use std::collections::BTreeSet;

const NOTE_QUERY: &str = "mapping/item-note-findings.rq";

fn findings_for(input: &str) -> Vec<Quad> {
    findings_through(&unaccounted(), input)
}

fn findings_through(resolver: &dyn Resolver, input: &str) -> Vec<Quad> {
    conversion(resolver, input).expect("conversion").findings
}

fn conversion(resolver: &dyn Resolver, input: &str) -> crate::Result<Conversion> {
    let adapter = load_adapter(resolver).expect("adapter");
    let prepared = prepare(&adapter, resolver).expect("prepared");
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    let xml = resolver.read(&iri).expect("input");
    convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            xml: &xml,
        },
    )
}

/// The tiny adapter, its accounting struck, with its note query rewritten.
fn rewritten(replacements: &[(&str, &str)]) -> Variant {
    replacements
        .iter()
        .fold(unaccounted(), |variant, (from, to)| {
            variant.replacing(NOTE_QUERY, from, *to)
        })
}

/// Every object of a predicate as N-Triples writes it: two blank nodes are one node
/// exactly where they write the same.
fn every(quads: &[Quad], predicate: &str) -> Vec<String> {
    let mut written: Vec<String> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == predicate)
        .map(|q| q.object.to_string())
        .collect();
    written.sort();
    written
}

/// Each finding's record selector and the step it is refined onto.
fn addresses(findings: &[Quad]) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = annotations(findings)
        .iter()
        .map(|annotation| address(findings, annotation))
        .collect();
    rows.sort();
    rows
}

fn row(record: &str, within: &str) -> (String, String) {
    (record.to_owned(), within.to_owned())
}

fn document(input: &str) -> Term {
    NamedNode::new(format!("{}fixtures/in/{input}", tiny().root()))
        .expect("an IRI")
        .into()
}

fn sources(findings: &[Quad]) -> Vec<Term> {
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasSource"))
        .map(|q| q.object.clone())
        .collect()
}

#[test]
fn names_the_document_the_record_was_read_from_where_the_query_named_this_record() {
    let findings = findings_for("two.xml");
    assert_eq!(sources(&findings), vec![document("two.xml"); 2]);
}

#[test]
fn moves_the_query_s_selector_under_the_record_s_own_position() {
    let findings = findings_for("two.xml");
    assert_eq!(
        addresses(&findings),
        [
            row("/catalog/item[1]", "note[1]"),
            row("/catalog/item[2]", "")
        ],
        "one finding of two names a node inside its record, and it is the first record's note"
    );
}

#[test]
fn gives_every_annotation_a_record_selector_of_its_own() {
    let findings = findings_for("order.xml");
    let selectors = every(&findings, &format!("{OA}hasSelector"));
    assert_eq!(annotations(&findings).len(), 4);
    assert_eq!(every(&findings, &format!("{OA}hasTarget")).len(), 4);
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        4,
        "two annotations share a selector node: {selectors:?}"
    );
}

#[test]
fn gives_each_note_of_a_record_a_finding_at_that_note_s_own_address() {
    let notes: Vec<(String, String)> = addresses(&findings_for("order.xml"))
        .into_iter()
        .filter(|(_, within)| !within.is_empty())
        .collect();
    assert_eq!(
        notes,
        [
            row("/catalog/item[1]", "note[1]"),
            row("/catalog/item[1]", "note[2]"),
            row("/catalog/item[2]", "note[1]")
        ],
        "two of one record's notes and one of the other's, each at its own place"
    );
}

const SELECTOR: &str = " ;\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value ?at ]";

/// The note query rewritten to name no node inside the record.
fn whole_record() -> Variant {
    rewritten(&[(SELECTOR, "")])
}

#[test]
fn selects_the_record_itself_for_a_query_that_writes_no_selector() {
    let findings = findings_through(&whole_record(), "two.xml");
    assert_eq!(
        addresses(&findings),
        [row("/catalog/item[1]", ""), row("/catalog/item[2]", "")]
    );
    assert!(every(&findings, &format!("{OA}refinedBy")).is_empty());
}

const TARGET: &str = "[\n      oa:hasSource bridge:thisRecord ;\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value ?at ]\n    ]";

#[test]
fn refuses_a_findings_query_whose_target_is_a_name() {
    let Err(refusal) = conversion(&rewritten(&[(TARGET, "bridge:thisRecord")]), "two.xml") else {
        panic!("the named form is accepted");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("item-note-findings.rq"), "{refusal}");
    assert!(refusal.contains("oa:hasTarget"), "{refusal}");
    assert!(
        refusal.contains(THIS_RECORD),
        "the refusal names the term the query wrote, not the document it became: {refusal}"
    );
}

#[test]
fn gives_an_annotation_targeting_the_record_itself_a_record_selector_of_its_own() {
    let findings = findings_through(&whole_record(), "order.xml");
    assert_eq!(annotations(&findings).len(), 4);

    let targets = every(&findings, &format!("{OA}hasTarget"));
    assert_eq!(
        targets.iter().collect::<BTreeSet<_>>().len(),
        4,
        "annotations share a target node: {targets:?}"
    );

    let selectors = every(&findings, &format!("{OA}hasSelector"));
    assert_eq!(selectors.len(), 4);
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        4,
        "annotations share a record selector: {selectors:?}"
    );

    assert_eq!(sources(&findings), vec![document("order.xml"); 4]);

    assert_eq!(
        addresses(&findings),
        [
            row("/catalog/item[1]", ""),
            row("/catalog/item[1]", ""),
            row("/catalog/item[1]", ""),
            row("/catalog/item[2]", "")
        ]
    );
}

const SOURCELESS: &str =
    "[\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value \"note\" ]\n    ]";

#[test]
fn refuses_a_findings_query_whose_target_names_no_document() {
    let Err(refusal) = conversion(&rewritten(&[(TARGET, SOURCELESS)]), "two.xml") else {
        panic!("a finding about no document is accepted");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("item-note-findings.rq"), "{refusal}");
    assert!(refusal.contains("oa:hasSource"), "{refusal}");
}

#[test]
fn refuses_a_findings_query_whose_annotation_has_no_target() {
    let targeting = format!("oa:hasTarget {TARGET} ;\n    ");
    let Err(refusal) = conversion(&rewritten(&[(&targeting, "")]), "two.xml") else {
        panic!("an annotation about no document is accepted");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("item-note-findings.rq"), "{refusal}");
    assert!(refusal.contains("oa:hasTarget"), "{refusal}");
}

const NOTE: &str = "urn:example:note";

/// The selector inside the target, named from the annotation as well.
const ALSO_NAMED: &str = "[
      oa:hasSource bridge:thisRecord ;
      oa:hasSelector _:sel
    ] ;
    <urn:example:note> _:sel";

#[test]
fn keeps_the_description_of_a_node_in_a_target_the_query_names_from_outside_it() {
    let findings = findings_through(
        &rewritten(&[
            (TARGET, ALSO_NAMED),
            (
                "sh:resultSeverity sh:Info .\n}",
                "sh:resultSeverity sh:Info .\n\n  _:sel a oa:XPathSelector ; rdf:value \"note\" .\n}",
            ),
        ]),
        "two.xml",
    );

    let named = every(&findings, NOTE);
    assert_eq!(named.len(), 1);
    let described: Vec<String> = findings
        .iter()
        .filter(|q| named.contains(&q.subject.to_string()))
        .map(|q| q.predicate.as_str().to_owned())
        .collect();
    assert!(
        described.contains(&RDF_VALUE.to_owned()),
        "the graph names a node with nothing on it: {described:?}"
    );
}

/// The note query rewritten to construct two annotations about the one target
/// node, which it mints once per solution.
const BOTH: &str = "_:note ;
    oa:hasBody ex:noteHasNoTerm ;
    oa:motivatedBy oa:classifying ;
    sh:resultSeverity sh:Info .

  [] a oa:Annotation ;
    oa:hasTarget _:note ;
    oa:hasBody ex:noteIsNotATitle ;
    oa:motivatedBy oa:classifying ;
    sh:resultSeverity sh:Info .

  _:note
    oa:hasSource bridge:thisRecord ;
    oa:hasSelector [ a oa:XPathSelector ; rdf:value \"note\" ] .";

#[test]
fn gives_two_annotations_the_query_pointed_at_one_target_a_record_selector_each() {
    let one_annotation = format!(
        "{TARGET} ;\n    oa:hasBody ex:noteHasNoTerm ;\n    oa:motivatedBy oa:classifying ;\n    sh:resultSeverity sh:Info ."
    );
    let findings = findings_through(&rewritten(&[(&one_annotation, BOTH)]), "two.xml");
    assert_eq!(
        annotations(&findings).len(),
        3,
        "two about the one note, one about the record with no title"
    );

    let targets = every(&findings, &format!("{OA}hasTarget"));
    assert_eq!(
        targets.iter().collect::<BTreeSet<_>>().len(),
        3,
        "annotations share a target node: {targets:?}"
    );

    let selectors = every(&findings, &format!("{OA}hasSelector"));
    assert_eq!(selectors.len(), 3);
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        3,
        "annotations share a record selector: {selectors:?}"
    );

    assert_eq!(
        addresses(&findings),
        [
            row("/catalog/item[1]", "note"),
            row("/catalog/item[1]", "note"),
            row("/catalog/item[2]", "")
        ]
    );

    assert_eq!(sources(&findings), vec![document("two.xml"); 3]);
}

const THIS_RECORD: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#thisRecord";

#[test]
fn refuses_a_findings_query_that_names_the_annotation_it_constructs() {
    let named = rewritten(&[("[] a oa:Annotation", "bridge:thisRecord a oa:Annotation")]);
    let Err(refusal) = conversion(&named, "two.xml") else {
        panic!("the named form is accepted");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("item-note-findings.rq"), "{refusal}");
    assert!(refusal.contains("oa:Annotation"), "{refusal}");
    assert!(
        refusal.contains(THIS_RECORD),
        "the refusal names the term the query wrote: {refusal}"
    );
}

/// The gap scheme and the note query rewritten together.
fn severities(scheme: &[(&str, &str)], query: &[(&str, &str)]) -> Variant {
    scheme.iter().fold(rewritten(query), |variant, (from, to)| {
        variant.replacing("vocab/catalog-gaps.ttl", from, *to)
    })
}

fn severity_of(findings: &[Quad], body: &str) -> Term {
    let named: Vec<String> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == body))
        .map(|q| q.subject.to_string())
        .collect();
    assert_eq!(named.len(), 1, "one annotation bodies {body}");
    one(findings, &named[0], &format!("{SH}resultSeverity")).expect("a severity")
}

fn severity_of_the_unbodied(findings: &[Quad]) -> Term {
    let unbodied: Vec<String> = annotations(findings)
        .into_iter()
        .filter(|annotation| one(findings, annotation, &format!("{OA}hasBody")).is_none())
        .collect();
    assert_eq!(unbodied.len(), 1, "one annotation carries no body");
    one(findings, &unbodied[0], &format!("{SH}resultSeverity")).expect("a severity")
}

fn severity(name: &str) -> Term {
    NamedNode::new(format!("{SH}{name}"))
        .expect("an IRI")
        .into()
}

const NOTE_HAS_NO_TERM: &str = "urn:example:catalog#noteHasNoTerm";
const WARNING_ON_THE_CONCEPT: (&str, &str) = (
    "ex:noteHasNoTerm a skos:Concept ;",
    "ex:noteHasNoTerm <http://www.w3.org/ns/shacl#resultSeverity> \
     <http://www.w3.org/ns/shacl#Warning> .\n\nex:noteHasNoTerm a skos:Concept ;",
);
const NO_SEVERITY_IN_THE_TEMPLATE: (&str, &str) = (" ;\n    sh:resultSeverity sh:Info", "");

#[test]
fn takes_the_concept_s_severity_where_the_query_s_template_wrote_none() {
    let findings = findings_through(
        &severities(&[WARNING_ON_THE_CONCEPT], &[NO_SEVERITY_IN_THE_TEMPLATE]),
        "two.xml",
    );
    assert_eq!(
        severity_of(&findings, NOTE_HAS_NO_TERM),
        severity("Warning")
    );
}

#[test]
fn takes_sh_info_where_neither_the_query_s_template_nor_the_concept_says() {
    let findings = findings_through(&severities(&[], &[NO_SEVERITY_IN_THE_TEMPLATE]), "two.xml");
    assert_eq!(severity_of(&findings, NOTE_HAS_NO_TERM), severity("Info"));
}

#[test]
fn takes_sh_info_for_a_body_the_gap_scheme_does_not_declare() {
    let findings = findings_through(
        &severities(
            &[WARNING_ON_THE_CONCEPT],
            &[
                NO_SEVERITY_IN_THE_TEMPLATE,
                (
                    "oa:hasBody ex:noteHasNoTerm",
                    "oa:hasBody ex:noteIsNotATitle",
                ),
            ],
        ),
        "two.xml",
    );
    assert_eq!(
        severity_of(&findings, "urn:example:catalog#noteIsNotATitle"),
        severity("Info")
    );
}

#[test]
fn takes_sh_info_for_an_annotation_the_query_gave_no_body() {
    let findings = findings_through(
        &severities(
            &[WARNING_ON_THE_CONCEPT],
            &[
                NO_SEVERITY_IN_THE_TEMPLATE,
                ("    oa:hasBody ex:noteHasNoTerm ;\n", ""),
            ],
        ),
        "two.xml",
    );
    assert_eq!(severity_of_the_unbodied(&findings), severity("Info"));
}

#[test]
fn keeps_the_severity_the_query_s_template_wrote_over_the_concept_s() {
    let findings = findings_through(&severities(&[WARNING_ON_THE_CONCEPT], &[]), "two.xml");
    assert_eq!(severity_of(&findings, NOTE_HAS_NO_TERM), severity("Info"));
}

#[test]
fn takes_the_concept_s_severity_whichever_way_round_a_template_wrote_two_bodies() {
    for bodies in [
        "oa:hasBody ex:noteIsNotATitle, ex:noteHasNoTerm ;",
        "oa:hasBody ex:noteHasNoTerm, ex:noteIsNotATitle ;",
    ] {
        let findings = findings_through(
            &severities(
                &[WARNING_ON_THE_CONCEPT],
                &[
                    NO_SEVERITY_IN_THE_TEMPLATE,
                    ("oa:hasBody ex:noteHasNoTerm ;", bodies),
                ],
            ),
            "two.xml",
        );
        assert_eq!(
            severity_of(&findings, NOTE_HAS_NO_TERM),
            severity("Warning"),
            "{bodies}"
        );
    }
}
