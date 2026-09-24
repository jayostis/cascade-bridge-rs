// An accounting entry whose verdict names a gap reports that gap, once per
// distinct path per record, addressed at that path's first occurrence — the
// address a census finding is written at, carrying the gap as its body, the
// path as its sh:value and, where the path stands at more than one node of the
// record, how many.
//
// Which kinds report is the gap concept's own skos:broader: a gap true of the
// path reports, and one true of what the record happens to hold at the path
// does not, because an entry cannot say which of that path's occurrences it is
// true of.
//
// A crate naming no accounting is untouched by all of it, as it was in wave 1.
mod common;

use cascade_bridge::Resolver;
use common::{
    accounting, address, annotations, concept, conversion, count, entry, findings, gap_scheme,
    node, one, says, step, tiny, with_accounting, Variant, ACCOUNTING, ACCOUNTING_PREAMBLE, BRIDGE,
    CRATE, GAPS_PREAMBLE, GAP_SCHEME, NOTE_GAP, OA, PATH_NOT_ACCOUNTED, SH, XSD_INTEGER,
};
use oxrdf::{Quad, Term};
use oxrdfio::{RdfFormat, RdfParser};

const SUMMARY_GAP: &str = "urn:example:catalog#summaryLosesItsMarkup";
const SKOS_BROADER: &str = "http://www.w3.org/2004/02/skos/core#broader";

/// Every annotation an entry reported, by the node it is: a finding about a
/// path, bodied at the gap the entry names rather than at the census's own
/// term.
fn reporters(findings: &[Quad]) -> Vec<String> {
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{SH}value"))
        .map(|q| q.subject.to_string())
        .filter(|annotation| {
            says(findings, annotation, &format!("{OA}hasBody")) != PATH_NOT_ACCOUNTED
        })
        .collect()
}

/// Each reported gap as what it says and where it says it: the gap it bodies,
/// the path it names, the record it is about, and the occurrence inside that
/// record it is addressed at.
fn reported(findings: &[Quad]) -> Vec<(String, String, String, String)> {
    let mut rows: Vec<(String, String, String, String)> = reporters(findings)
        .iter()
        .map(|annotation| {
            let (record, within) = address(findings, annotation);
            (
                says(findings, annotation, &format!("{OA}hasBody")),
                says(findings, annotation, &format!("{SH}value")),
                record,
                within,
            )
        })
        .collect();
    rows.sort();
    rows
}

/// A row of `reported`, spelled as a test writes one.
fn row(gap: &str, path: &str, record: &str, within: &str) -> (String, String, String, String) {
    (
        gap.to_owned(),
        path.to_owned(),
        record.to_owned(),
        within.to_owned(),
    )
}

/// Each reported gap as the record it is about and the count it carries.
fn counted(findings: &[Quad]) -> Vec<(String, Option<String>)> {
    let mut rows: Vec<(String, Option<String>)> = reporters(findings)
        .iter()
        .map(|annotation| (address(findings, annotation).0, count(findings, annotation)))
        .collect();
    rows.sort();
    rows
}

/// The one annotation an entry reported about this path.
fn about(findings: &[Quad], path: &str) -> String {
    let mut found = reporters(findings)
        .into_iter()
        .filter(|annotation| says(findings, annotation, &format!("{SH}value")) == path);
    let first = found.next();
    assert!(first.is_some(), "no finding reports a gap at {path}");
    assert!(found.next().is_none(), "more than one finding about {path}");
    first.expect("checked above")
}

/// The tiny adapter with its gap scheme replaced, so a kind and a severity can
/// be varied without being committed.
fn with_gaps(body: &str) -> Variant {
    Variant::of(tiny()).with(GAP_SCHEME, body)
}

/// One entry naming the gap a case turns on, or none.
fn stated(path: &str, verdict: &str, gap: Option<&str>) -> String {
    match gap {
        Some(gap) => entry(path, verdict, &[&format!("bridge:namesGap {gap}")]),
        None => entry(path, verdict, &[]),
    }
}

const UNWRITTEN: &str = "vocab/gaps-nobody-wrote.ttl";

#[test]
fn reports_a_path_a_record_carries_three_times_once_at_the_first_counting_three() {
    let found = findings(&tiny(), "gap-three-times.xml");
    assert_eq!(
        reported(&found),
        [row(NOTE_GAP, "/item/note", "/catalog/item[1]", "note[1]")],
        "one finding stands for every node at the path"
    );
    assert_eq!(
        count(&found, &about(&found, "/item/note")),
        Some("3".to_owned())
    );
}

#[test]
fn carries_no_count_on_a_finding_for_a_path_the_record_carries_once() {
    let found = findings(&tiny(), "two.xml");
    assert_eq!(
        reported(&found),
        [row(NOTE_GAP, "/item/note", "/catalog/item[1]", "note[1]")]
    );
    assert_eq!(
        count(&found, &about(&found, "/item/note")),
        None,
        "a count of one is what its absence says"
    );
}

#[test]
fn counts_only_its_own_record_s_nodes_where_two_records_carry_the_path() {
    let found = findings(&tiny(), "order.xml");
    assert_eq!(
        reported(&found),
        [
            row(NOTE_GAP, "/item/note", "/catalog/item[1]", "note[1]"),
            row(NOTE_GAP, "/item/note", "/catalog/item[2]", "note[1]")
        ]
    );
    assert_eq!(
        counted(&found),
        [
            ("/catalog/item[1]".to_owned(), Some("2".to_owned())),
            ("/catalog/item[2]".to_owned(), None)
        ],
        "the record with two notes counts two, the record with one counts nothing"
    );
}

#[test]
fn writes_the_count_as_an_xsd_integer() {
    let found = findings(&tiny(), "gap-three-times.xml");
    let occurrences = one(
        &found,
        &about(&found, "/item/note"),
        &format!("{BRIDGE}occurrences"),
    )
    .expect("a count");
    let Term::Literal(literal) = occurrences else {
        panic!("a count is a literal: {occurrences}");
    };
    assert_eq!(literal.datatype().as_str(), XSD_INTEGER);
}

/// The paths of `unaccounted-attribute.xml`'s record, the two attributes of it
/// named as gaps: one on a child element, one on the record element itself.
fn attribute_gaps() -> Variant {
    with_accounting(&accounting(&[
        stated("/item/@id", "noHome", Some("ex:noteHasNoTerm")),
        stated("/item/title", "carried", None),
        stated("/item/label", "carried", None),
        stated("/item/label/@colour", "noHome", Some("ex:noteHasNoTerm")),
    ]))
}

#[test]
fn addresses_an_attribute_s_finding_at_the_element_it_stands_on() {
    assert_eq!(
        reported(&findings(&attribute_gaps(), "unaccounted-attribute.xml")),
        [
            row(NOTE_GAP, "/item/@id", "/catalog/item[1]", ""),
            row(
                NOTE_GAP,
                "/item/label/@colour",
                "/catalog/item[1]",
                "label[1]"
            )
        ],
        "an XPath selector has no form for an attribute, and the lift writes elements"
    );
}

#[test]
fn refines_an_attribute_of_the_record_element_no_further() {
    let found = findings(&attribute_gaps(), "unaccounted-attribute.xml");
    let annotation = about(&found, "/item/@id");
    let target = node(&found, &annotation, &format!("{OA}hasTarget"));
    let selector = node(&found, &target, &format!("{OA}hasSelector"));
    assert_eq!(
        found
            .iter()
            .filter(|q| q.subject.to_string() == selector)
            .filter(|q| q.predicate.as_str() == format!("{OA}refinedBy"))
            .count(),
        0,
        "the target's selector already names the element the attribute stands on"
    );
}

#[test]
fn writes_a_namespaced_path_as_the_census_writes_it_for_the_same_node() {
    let item = step("item");
    let bogus = step("bogus");
    let path = format!("/{item}/{bogus}");
    let namespaced = with_accounting(&accounting(&[
        stated(&format!("/{item}/@id"), "carried", None),
        stated(&path, "noHome", Some("ex:noteHasNoTerm")),
        stated(&format!("{path}/@colour"), "carried", None),
        stated(&format!("{path}/@{}", step("colour")), "carried", None),
    ]));
    assert_eq!(
        reported(&findings(&namespaced, "namespaced.xml")),
        [row(
            NOTE_GAP,
            &path,
            &format!("/{}/{item}[1]", step("catalog")),
            &format!("{bogus}[1]")
        )],
        "a step a census would write, written by an entry's finding for the same node"
    );
}

#[test]
fn takes_the_severity_the_gap_declares_and_sh_info_where_it_declares_none() {
    let declared = with_gaps(&gap_scheme(&[
        concept("noteHasNoTerm", "noPredicate", Some("Warning")),
        concept("summaryLosesItsMarkup", "sourceLacksRequired", None),
    ]));
    let found = findings(&declared, "every-verdict.xml");
    assert_eq!(
        says(
            &found,
            &about(&found, "/item/note"),
            &format!("{SH}resultSeverity")
        ),
        format!("{SH}Warning"),
        "severity is a property of the kind of problem, not of one occurrence of it"
    );
    assert_eq!(
        says(
            &found,
            &about(&found, "/item/summary"),
            &format!("{SH}resultSeverity")
        ),
        format!("{SH}Info")
    );
}

#[test]
fn names_the_path_as_sh_value_the_same_string_the_entry_s_source_path_carries() {
    let found = findings(&tiny(), "every-verdict.xml");
    let mut paths: Vec<String> = reported(&found)
        .into_iter()
        .map(|(_, path, ..)| path)
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        ["/item/note"],
        "one finding stands for every node at the path, and one node's value would drop the rest"
    );
}

#[test]
fn bodies_a_reported_gap_at_the_gap_the_entry_names_and_motivates_it_by_classifying() {
    let found = findings(&tiny(), "every-verdict.xml");
    assert_eq!(
        reported(&found),
        [row(NOTE_GAP, "/item/note", "/catalog/item[1]", "note[1]")]
    );
    for annotation in reporters(&found) {
        assert_eq!(
            says(&found, &annotation, &format!("{OA}motivatedBy")),
            format!("{OA}classifying")
        );
        let target = node(&found, &annotation, &format!("{OA}hasTarget"));
        assert!(
            says(&found, &target, &format!("{OA}hasSource"))
                .ends_with("fixtures/in/every-verdict.xml"),
            "{found:?}"
        );
    }
}

#[test]
fn reports_a_gap_of_every_kind_that_is_true_of_the_path() {
    for kind in ["noPredicate", "sourceLacksRequired"] {
        let declared = with_gaps(&gap_scheme(&[
            concept("noteHasNoTerm", kind, None),
            concept("summaryLosesItsMarkup", "carriedWithLoss", None),
        ]));
        let found = findings(&declared, "two.xml");
        assert_eq!(
            reported(&found),
            [row(NOTE_GAP, "/item/note", "/catalog/item[1]", "note[1]")],
            "a gap of kind {kind} is true of the path"
        );
    }
}

#[test]
fn emits_nothing_for_a_gap_of_a_kind_true_of_what_the_record_holds_at_the_path() {
    for kind in ["carriedWithLoss", "valueNotMapped", "schemaRuleUnnamed"] {
        let declared = with_gaps(&gap_scheme(&[
            concept("noteHasNoTerm", kind, None),
            concept("summaryLosesItsMarkup", kind, None),
        ]));
        assert_eq!(
            reported(&findings(&declared, "every-verdict.xml")),
            Vec::new(),
            "only some of a path's occurrences are {kind}, and an entry cannot say which"
        );
    }
}

/// Every case above reads the reporting set forwards: it names a kind and
/// asserts a finding. Narrowing the set passes all of them by reporting
/// nothing at all, so the set is read backwards here — with a guard that the
/// committed adapter really does stand an entry, a gap and a record at the
/// path, and would report the moment the kind said the path rather than what
/// the record holds at it.
#[test]
fn bodies_no_finding_at_a_gap_the_committed_adapter_declares_under_a_kind_that_does_not_report() {
    let resolver = tiny();
    let iri = format!("{}{GAP_SCHEME}", resolver.root());
    let scheme = resolver.read(&iri).expect("the committed gap scheme");
    let declared: Vec<Quad> = RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(&iri)
        .expect("the scheme's IRI")
        .for_slice(&scheme)
        .collect::<Result<_, _>>()
        .expect("the committed gap scheme is Turtle");
    assert_eq!(
        says(&declared, &format!("<{SUMMARY_GAP}>"), SKOS_BROADER),
        format!("{BRIDGE}carriedWithLoss"),
        "the committed scheme declares the summary's gap under a kind that does not report"
    );

    let found = findings(&resolver, "every-verdict.xml");
    let bodies: Vec<String> = annotations(&found)
        .iter()
        .map(|annotation| says(&found, annotation, &format!("{OA}hasBody")))
        .collect();
    assert!(
        !bodies.iter().any(|body| body == SUMMARY_GAP),
        "no finding a run produced bodies a loss the entry recorded: {bodies:?}"
    );

    let reporting = with_gaps(&gap_scheme(&[
        concept("noteHasNoTerm", "noPredicate", None),
        concept("summaryLosesItsMarkup", "noPredicate", None),
    ]));
    assert_eq!(
        reported(&findings(&reporting, "every-verdict.xml")),
        [
            row(NOTE_GAP, "/item/note", "/catalog/item[1]", "note[1]"),
            row(
                SUMMARY_GAP,
                "/item/summary",
                "/catalog/item[1]",
                "summary[1]"
            )
        ],
        "the entry is live and the record stands at its path: the kind is the only thing quieting it"
    );
}

#[test]
fn emits_nothing_for_a_verdict_that_names_no_gap() {
    let no_gap = with_accounting(&accounting(&[
        stated("/item/@id", "carried", Some("ex:noteHasNoTerm")),
        stated("/item/@internal", "ignored", Some("ex:noteHasNoTerm")),
        stated("/item/title", "carried", Some("ex:noteHasNoTerm")),
        stated(
            "/item/supersededTitle",
            "redundantWith",
            Some("ex:noteHasNoTerm"),
        ),
        stated("/item/summary", "consumed", Some("ex:noteHasNoTerm")),
        stated("/item/checked", "consumed", Some("ex:noteHasNoTerm")),
        stated("/item/note", "carried", Some("ex:noteHasNoTerm")),
    ]));
    assert_eq!(
        reported(&findings(&no_gap, "every-verdict.xml")),
        Vec::new(),
        "the verdict is what decides whether an entry names a gap at all"
    );
}

#[test]
fn leaves_a_crate_whose_entries_name_no_gaps_the_findings_it_has_today() {
    let silent = with_accounting(&accounting(&[
        stated("/item/@id", "carried", None),
        stated("/item/title", "carried", None),
        stated("/item/note", "carried", None),
    ]));
    let found = findings(&silent, "two.xml");
    assert_eq!(reported(&found), Vec::new());
    assert_eq!(
        annotations(&found).len(),
        2,
        "the two findings the adapter's queries construct, and nothing else"
    );
}

#[test]
fn stands_a_reported_gap_beside_the_finding_a_query_constructs_at_the_same_node() {
    let found = findings(&tiny(), "two.xml");
    let noted: Vec<String> = found
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == NOTE_GAP))
        .map(|q| q.subject.to_string())
        .collect();
    assert_eq!(
        noted.len(),
        2,
        "the query's finding and the entry's both stand; retiring the query is the adapter's job"
    );
    let valued: Vec<String> = noted
        .iter()
        .map(|annotation| says(&found, annotation, &format!("{SH}value")))
        .collect();
    assert!(
        valued.contains(&"/item/note".to_owned()) && valued.contains(&String::new()),
        "one names the path and one does not: {valued:?}"
    );
}

/// Neither Turtle nor anything else: a gap scheme that cannot be read at all.
const UNPARSEABLE: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#\n";

#[test]
fn refuses_an_unparseable_gap_scheme() {
    let Err(refusal) = conversion(&with_gaps(UNPARSEABLE), "two.xml") else {
        panic!("a gap scheme that is not Turtle is read as a scheme declaring nothing");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(GAP_SCHEME), "{refusal}");
}

#[test]
fn refuses_a_gap_scheme_the_crate_names_and_nothing_answers_to() {
    let missing = Variant::of(tiny()).replacing_exactly(CRATE, GAP_SCHEME, UNWRITTEN, 1);
    let Err(refusal) = conversion(&missing, "two.xml") else {
        panic!("a crate that says what it does not carry and cannot show it is read in silence");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(UNWRITTEN), "{refusal}");
}

/// The scheme with everything the committed accounting names declared as it
/// declares it, but for the one concept a case is about.
fn scheme_but_for(concept_under_test: &str) -> String {
    format!(
        "{GAPS_PREAMBLE}\nex:gaps a skos:ConceptScheme .\n{}\n{concept_under_test}",
        concept("summaryLosesItsMarkup", "carriedWithLoss", None)
    )
}

/// One story for every way a gap scheme can fail to say what kind of gap an
/// entry names and how severe it is: the run is refused, naming what it could
/// not read. A Bridge that went quiet instead would drop a finding for a
/// reason the reader of its output cannot see, which is the worse failure the
/// contract's point 3 names — and a finding the specification's own
/// <#SourceFinding> shape would then refuse is no better.
#[test]
fn refuses_a_gap_declaring_a_severity_the_specification_does_not_name() {
    let catastrophic = with_gaps(&scheme_but_for(
        "ex:noteHasNoTerm a skos:Concept ;\n  skos:inScheme ex:gaps ;\n  skos:broader bridge:noPredicate ;\n  sh:resultSeverity ex:Catastrophic .\n",
    ));
    let Err(refusal) = conversion(&catastrophic, "two.xml") else {
        panic!("a severity outside the three is carried into a finding the profile refuses");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("Catastrophic"), "{refusal}");
}

#[test]
fn refuses_a_gap_declaring_two_severities() {
    let both = with_gaps(&scheme_but_for(
        "ex:noteHasNoTerm a skos:Concept ;\n  skos:inScheme ex:gaps ;\n  skos:broader bridge:noPredicate ;\n  sh:resultSeverity sh:Warning, sh:Violation .\n",
    ));
    let Err(refusal) = conversion(&both, "two.xml") else {
        panic!("a gap declaring two severities is judged at whichever one the parse saw last");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("noteHasNoTerm"), "{refusal}");
}

#[test]
fn refuses_a_gap_declaring_a_severity_that_is_no_iri() {
    let quoted = with_gaps(&scheme_but_for(
        "ex:noteHasNoTerm a skos:Concept ;\n  skos:inScheme ex:gaps ;\n  skos:broader bridge:noPredicate ;\n  sh:resultSeverity \"sh:Warning\" .\n",
    ));
    let Err(refusal) = conversion(&quoted, "two.xml") else {
        panic!("a severity that is not an IRI is dropped, and the gap reports at sh:Info");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("noteHasNoTerm"), "{refusal}");
}

#[test]
fn refuses_a_gap_the_scheme_declares_with_no_kind() {
    let kindless = with_gaps(&scheme_but_for(
        "ex:noteHasNoTerm a skos:Concept ;\n  skos:inScheme ex:gaps ;\n  sh:resultSeverity sh:Warning .\n",
    ));
    let Err(refusal) = conversion(&kindless, "two.xml") else {
        panic!("a gap with no skos:broader is read as a gap of a kind that does not report");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("noteHasNoTerm"), "{refusal}");
}

#[test]
fn refuses_an_entry_naming_a_gap_the_scheme_does_not_declare() {
    let unwritten = with_accounting(&accounting(&[
        stated("/item/@id", "carried", None),
        stated("/item/title", "carried", None),
        stated("/item/note", "noHome", Some("ex:noGapAnyoneDeclared")),
    ]));
    let Err(refusal) = conversion(&unwritten, "two.xml") else {
        panic!("a gap nothing declares is read as a gap of a kind that does not report");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("noGapAnyoneDeclared"), "{refusal}");
}

/// An accounting's verdict and the gap it names are IRIs, as its
/// bridge:sourcePath is a literal, and the same parse refuses all three.
#[test]
fn refuses_a_verdict_that_is_no_iri() {
    let quoted = with_accounting(&format!(
        "{ACCOUNTING_PREAMBLE}\n[] a bridge:PathEntry ;\n   bridge:sourcePath \"/item/note\" ;\n   bridge:verdict \"noHome\" .\n"
    ));
    let Err(refusal) = conversion(&quoted, "two.xml") else {
        panic!("a verdict that is not an IRI is dropped, and its entry accounts for the path");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(ACCOUNTING), "{refusal}");
}

#[test]
fn refuses_a_named_gap_that_is_no_iri() {
    let quoted = with_accounting(&format!(
        "{ACCOUNTING_PREAMBLE}\n[] a bridge:PathEntry ;\n   bridge:sourcePath \"/item/note\" ;\n   bridge:verdict bridge:noHome ;\n   bridge:namesGap \"ex:noteHasNoTerm\" .\n"
    ));
    let Err(refusal) = conversion(&quoted, "two.xml") else {
        panic!("a gap that is not an IRI is dropped, and its entry reports nothing");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(ACCOUNTING), "{refusal}");
}

#[test]
fn refuses_a_gap_declaring_two_kinds() {
    let both = with_gaps(&scheme_but_for(
        "ex:noteHasNoTerm a skos:Concept ;\n  skos:inScheme ex:gaps ;\n  skos:broader bridge:noPredicate, bridge:carriedWithLoss .\n",
    ));
    let Err(refusal) = conversion(&both, "two.xml") else {
        panic!("a gap declaring two kinds reports, or does not, by whichever the parse saw last");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("noteHasNoTerm"), "{refusal}");
}

/// What an entry says is what it says once. A second verdict leaves the parse
/// order deciding whether the entry reports at all, and a second gap which gap
/// it reports, which is the reason a second severity is refused one file over.
#[test]
fn refuses_an_entry_declaring_two_verdicts() {
    let both = with_accounting(&format!(
        "{ACCOUNTING_PREAMBLE}\n[] a bridge:PathEntry ;\n   bridge:sourcePath \"/item/note\" ;\n   bridge:verdict bridge:carried, bridge:noHome ;\n   bridge:namesGap ex:noteHasNoTerm .\n"
    ));
    let Err(refusal) = conversion(&both, "two.xml") else {
        panic!("an entry declaring two verdicts is judged at whichever one the parse saw last");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("/item/note"), "{refusal}");
}

#[test]
fn refuses_an_entry_naming_two_gaps() {
    let both = with_accounting(&format!(
        "{ACCOUNTING_PREAMBLE}\n[] a bridge:PathEntry ;\n   bridge:sourcePath \"/item/note\" ;\n   bridge:verdict bridge:noHome ;\n   bridge:namesGap ex:noteHasNoTerm, ex:itemHasNoTitle .\n"
    ));
    let Err(refusal) = conversion(&both, "two.xml") else {
        panic!("an entry naming two gaps reports whichever one the parse saw last");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("/item/note"), "{refusal}");
}

/// A bridge:verdict and a bridge:namesGap are read from a bridge:PathEntry and
/// nowhere else, so what neither is read from cannot make a run fail. A
/// bridge:sourcePath is the standing exception: dropping one that is no
/// literal would leave the census reporting the path as unaccounted, which is
/// a wrong finding rather than an absent one.
#[test]
fn reads_no_verdict_and_no_gap_from_a_subject_that_is_no_path_entry() {
    let alongside = with_accounting(&format!(
        "{}\nex:notAnEntry bridge:verdict \"free text\" ;\n   bridge:namesGap \"ex:noteHasNoTerm\" ;\n   bridge:verdict \"twice over\" .\n",
        accounting(&[
            stated("/item/@id", "carried", None),
            stated("/item/title", "carried", None),
            stated("/item/note", "noHome", Some("ex:noteHasNoTerm")),
        ])
    ));
    assert_eq!(
        reported(&findings(&alongside, "two.xml")),
        [row(NOTE_GAP, "/item/note", "/catalog/item[1]", "note[1]")],
        "the entries report what they report, and the subject beside them is not one"
    );
}

#[test]
fn refuses_a_source_path_that_is_no_literal_wherever_it_stands() {
    let addressed = with_accounting(&format!(
        "{ACCOUNTING_PREAMBLE}\nex:notAnEntry bridge:sourcePath <urn:example:catalog#note> .\n"
    ));
    let Err(refusal) = conversion(&addressed, "two.xml") else {
        panic!("a path that is no literal is dropped, and the census reports the path it named");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(ACCOUNTING), "{refusal}");
}

/// Contract point 3, which no test was named for: the verdict decides whether
/// the entry names a gap at all, and it decides that on its own. The pairing
/// of a verdict with a kind is the adapter profile's to refuse, so
/// bridge:carriedInPart naming a gap of kind bridge:noPredicate — a pairing
/// the lint's table does not list — reports rather than going quiet.
#[test]
fn reports_from_a_verdict_that_names_a_gap_and_from_no_other_whatever_kind_it_names() {
    for verdict in ["noHome", "carriedInPart"] {
        let stated = with_accounting(&accounting(&[stated(
            "/item/note",
            verdict,
            Some("ex:noteHasNoTerm"),
        )]));
        assert_eq!(
            reported(&findings(&stated, "two.xml")),
            [row(NOTE_GAP, "/item/note", "/catalog/item[1]", "note[1]")],
            "bridge:{verdict} names a gap"
        );
    }
    for verdict in ["carried", "ignored", "consumed", "redundantWith"] {
        let stated = with_accounting(&accounting(&[stated(
            "/item/note",
            verdict,
            Some("ex:noteHasNoTerm"),
        )]));
        assert_eq!(
            reported(&findings(&stated, "two.xml")),
            Vec::new(),
            "bridge:{verdict} names no gap"
        );
    }
}

/// The choice, recorded: an entry whose verdict names no gap is read no
/// further, so what it names need not be a gap the scheme declares. An
/// accounting is refused for what makes it unreadable, and not for what an
/// entry the Bridge never consults happens to say.
#[test]
fn asks_nothing_of_the_scheme_about_a_gap_named_by_a_verdict_that_names_none() {
    let undeclared = with_accounting(&accounting(&[
        stated("/item/@id", "carried", None),
        stated("/item/title", "carried", None),
        stated("/item/note", "carried", Some("ex:noGapAnyoneDeclared")),
    ]));
    assert_eq!(reported(&findings(&undeclared, "two.xml")), Vec::new());
}
