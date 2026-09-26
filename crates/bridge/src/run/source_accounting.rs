use crate::fixtures::{
    address, annotations, conversion, count, findings, node, says, step, tiny, unaccounted,
    with_accounting, Variant, ACCOUNTING, BRIDGE, CRATE, OA, PATH_NOT_ACCOUNTED, RDF_TYPE, SH,
};
use crate::Resolver;
use oxrdf::{Quad, Term};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::BTreeSet;

const SOURCE_PATH: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#sourcePath";

fn censuses(findings: &[Quad]) -> Vec<String> {
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == PATH_NOT_ACCOUNTED))
        .map(|q| q.subject.to_string())
        .collect()
}

/// Each census finding as its path, its record, and the occurrence it is addressed at.
fn census(findings: &[Quad]) -> Vec<(String, String, String)> {
    let mut rows: Vec<(String, String, String)> = censuses(findings)
        .iter()
        .map(|annotation| {
            let (record, within) = address(findings, annotation);
            (
                says(findings, annotation, &format!("{SH}value")),
                record,
                within,
            )
        })
        .collect();
    rows.sort();
    rows
}

/// Each census finding as its path, its record, and the count it carries.
fn counted(findings: &[Quad]) -> Vec<(String, String, Option<String>)> {
    let mut rows: Vec<(String, String, Option<String>)> = censuses(findings)
        .iter()
        .map(|annotation| {
            (
                says(findings, annotation, &format!("{SH}value")),
                address(findings, annotation).0,
                count(findings, annotation),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// How many nodes below the record this finding's record selector is refined onto.
fn refinements(findings: &[Quad], annotation: &str) -> usize {
    let target = node(findings, annotation, &format!("{OA}hasTarget"));
    let selector = node(findings, &target, &format!("{OA}hasSelector"));
    findings
        .iter()
        .filter(|q| q.subject.to_string() == selector)
        .filter(|q| q.predicate.as_str() == format!("{OA}refinedBy"))
        .count()
}

/// The record's selector and the steps it is refined onto, as one path.
fn located(findings: &[Quad], annotation: &str) -> String {
    match address(findings, annotation) {
        (record, below) if below.is_empty() => record,
        (record, below) => format!("{record}/{below}"),
    }
}

fn violations(findings: &[Quad]) -> BTreeSet<String> {
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{SH}resultSeverity"))
        .filter(
            |q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == format!("{SH}Violation")),
        )
        .map(|q| located(findings, &q.subject.to_string()))
        .collect()
}

fn paths(findings: &[Quad]) -> BTreeSet<String> {
    census(findings)
        .into_iter()
        .map(|(path, _, _)| path)
        .collect()
}

/// Each finding as its body, severity, path, record, and the node below the record it
/// selects.
fn said(findings: &[Quad]) -> Vec<(String, String, String, String, String)> {
    let mut rows: Vec<(String, String, String, String, String)> = annotations(findings)
        .iter()
        .map(|annotation| {
            let (record, within) = address(findings, annotation);
            (
                says(findings, annotation, &format!("{OA}hasBody")),
                says(findings, annotation, &format!("{SH}resultSeverity")),
                says(findings, annotation, &format!("{SH}value")),
                record,
                within,
            )
        })
        .collect();
    rows.sort();
    rows
}

/// The paths an accounting's entries name, as often as each is named.
fn source_paths(iri: &str, turtle: &[u8]) -> Vec<String> {
    let mut named: Vec<String> = RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(iri)
        .expect("the accounting's IRI")
        .for_slice(turtle)
        .map(|quad| quad.expect("the accounting is Turtle"))
        .filter(|quad| quad.predicate.as_str() == SOURCE_PATH)
        .map(|quad| match quad.object {
            Term::Literal(literal) => literal.value().to_owned(),
            other => other.to_string(),
        })
        .collect();
    named.sort();
    named
}

/// The committed accounting with the entry for one path cut out, checked by reading it
/// back.
fn without(path: &str) -> Variant {
    let directory = tiny();
    let iri = format!("{}{ACCOUNTING}", directory.root());
    let committed = directory.read(&iri).expect("the accounting");
    let text = String::from_utf8(committed.clone()).expect("utf-8");
    let entry = format!("bridge:sourcePath \"{path}\"");
    let kept = text
        .split("\n\n")
        .filter(|block| !block.contains(&entry))
        .collect::<Vec<&str>>()
        .join("\n\n");
    let mut expected = source_paths(&iri, &committed);
    let at = expected
        .iter()
        .position(|named| named == path)
        .unwrap_or_else(|| panic!("the accounting holds an entry for {path}"));
    expected.remove(at);
    assert_eq!(
        source_paths(&iri, kept.as_bytes()),
        expected,
        "the cut took out the entry for {path} and nothing else"
    );
    Variant::of(directory).with(ACCOUNTING, kept)
}

const UNWRITTEN: &str = "vocab/an-accounting-nobody-wrote.ttl";

#[test]
fn leaves_a_crate_that_names_no_accounting_the_findings_it_has_today() {
    let unaccounted = unaccounted();
    for input in [
        "two.xml",
        "order.xml",
        "every-verdict.xml",
        "gap-three-times.xml",
    ] {
        let named_no_path: Vec<(String, String, String, String, String)> =
            said(&findings(&tiny(), input))
                .into_iter()
                .filter(|(_, _, path, _, _)| path.is_empty())
                .collect();
        assert_eq!(
            said(&findings(&unaccounted, input)),
            named_no_path,
            "{input} through a crate naming no accounting"
        );
    }
    assert_eq!(
        annotations(&findings(&unaccounted, "unaccounted-child.xml")).len(),
        0,
        "a path no entry accounts for draws nothing where there is no accounting"
    );
    assert_eq!(
        annotations(&findings(&unaccounted, "every-verdict.xml")).len(),
        1,
        "the record's one note draws the one finding the adapter's queries construct"
    );

    assert_eq!(
        paths(&findings(&tiny(), "unaccounted-child.xml"))
            .into_iter()
            .collect::<Vec<String>>(),
        ["/item/novelty"],
        "the accounting is what the census is read from"
    );
}

#[test]
fn reports_a_path_the_accounting_omits_at_its_first_occurrence_in_the_record() {
    let findings = findings(&tiny(), "unaccounted-child.xml");
    assert_eq!(
        census(&findings),
        [(
            "/item/novelty".to_owned(),
            "/catalog/item[1]".to_owned(),
            "novelty[1]".to_owned()
        )]
    );

    let annotation = censuses(&findings).first().expect("a census").clone();
    assert_eq!(
        says(&findings, &annotation, RDF_TYPE),
        format!("{OA}Annotation")
    );
    assert_eq!(
        says(&findings, &annotation, &format!("{OA}motivatedBy")),
        format!("{OA}classifying")
    );
    assert_eq!(
        says(&findings, &annotation, &format!("{SH}resultSeverity")),
        format!("{SH}Info"),
        "a path nothing accounts for is a backlog item, not a defect in the document"
    );
    assert_eq!(
        count(&findings, &annotation),
        None,
        "a count of one is what its absence says"
    );
    let target = node(&findings, &annotation, &format!("{OA}hasTarget"));
    assert!(
        says(&findings, &target, &format!("{OA}hasSource"))
            .ends_with("fixtures/in/unaccounted-child.xml"),
        "{findings:?}"
    );
}

#[test]
fn reports_a_path_a_record_carries_five_times_once_addressed_at_the_first() {
    let found = findings(&tiny(), "unaccounted-five-times.xml");
    assert_eq!(
        census(&found),
        [(
            "/item/novelty".to_owned(),
            "/catalog/item[1]".to_owned(),
            "novelty[1]".to_owned()
        )]
    );
    assert_eq!(
        counted(&found),
        [(
            "/item/novelty".to_owned(),
            "/catalog/item[1]".to_owned(),
            Some("5".to_owned())
        )],
        "a path newly appeared cannot otherwise be told from a thousand of them"
    );
}

#[test]
fn counts_only_its_own_record_s_nodes_where_two_records_carry_the_path() {
    assert_eq!(
        counted(&findings(&tiny(), "unaccounted-twice-then-once.xml")),
        [
            (
                "/item/novelty".to_owned(),
                "/catalog/item[1]".to_owned(),
                Some("2".to_owned())
            ),
            (
                "/item/novelty".to_owned(),
                "/catalog/item[2]".to_owned(),
                None
            )
        ]
    );
}

#[test]
fn reports_a_path_two_records_carry_once_in_each() {
    assert_eq!(
        census(&findings(&tiny(), "unaccounted-in-each-record.xml")),
        [
            (
                "/item/novelty".to_owned(),
                "/catalog/item[1]".to_owned(),
                "novelty[1]".to_owned()
            ),
            (
                "/item/novelty".to_owned(),
                "/catalog/item[2]".to_owned(),
                "novelty[1]".to_owned()
            )
        ]
    );
}

#[test]
fn ends_an_attribute_s_path_in_its_own_name_and_refines_onto_the_element_it_stands_on() {
    assert_eq!(
        census(&findings(&tiny(), "unaccounted-attribute.xml")),
        [(
            "/item/label/@colour".to_owned(),
            "/catalog/item[1]".to_owned(),
            "label[1]".to_owned()
        )],
        "an XPath selector has no form for an attribute, and the lift writes elements"
    );
}

#[test]
fn refines_onto_every_step_below_the_record_down_to_the_one_the_path_ends_at() {
    let found = findings(&tiny(), "unaccounted-under-a-repeated-parent.xml");
    assert_eq!(
        counted(&found),
        [
            (
                "/item/label/@colour".to_owned(),
                "/catalog/item[1]".to_owned(),
                Some("2".to_owned())
            ),
            (
                "/item/label/deep".to_owned(),
                "/catalog/item[1]".to_owned(),
                Some("2".to_owned())
            )
        ],
        "a colour on each label, and a deep under each of the second label's"
    );
    assert_eq!(
        census(&found),
        [
            (
                "/item/label/@colour".to_owned(),
                "/catalog/item[1]".to_owned(),
                "label[1]".to_owned()
            ),
            (
                "/item/label/deep".to_owned(),
                "/catalog/item[1]".to_owned(),
                "label[2]/deep[1]".to_owned()
            )
        ],
        "a label the record carries twice stands between the record and the path"
    );
}

#[test]
fn gives_an_element_the_one_position_in_a_schema_finding_and_in_a_census_finding() {
    let found = findings(&tiny(), "unaccounted-under-a-repeated-parent.xml");
    let label = "/catalog/item[1]/label[2]";
    assert_eq!(
        violations(&found).into_iter().collect::<Vec<String>>(),
        [label],
        "the label carrying the unaccounted element breaks the schema the record is read by"
    );

    let deep = censuses(&found)
        .into_iter()
        .find(|annotation| says(&found, annotation, &format!("{SH}value")) == "/item/label/deep")
        .expect("a census about the element that label carries");
    assert_eq!(
        located(&found, &deep),
        format!("{label}/deep[1]"),
        "a schema finding and a census finding count a record's elements in walks of their own"
    );
}

#[test]
fn writes_a_namespaced_path_as_the_lift_writes_a_step_of_a_record_s_own_address() {
    let record = format!("/{}/{}[1]", step("catalog"), step("item"));
    let bogus = format!("/{}/{}", step("item"), step("bogus"));
    let within = format!("{}[1]", step("bogus"));
    assert_eq!(
        census(&findings(&tiny(), "namespaced.xml")),
        [(bogus.clone(), record.clone(), within.clone())],
        "the accounting names both colours of the element, and neither is reported"
    );

    // The element carries a namespaced colour and an unnamespaced one. A last
    // step of "@colour" would make them one path, and the entry left standing
    // would silence the one struck out.
    let namespaced = format!("{bogus}/@{}", step("colour"));
    assert_eq!(
        census(&findings(&without(&namespaced), "namespaced.xml")),
        [
            (bogus, record.clone(), within.clone()),
            (namespaced, record, within)
        ]
    );
}

/// The paths of `every-verdict.xml`'s record, each with the step below the record a
/// finding about it selects; none for an attribute of the record element.
const EVERY_VERDICT: [(&str, &str); 7] = [
    ("/item/@id", ""),
    ("/item/@internal", ""),
    ("/item/title", "title[1]"),
    ("/item/supersededTitle", "supersededTitle[1]"),
    ("/item/summary", "summary[1]"),
    ("/item/checked", "checked[1]"),
    ("/item/note", "note[1]"),
];

#[test]
fn reports_nothing_for_a_record_every_path_of_which_has_an_entry_whatever_its_verdict() {
    assert_eq!(census(&findings(&tiny(), "every-verdict.xml")), Vec::new());

    for (path, within) in EVERY_VERDICT {
        let found = findings(&without(path), "every-verdict.xml");
        assert_eq!(
            census(&found),
            [(
                path.to_owned(),
                "/catalog/item[1]".to_owned(),
                within.to_owned()
            )],
            "the entry for {path} is what silences it"
        );
        let annotation = censuses(&found).first().expect("a census").clone();
        assert_eq!(
            refinements(&found, &annotation),
            usize::from(!within.is_empty()),
            "what a finding about {path} selects below the record"
        );
    }
}

const UNPARSEABLE: &str = "@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#\n";

const NOTHING_ACCOUNTED: &str = "# no path of the catalog format is accounted for yet\n";

#[test]
fn refuses_an_unparseable_accounting_where_an_absent_one_is_read_in_silence() {
    assert_eq!(census(&findings(&unaccounted(), "two.xml")), Vec::new());

    let Err(refusal) = conversion(&with_accounting(UNPARSEABLE), "two.xml") else {
        panic!("an accounting that is not Turtle is read as an accounting of nothing");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(ACCOUNTING), "{refusal}");
}

#[test]
fn refuses_an_accounting_the_crate_names_and_nothing_answers_to() {
    assert_eq!(census(&findings(&unaccounted(), "two.xml")), Vec::new());

    let missing = Variant::of(tiny()).replacing_exactly(CRATE, ACCOUNTING, UNWRITTEN, 2);
    let Err(refusal) = conversion(&missing, "two.xml") else {
        panic!("a crate that says what it accounts for and cannot show it is read in silence");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(UNWRITTEN), "{refusal}");
}

#[test]
fn accounts_for_no_path_with_an_empty_accounting_where_an_absent_one_accounts_for_every_path() {
    assert_eq!(census(&findings(&unaccounted(), "two.xml")), Vec::new());

    let reported = findings(&with_accounting(NOTHING_ACCOUNTED), "two.xml");
    assert_eq!(
        paths(&reported).into_iter().collect::<Vec<String>>(),
        ["/item/@id", "/item/note", "/item/title"]
    );
    assert_eq!(
        census(&reported),
        [
            (
                "/item/@id".to_owned(),
                "/catalog/item[1]".to_owned(),
                String::new()
            ),
            (
                "/item/@id".to_owned(),
                "/catalog/item[2]".to_owned(),
                String::new()
            ),
            (
                "/item/note".to_owned(),
                "/catalog/item[1]".to_owned(),
                "note[1]".to_owned()
            ),
            (
                "/item/title".to_owned(),
                "/catalog/item[1]".to_owned(),
                "title[1]".to_owned()
            ),
        ],
        "three paths of the record with a title and a note, one of the record without"
    );
}

/// A path hung on a node that says nothing about what it is.
const NO_PATH_ENTRY: &str = r#"@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#> .

[] bridge:sourcePath "/item/@id" ;
   bridge:verdict bridge:carried .
"#;

/// An entry whose path is a name rather than a string, which no path of a
/// record can equal.
const A_PATH_THAT_IS_NO_LITERAL: &str = r#"@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#> .

[] a bridge:PathEntry ;
   bridge:sourcePath <urn:example:catalog#id> ;
   bridge:verdict bridge:carried .
"#;

#[test]
fn accounts_for_no_path_by_a_source_path_hung_on_a_node_that_is_no_path_entry() {
    assert_eq!(
        paths(&findings(&with_accounting(NO_PATH_ENTRY), "two.xml"))
            .into_iter()
            .collect::<Vec<String>>(),
        ["/item/@id", "/item/note", "/item/title"],
        "the path the file names is the one it would have silenced"
    );
}

#[test]
fn refuses_a_source_path_that_is_no_literal_where_dropping_it_would_report_the_path() {
    let Err(refusal) = conversion(&with_accounting(A_PATH_THAT_IS_NO_LITERAL), "two.xml") else {
        panic!("a path that is not a string is read past in silence");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(ACCOUNTING), "{refusal}");
}

#[test]
fn names_the_census_body_the_specification_fixed_and_no_other() {
    let findings = findings(&tiny(), "unaccounted-child.xml");
    let bodies: Vec<String> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
        .map(|q| q.object.to_string())
        .filter(|body| body.contains(BRIDGE))
        .collect();
    assert_eq!(bodies, [format!("<{PATH_NOT_ACCOUNTED}>")]);
}
