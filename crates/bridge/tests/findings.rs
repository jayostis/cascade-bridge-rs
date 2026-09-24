// A findings query is a CONSTRUCT whose annotations name the record by
// bridge:thisRecord. The Bridge puts the document in that name's place and
// moves the query's selector under the record's own position, so an adapter
// says what inside a record a finding is about and the Bridge says which
// record that was.
mod common;

use cascade_bridge::{Conversion, DirectoryResolver, Resolver};
use common::Subject;
use oxrdf::{Quad, Term};
use std::collections::BTreeSet;
use std::path::PathBuf;

const OA: &str = "http://www.w3.org/ns/oa#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";

const SOURCE_ACCOUNTING: &str = "bridge:sourceAccounting";

/// The tiny adapter with its accounting struck out of its crate. A census
/// finding and the finding an entry reports are neither of them a query's,
/// and what a findings query says is what this file is about.
struct Queries {
    directory: DirectoryResolver,
}

impl Resolver for Queries {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("ro-crate-metadata.json") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        let kept: Vec<&str> = text
            .lines()
            .filter(|line| !line.contains(SOURCE_ACCOUNTING))
            .collect();
        assert_eq!(
            text.lines().count() - kept.len(),
            2,
            "the crate names an accounting, in its context and on its root entity"
        );
        Ok(kept.join("\n").into_bytes())
    }
}

fn tiny() -> Queries {
    Queries {
        directory: DirectoryResolver::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"),
        )
        .expect("resolver"),
    }
}

thread_local! {
    static QUERIES: Subject<Queries> = Subject::of(tiny());
}

/// The tiny adapter's findings for one of its committed inputs.
fn findings_for(input: &str) -> Vec<Quad> {
    QUERIES.with(|queries| queries.findings(input))
}

fn findings_through(resolver: &dyn Resolver, input: &str) -> Vec<Quad> {
    conversion(resolver, input).expect("conversion").findings
}

fn conversion(resolver: &dyn Resolver, input: &str) -> cascade_bridge::Result<Conversion> {
    let prepared = common::prepared(resolver).expect("prepared");
    common::convert_input(&prepared, resolver, input)
}

/// Every object of a predicate, written as N-Triples writes it.
fn objects(quads: &[Quad], predicate: &str) -> Vec<String> {
    let mut written: Vec<String> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == predicate)
        .map(|q| q.object.to_string())
        .collect();
    written.sort();
    written
}

/// The XPath every selector node holds, sorted.
fn selector_values(quads: &[Quad]) -> Vec<String> {
    let selector = format!("{OA}XPathSelector");
    let selectors: Vec<String> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == selector))
        .map(|q| q.subject.to_string())
        .collect();
    let mut values: Vec<String> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_VALUE)
        .filter(|q| selectors.contains(&q.subject.to_string()))
        .map(|q| q.object.to_string())
        .collect();
    values.sort();
    values
}

fn annotations(quads: &[Quad]) -> usize {
    let annotation = format!("{OA}Annotation");
    quads
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == annotation))
        .count()
}

#[test]
fn names_the_document_the_record_was_read_from_where_the_query_named_this_record() {
    let findings = findings_for("two.xml");
    let sources = objects(&findings, &format!("{OA}hasSource"));
    assert_eq!(sources.len(), 2);
    for source in sources {
        assert!(source.ends_with("fixtures/in/two.xml>"), "{source}");
    }
}

#[test]
fn moves_the_query_s_selector_under_the_record_s_own_position() {
    let findings = findings_for("two.xml");
    assert_eq!(
        objects(&findings, &format!("{OA}refinedBy")).len(),
        1,
        "one finding of two names a node inside its record"
    );

    assert_eq!(
        selector_values(&findings),
        [
            "\"/catalog/item[1]\"",
            "\"/catalog/item[2]\"",
            "\"note[1]\""
        ]
    );
}

#[test]
fn gives_every_annotation_a_record_selector_of_its_own() {
    let findings = findings_for("order.xml");
    let selectors = objects(&findings, &format!("{OA}hasSelector"));
    assert_eq!(annotations(&findings), 4);
    assert_eq!(objects(&findings, &format!("{OA}hasTarget")).len(), 4);
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        4,
        "two annotations share a selector node: {selectors:?}"
    );
}

#[test]
fn gives_each_note_of_a_record_a_finding_at_that_note_s_own_address() {
    let notes: Vec<String> = selector_values(&findings_for("order.xml"))
        .into_iter()
        .filter(|value| value.starts_with("\"note"))
        .collect();
    assert_eq!(
        notes,
        ["\"note[1]\"", "\"note[1]\"", "\"note[2]\""],
        "two of one record's notes and one of the other's, each at its own place"
    );
}

/// The tiny adapter with the findings query that names a node inside the
/// record rewritten to name none.
struct WholeRecord {
    directory: Queries,
}

const SELECTOR: &str = " ;\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value ?at ]";

impl Resolver for WholeRecord {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("mapping/item-note-findings.rq") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(text.contains(SELECTOR), "the query names a selector");
        Ok(text.replace(SELECTOR, "").into_bytes())
    }
}

#[test]
fn selects_the_record_itself_for_a_query_that_writes_no_selector() {
    let findings = findings_through(&WholeRecord { directory: tiny() }, "two.xml");
    assert_eq!(annotations(&findings), 2);
    assert!(objects(&findings, &format!("{OA}refinedBy")).is_empty());
    assert_eq!(objects(&findings, &format!("{OA}hasSelector")).len(), 2);
}

const TARGET: &str = "[\n      oa:hasSource bridge:thisRecord ;\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value ?at ]\n    ]";

/// The tiny adapter with the findings query that builds a target of its own
/// rewritten to name the record itself, which the specification forbids: one
/// name is one node for every finding the query produces.
struct NamedTarget {
    directory: Queries,
}

impl Resolver for NamedTarget {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("mapping/item-note-findings.rq") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(
            text.contains(TARGET),
            "the query builds a target of its own"
        );
        Ok(text.replace(TARGET, "bridge:thisRecord").into_bytes())
    }
}

#[test]
fn refuses_a_findings_query_whose_target_is_a_name() {
    let Err(refusal) = conversion(&NamedTarget { directory: tiny() }, "two.xml") else {
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
    let findings = findings_through(&WholeRecord { directory: tiny() }, "order.xml");
    assert_eq!(annotations(&findings), 4);

    let targets = objects(&findings, &format!("{OA}hasTarget"));
    assert_eq!(
        targets.iter().collect::<BTreeSet<_>>().len(),
        4,
        "annotations share a target node: {targets:?}"
    );

    let selectors = objects(&findings, &format!("{OA}hasSelector"));
    assert_eq!(selectors.len(), 4);
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        4,
        "annotations share a record selector: {selectors:?}"
    );

    let sources = objects(&findings, &format!("{OA}hasSource"));
    assert_eq!(sources.len(), 4);
    for source in sources {
        assert!(source.ends_with("fixtures/in/order.xml>"), "{source}");
    }

    assert_eq!(
        selector_values(&findings),
        [
            "\"/catalog/item[1]\"",
            "\"/catalog/item[1]\"",
            "\"/catalog/item[1]\"",
            "\"/catalog/item[2]\""
        ]
    );
}

/// The tiny adapter with its findings query rewritten, each replacement
/// asserted to have something to replace so a query that moved on cannot leave
/// a test passing on the query it no longer has.
struct Rewritten {
    directory: Queries,
    replacements: Vec<(String, String)>,
}

impl Rewritten {
    fn new(replacements: &[(&str, &str)]) -> Self {
        Self {
            directory: tiny(),
            replacements: replacements
                .iter()
                .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
                .collect(),
        }
    }
}

impl Resolver for Rewritten {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("mapping/item-note-findings.rq") {
            return Ok(bytes);
        }
        let mut text = String::from_utf8(bytes).expect("utf-8");
        for (from, to) in &self.replacements {
            assert!(text.contains(from), "the query holds {from:?}");
            text = text.replace(from, to);
        }
        Ok(text.into_bytes())
    }
}

const SOURCELESS: &str =
    "[\n      oa:hasSelector [ a oa:XPathSelector ; rdf:value \"note\" ]\n    ]";

#[test]
fn refuses_a_findings_query_whose_target_names_no_document() {
    let Err(refusal) = conversion(&Rewritten::new(&[(TARGET, SOURCELESS)]), "two.xml") else {
        panic!("a finding about no document is accepted");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("item-note-findings.rq"), "{refusal}");
    assert!(refusal.contains("oa:hasSource"), "{refusal}");
}

#[test]
fn refuses_a_findings_query_whose_annotation_has_no_target() {
    let targeting = format!("oa:hasTarget {TARGET} ;\n    ");
    let Err(refusal) = conversion(&Rewritten::new(&[(&targeting, "")]), "two.xml") else {
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
        &Rewritten::new(&[
            (TARGET, ALSO_NAMED),
            (
                "sh:resultSeverity sh:Info .\n}",
                "sh:resultSeverity sh:Info .\n\n  _:sel a oa:XPathSelector ; rdf:value \"note\" .\n}",
            ),
        ]),
        "two.xml",
    );

    let named = objects(&findings, NOTE);
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

/// The tiny adapter with the findings query rewritten to construct two
/// annotations about the one target node, which it mints once per solution.
struct SharedTarget {
    directory: Queries,
}

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

impl Resolver for SharedTarget {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("mapping/item-note-findings.rq") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        let one = format!(
            "{TARGET} ;\n    oa:hasBody ex:noteHasNoTerm ;\n    oa:motivatedBy oa:classifying ;\n    sh:resultSeverity sh:Info ."
        );
        assert!(text.contains(&one), "the query constructs one annotation");
        Ok(text.replace(&one, BOTH).into_bytes())
    }
}

#[test]
fn gives_two_annotations_the_query_pointed_at_one_target_a_record_selector_each() {
    let findings = findings_through(&SharedTarget { directory: tiny() }, "two.xml");
    assert_eq!(
        annotations(&findings),
        3,
        "two about the one note, one about the record with no title"
    );

    let targets = objects(&findings, &format!("{OA}hasTarget"));
    assert_eq!(
        targets.iter().collect::<BTreeSet<_>>().len(),
        3,
        "annotations share a target node: {targets:?}"
    );

    let selectors = objects(&findings, &format!("{OA}hasSelector"));
    assert_eq!(selectors.len(), 3);
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        3,
        "annotations share a record selector: {selectors:?}"
    );

    assert_eq!(objects(&findings, &format!("{OA}refinedBy")).len(), 2);
    assert_eq!(
        selector_values(&findings),
        [
            "\"/catalog/item[1]\"",
            "\"/catalog/item[1]\"",
            "\"/catalog/item[2]\"",
            "\"note\"",
            "\"note\""
        ]
    );

    let sources = objects(&findings, &format!("{OA}hasSource"));
    assert_eq!(sources.len(), 3);
    for source in sources {
        assert!(source.ends_with("fixtures/in/two.xml>"), "{source}");
    }
}

const THIS_RECORD: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#thisRecord";

#[test]
fn refuses_a_findings_query_that_names_the_annotation_it_constructs() {
    let named = Rewritten::new(&[("[] a oa:Annotation", "bridge:thisRecord a oa:Annotation")]);
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

const SH: &str = "http://www.w3.org/ns/shacl#";

/// The tiny adapter with its gap scheme and its findings query rewritten
/// together: what a concept declares and what a template writes are the two
/// halves of a finding's severity, and a test of one sets the other.
struct Severities {
    directory: Queries,
    scheme: Vec<(String, String)>,
    query: Vec<(String, String)>,
}

impl Severities {
    fn new(scheme: &[(&str, &str)], query: &[(&str, &str)]) -> Self {
        let owned = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
                .collect()
        };
        Self {
            directory: tiny(),
            scheme: owned(scheme),
            query: owned(query),
        }
    }
}

impl Resolver for Severities {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        let replacements = if iri.ends_with("vocab/catalog-gaps.ttl") {
            &self.scheme
        } else if iri.ends_with("mapping/item-note-findings.rq") {
            &self.query
        } else {
            return Ok(bytes);
        };
        let mut text = String::from_utf8(bytes).expect("utf-8");
        for (from, to) in replacements {
            assert!(text.contains(from), "{iri} holds {from:?}");
            text = text.replace(from, to);
        }
        Ok(text.into_bytes())
    }
}

/// The severity of the annotation whose body is this one, of the two findings
/// two.xml draws: the note the query addresses, and the item with no title.
fn severity_of(findings: &[Quad], body: &str) -> String {
    let named = findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == body))
        .map(|q| q.subject.clone())
        .collect::<Vec<_>>();
    assert_eq!(named.len(), 1, "one annotation bodies {body}");
    let severities: Vec<String> = findings
        .iter()
        .filter(|q| q.subject == named[0])
        .filter(|q| q.predicate.as_str() == format!("{SH}resultSeverity"))
        .map(|q| q.object.to_string())
        .collect();
    assert_eq!(severities.len(), 1, "the finding carries one severity");
    severities[0].clone()
}

/// The severity of the one annotation the query gave no body.
fn severity_of_the_unbodied(findings: &[Quad]) -> String {
    let bodied: Vec<String> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
        .map(|q| q.subject.to_string())
        .collect();
    let annotation = format!("{OA}Annotation");
    let unbodied: Vec<_> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == annotation))
        .map(|q| q.subject.clone())
        .filter(|s| !bodied.contains(&s.to_string()))
        .collect();
    assert_eq!(unbodied.len(), 1, "one annotation carries no body");
    let severities: Vec<String> = findings
        .iter()
        .filter(|q| q.subject == unbodied[0])
        .filter(|q| q.predicate.as_str() == format!("{SH}resultSeverity"))
        .map(|q| q.object.to_string())
        .collect();
    assert_eq!(severities.len(), 1, "the finding carries one severity");
    severities[0].clone()
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
        &Severities::new(&[WARNING_ON_THE_CONCEPT], &[NO_SEVERITY_IN_THE_TEMPLATE]),
        "two.xml",
    );
    assert_eq!(
        severity_of(&findings, NOTE_HAS_NO_TERM),
        format!("<{SH}Warning>")
    );
}

#[test]
fn takes_sh_info_where_neither_the_query_s_template_nor_the_concept_says() {
    let findings = findings_through(
        &Severities::new(&[], &[NO_SEVERITY_IN_THE_TEMPLATE]),
        "two.xml",
    );
    assert_eq!(
        severity_of(&findings, NOTE_HAS_NO_TERM),
        format!("<{SH}Info>")
    );
}

#[test]
fn takes_sh_info_for_a_body_the_gap_scheme_does_not_declare() {
    let findings = findings_through(
        &Severities::new(
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
        format!("<{SH}Info>")
    );
}

#[test]
fn takes_sh_info_for_an_annotation_the_query_gave_no_body() {
    let findings = findings_through(
        &Severities::new(
            &[WARNING_ON_THE_CONCEPT],
            &[
                NO_SEVERITY_IN_THE_TEMPLATE,
                ("    oa:hasBody ex:noteHasNoTerm ;\n", ""),
            ],
        ),
        "two.xml",
    );
    assert_eq!(severity_of_the_unbodied(&findings), format!("<{SH}Info>"));
}

#[test]
fn keeps_the_severity_the_query_s_template_wrote_over_the_concept_s() {
    let findings = findings_through(&Severities::new(&[WARNING_ON_THE_CONCEPT], &[]), "two.xml");
    assert_eq!(
        severity_of(&findings, NOTE_HAS_NO_TERM),
        format!("<{SH}Info>")
    );
}

/// Two bodies on one annotation is not conforming output, the specification's
/// `<#SourceFinding>` taking exactly one: what is under test is that the
/// severity does not turn on which body a query happened to write first.
#[test]
fn takes_the_concept_s_severity_whichever_way_round_a_template_wrote_two_bodies() {
    for bodies in [
        "oa:hasBody ex:noteIsNotATitle, ex:noteHasNoTerm ;",
        "oa:hasBody ex:noteHasNoTerm, ex:noteIsNotATitle ;",
    ] {
        let findings = findings_through(
            &Severities::new(
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
            format!("<{SH}Warning>"),
            "{bodies}"
        );
    }
}
