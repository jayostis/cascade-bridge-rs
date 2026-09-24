// A finding's address is an XPath, and this Bridge follows it. An address that
// selects no node, or several, names nothing the finding can be about, so the
// Bridge reports it beside the adapter's own finding and produces the graph all
// the same: verification reports, and never refuses.
//
// An address that does select one node is compared by that node: two findings
// whose addresses reach the same element are one finding however either of
// them is spelled. Comparison is stricter than conversion: an address on
// either side that reaches other than one node fails the entry, and is said to
// have failed it by the address and what it selected.
mod common;

use cascade_bridge::{
    canonical_lines, load_adapter, run_manifest, Conversion, Resolver, RunOptions,
};
use common::{address, annotations, committed, converted, says, tiny, Variant, OA, SH};
use oxrdf::{Quad, Term};

const ADDRESS_NOT_ONE_NODE: &str =
    "https://ns.cascadeprotocol.org/bridge/v1-draft#addressNotOneNode";

/// The findings query whose annotations name a node inside the record: the one
/// file that decides what a refinement of the tiny adapter says.
const NOTE_QUERY: &str = "mapping/item-note-findings.rq";
const NOTE: &str = "rdf:value ?at";

/// The tiny adapter with one string of one of its files replaced.
fn variant(file: &str, from: &str, to: &str) -> Variant {
    Variant::of(tiny()).replacing(file, from, to)
}

/// Every address this Bridge could not follow, as the node the report selects
/// and the address as it was written.
fn reports(findings: &[Quad]) -> Vec<(String, String)> {
    let mut reported: Vec<(String, String)> = findings
        .iter()
        .filter(|quad| quad.predicate.as_str() == format!("{OA}hasBody"))
        .filter(
            |quad| matches!(&quad.object, Term::NamedNode(n) if n.as_str() == ADDRESS_NOT_ONE_NODE),
        )
        .map(|quad| {
            let annotation = quad.subject.to_string();
            let (selects, _) = address(findings, &annotation);
            (selects, says(findings, &annotation, &format!("{SH}value")))
        })
        .collect();
    reported.sort();
    reported
}

fn graph(conversion: &Conversion) -> Vec<String> {
    canonical_lines(conversion.quads.clone())
        .expect("canonical")
        .into_iter()
        .collect()
}

#[test]
fn reports_an_address_that_selects_no_node_and_produces_the_graph_all_the_same() {
    let plain = converted(&tiny(), "two.xml");
    let strayed = converted(
        &variant(NOTE_QUERY, NOTE, "rdf:value \"nowhere\""),
        "two.xml",
    );
    assert_eq!(reports(&plain.findings), Vec::<(String, String)>::new());
    assert_eq!(
        reports(&strayed.findings),
        [("/catalog".to_owned(), "nowhere".to_owned())]
    );
    assert_eq!(graph(&strayed), graph(&plain));
}

#[test]
fn reports_an_address_that_selects_more_than_one_node() {
    let several = converted(&variant(NOTE_QUERY, NOTE, "rdf:value \"*\""), "two.xml");
    assert_eq!(
        reports(&several.findings),
        [("/catalog".to_owned(), "*".to_owned())],
        "the record holds a title and a note, and an address selects one node"
    );
}

#[test]
fn reports_an_address_that_is_no_xpath_at_all() {
    let nonsense = converted(&variant(NOTE_QUERY, NOTE, "rdf:value \"(((\""), "two.xml");
    assert_eq!(
        reports(&nonsense.findings),
        [("/catalog".to_owned(), "(((".to_owned())]
    );
}

#[test]
fn reports_an_address_two_findings_of_one_record_share_once() {
    let shared = converted(
        &variant(NOTE_QUERY, NOTE, "rdf:value \"nowhere\""),
        "order.xml",
    );
    assert_eq!(
        reports(&shared.findings),
        [
            ("/catalog".to_owned(), "nowhere".to_owned()),
            ("/catalog".to_owned(), "nowhere".to_owned())
        ],
        "the first record's two notes share one address, and one report; each \
         record reports the address its own findings carry, and every report \
         selects the document element"
    );
}

/// A comment and a processing instruction are no part of the graph the lift
/// builds, and XPath counts both, so the record the lift writes out again
/// carries them where the document wrote them.
#[test]
fn follows_an_address_through_what_the_graph_leaves_out() {
    for address in ["comment()[1]", "processing-instruction()[1]", "node()[2]"] {
        let counted = converted(
            &Variant::of(tiny())
                .replacing(
                    "fixtures/in/two.xml",
                    "<item id=\"1\">",
                    "<item id=\"1\"><!-- said of the first --><?say it again?>",
                )
                .replacing(NOTE_QUERY, NOTE, format!("rdf:value \"{address}\"")),
            "two.xml",
        );
        assert_eq!(
            reports(&counted.findings),
            Vec::<(String, String)>::new(),
            "{address}"
        );
    }
}

/// A refinement selects one node of the record its selector names, so an
/// address is followed through the record read as a document of its own, and
/// one that walks out of the record reaches nothing the finding can be about.
/// An address rooted at the document is one of those, the record being the
/// root of what it is read from.
#[test]
fn reports_an_address_that_leaves_the_record() {
    for address in [
        "..",
        "../item[2]",
        "following-sibling::item[1]",
        "/catalog/item[1]/note[1]",
    ] {
        let outward = converted(
            &variant(NOTE_QUERY, NOTE, &format!("rdf:value \"{address}\"")),
            "two.xml",
        );
        assert_eq!(
            reports(&outward.findings),
            [("/catalog".to_owned(), address.to_owned())],
            "{address}"
        );
    }
}

/// The guard that verification is not noisy: every address the adapter on disk
/// writes, for every input it committed, selects the one node it names. It
/// guards nothing unless some of those findings are addressed below their
/// record, so it counts them rather than how many inputs there are.
#[test]
fn reports_nothing_for_any_input_the_adapter_committed() {
    let resolver = tiny();
    let mut followed = 0;
    for input in committed() {
        let conversion = converted(&resolver, &input);
        assert_eq!(
            reports(&conversion.findings),
            Vec::<(String, String)>::new(),
            "{input}"
        );
        followed += annotations(&conversion.findings)
            .iter()
            .filter(|annotation| !address(&conversion.findings, annotation).1.is_empty())
            .count();
    }
    assert!(
        followed > 0,
        "no committed input draws a finding addressed below its record"
    );
}

#[test]
fn produces_the_graph_for_a_document_no_tree_can_be_built_from() {
    let two_rooted = converted(
        &variant(
            "fixtures/in/two.xml",
            "</catalog>",
            "</catalog>\n<catalog/>",
        ),
        "two.xml",
    );
    assert_eq!(graph(&two_rooted), graph(&converted(&tiny(), "two.xml")));
    assert_eq!(
        reports(&two_rooted.findings),
        Vec::<(String, String)>::new(),
        "a document with no one document element is judged by nothing, not refused"
    );
}

/// The oracle the manifest's passing entry is judged against, and the two
/// addresses it carries: the record its findings stand in, and the node inside
/// the record one of them is about.
const ORACLE: &str = "fixtures/findings/two.ttl";
const RECORD: &str = "\"/catalog/item[1]\"";
/// The oracle's copy of the finding the findings query writes, whole. The
/// census finding of the same record names the same node, so what a shorter
/// anchor would reach is both of them.
const QUERY_FINDING: &str = r#"rdf:value "note[1]" ]
    ]
  ] ;
  oa:hasBody ex:noteHasNoTerm ;
  oa:motivatedBy oa:classifying ;
  sh:resultSeverity sh:Info ."#;

/// That finding with its address spelled another way.
fn query_finding(address: &str) -> String {
    QUERY_FINDING.replacen("note[1]", address, 1)
}

/// What the manifest made of one of its entries, and what it said about it.
fn judged(resolver: &dyn Resolver, entry: &str) -> (String, String) {
    let adapter = load_adapter(resolver).expect("adapter");
    let results = run_manifest(&adapter, resolver, RunOptions::default()).expect("manifest");
    let result = results
        .iter()
        .find(|result| result.name == entry)
        .unwrap_or_else(|| panic!("the manifest lists {entry}"));
    (
        result.outcome.as_str().to_owned(),
        result.description.clone(),
    )
}

#[test]
fn passes_a_record_address_spelled_another_correct_way() {
    let (outcome, said) = judged(
        &variant(ORACLE, RECORD, "\"/catalog/*[local-name()='item'][1]\""),
        "pass",
    );
    assert_eq!(outcome, "passed", "{said}");
}

#[test]
fn passes_a_refinement_spelled_another_correct_way() {
    let (outcome, said) = judged(
        &variant(ORACLE, QUERY_FINDING, &query_finding("child::note[last()]")),
        "pass",
    );
    assert_eq!(outcome, "passed", "{said}");
}

/// Without this, a comparison that made every address equal would pass the two
/// above and be reported as the loosening they ask for. The oracle's other
/// record is the one moved, because both spellings must select a node for the
/// findings to be compared at all.
#[test]
fn fails_an_address_that_selects_another_node_of_the_same_document() {
    let (outcome, said) = judged(&variant(ORACLE, "\"/catalog/item[2]\"", RECORD), "pass");
    assert_eq!(outcome, "failed", "{said}");
    assert!(said.contains("findings differ"), "{said}");
}

/// The adapter's own address and the oracle's, each of them the one node the
/// finding is about, spelled two correct ways.
fn spelled(adapter: &str, oracle: &str) -> Variant {
    Variant::of(tiny())
        .replacing(NOTE_QUERY, NOTE, format!("rdf:value \"{adapter}\""))
        .replacing(ORACLE, QUERY_FINDING, query_finding(oracle))
}

/// A text node is a node a finding is about as an element is.
#[test]
fn passes_a_refinement_of_a_text_node_spelled_another_correct_way() {
    let (outcome, said) = judged(&spelled("note[1]/text()", "note[1]/text()[1]"), "pass");
    assert_eq!(outcome, "passed", "{said}");
}

/// The record is one of the nodes its own findings are about.
#[test]
fn passes_a_refinement_of_the_record_itself_spelled_another_correct_way() {
    let (outcome, said) = judged(&spelled(".", "self::item"), "pass");
    assert_eq!(outcome, "passed", "{said}");
}

/// A refinement selects one node of its record where a comparison reads it as
/// where a conversion writes it, so an address that walks out of the record
/// fails the entry by the address and what it selected, rather than being
/// compared by the node it reached out there.
#[test]
fn fails_an_entry_whose_refinement_leaves_the_record() {
    let (outcome, said) = judged(&spelled("../item[2]", "following-sibling::item[1]"), "pass");
    assert_eq!(outcome, "failed", "{said}");
    assert!(
        said.contains("produced \"../item[2]\" selects no node"),
        "{said}"
    );
    assert!(
        said.contains("expected \"following-sibling::item[1]\" selects no node"),
        "{said}"
    );
}

/// The Bridge's own report of the address, as the oracle carries it beside the
/// finding whose address it is about.
const REPORTED: &str = "@prefix ex:  <urn:example:catalog#> .

[] a oa:Annotation ;
  oa:hasTarget [
    oa:hasSource <../in/two.xml> ;
    oa:hasSelector [ a oa:XPathSelector ; rdf:value \"/catalog\" ]
  ] ;
  oa:hasBody <https://ns.cascadeprotocol.org/bridge/v1-draft#addressNotOneNode> ;
  oa:motivatedBy oa:classifying ;
  sh:value \"nowhere\" ;
  sh:resultSeverity sh:Violation .
";

/// Both sides carry the same findings and the same address, so a comparison
/// of them holds nothing missing and nothing extra: what fails the entry is
/// the address itself, said as the address and what it selected.
#[test]
fn fails_an_entry_whose_address_selects_no_node() {
    let (outcome, said) = judged(
        &Variant::of(tiny())
            .replacing(NOTE_QUERY, NOTE, "rdf:value \"nowhere\"")
            .replacing(ORACLE, QUERY_FINDING, query_finding("nowhere"))
            .replacing(ORACLE, "@prefix ex:  <urn:example:catalog#> .", REPORTED),
        "pass",
    );
    assert_eq!(outcome, "failed", "{said}");
    assert!(
        said.contains("produced \"nowhere\" selects no node"),
        "{said}"
    );
    assert!(
        said.contains("expected \"nowhere\" selects no node"),
        "{said}"
    );
    assert!(!said.contains("findings differ"), "{said}");
}

/// The first record of `order.xml` holds two notes, so a step with no index
/// selects both where each finding is about one.
#[test]
fn fails_an_entry_whose_address_selects_several_nodes() {
    let (outcome, said) = judged(
        &variant(NOTE_QUERY, NOTE, "rdf:value \"note\""),
        "findings-repeated",
    );
    assert_eq!(outcome, "failed", "{said}");
    assert!(said.contains("produced \"note\" selects 2 nodes"), "{said}");
}

/// An oracle's own address is judged by the same rule as the one the adapter
/// wrote.
#[test]
fn fails_an_entry_whose_expected_address_selects_several_nodes() {
    let (outcome, said) = judged(&variant(ORACLE, RECORD, "\"/catalog/item\""), "pass");
    assert_eq!(outcome, "failed", "{said}");
    assert!(
        said.contains("expected \"/catalog/item\" selects 2 nodes"),
        "{said}"
    );
}
