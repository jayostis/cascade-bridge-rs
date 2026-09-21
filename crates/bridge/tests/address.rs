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
use cascade_bridge::{
    canonical_lines, convert, load_adapter, prepare, run_manifest, Conversion, DirectoryResolver,
    Resolver, RunOptions, Source,
};
use oxrdf::{Quad, Term};
use std::path::PathBuf;

const OA: &str = "http://www.w3.org/ns/oa#";
const SH_VALUE: &str = "http://www.w3.org/ns/shacl#value";
const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";
const ADDRESS_NOT_ONE_NODE: &str =
    "https://ns.cascadeprotocol.org/bridge/v1-draft#addressNotOneNode";

/// The findings query whose annotations name a node inside the record: the one
/// file that decides what a refinement of the tiny adapter says.
const NOTE_QUERY: &str = "mapping/item-note-findings.rq";
const NOTE: &str = "rdf:value ?at";

/// The tiny adapter with some of its files rewritten as they are read, so a
/// variant of a committed input, query or oracle is run without committing one.
struct Variant {
    directory: DirectoryResolver,
    edits: Vec<(&'static str, &'static str, String)>,
}

impl Resolver for Variant {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        let mut text = String::from_utf8(bytes).expect("utf-8");
        for (file, from, to) in &self.edits {
            if !iri.ends_with(file) {
                continue;
            }
            assert!(text.contains(from), "{file} does not carry {from}");
            text = text.replace(from, to);
        }
        Ok(text.into_bytes())
    }
}

fn directory() -> DirectoryResolver {
    DirectoryResolver::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"))
        .expect("resolver")
}

fn tiny() -> Variant {
    Variant {
        directory: directory(),
        edits: Vec::new(),
    }
}

/// The tiny adapter with one string of one of its files replaced.
fn variant(file: &'static str, from: &'static str, to: &str) -> Variant {
    variants(vec![(file, from, to.to_owned())])
}

fn variants(edits: Vec<(&'static str, &'static str, String)>) -> Variant {
    Variant {
        directory: directory(),
        edits,
    }
}

fn run(resolver: &dyn Resolver, input: &str) -> Conversion {
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
    .expect("conversion")
}

fn text(term: &Term) -> String {
    match term {
        Term::Literal(literal) => literal.value().to_owned(),
        other => other.to_string(),
    }
}

fn objects(findings: &[Quad], subject: &str, predicate: &str) -> Vec<Term> {
    findings
        .iter()
        .filter(|quad| quad.subject.to_string() == subject && quad.predicate.as_str() == predicate)
        .map(|quad| quad.object.clone())
        .collect()
}

fn reached(findings: &[Quad], subject: &str, predicate: &str) -> String {
    objects(findings, subject, predicate)
        .first()
        .map(|term| term.to_string())
        .unwrap_or_default()
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
            let target = reached(findings, &annotation, &format!("{OA}hasTarget"));
            let selector = reached(findings, &target, &format!("{OA}hasSelector"));
            let selects = objects(findings, &selector, RDF_VALUE);
            let written = objects(findings, &annotation, SH_VALUE);
            (
                selects.first().map(text).unwrap_or_default(),
                written.first().map(text).unwrap_or_default(),
            )
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
    let plain = run(&tiny(), "two.xml");
    let strayed = run(
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
    let several = run(&variant(NOTE_QUERY, NOTE, "rdf:value \"*\""), "two.xml");
    assert_eq!(
        reports(&several.findings),
        [("/catalog".to_owned(), "*".to_owned())],
        "the record holds a title and a note, and an address selects one node"
    );
}

#[test]
fn reports_an_address_that_is_no_xpath_at_all() {
    let nonsense = run(&variant(NOTE_QUERY, NOTE, "rdf:value \"(((\""), "two.xml");
    assert_eq!(
        reports(&nonsense.findings),
        [("/catalog".to_owned(), "(((".to_owned())]
    );
}

#[test]
fn reports_an_address_two_findings_of_one_record_share_once() {
    let shared = run(
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

/// A comment and a processing instruction are no part of what the lift
/// rebuilds from a record, and an address that counts them is followed through
/// the source, where they stand.
#[test]
fn follows_an_address_through_what_the_lift_leaves_out() {
    for address in ["comment()[1]", "processing-instruction()[1]", "node()[2]"] {
        let counted = run(
            &variants(vec![
                (
                    "fixtures/in/two.xml",
                    "<item id=\"1\">",
                    "<item id=\"1\"><!-- said of the first --><?say it again?>".to_owned(),
                ),
                (NOTE_QUERY, NOTE, format!("rdf:value \"{address}\"")),
            ]),
            "two.xml",
        );
        assert_eq!(
            reports(&counted.findings),
            Vec::<(String, String)>::new(),
            "{address}"
        );
    }
}

/// An address is followed from the record through the whole source, so one
/// that walks out of the record reaches the node it names rather than the edge
/// of a record read on its own.
#[test]
fn follows_an_address_that_leaves_the_record() {
    let outward = run(
        &variant(NOTE_QUERY, NOTE, "rdf:value \"../item[2]\""),
        "two.xml",
    );
    assert_eq!(reports(&outward.findings), Vec::<(String, String)>::new());
}

/// Every input the adapter committed, which its oracles are written against.
fn committed() -> Vec<String> {
    let inputs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter/fixtures/in");
    let mut named: Vec<String> = std::fs::read_dir(inputs)
        .expect("the committed inputs")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    named.sort();
    named
}

/// The guard that verification is not noisy: every address the adapter on disk
/// writes, for every input it committed, selects the one node it names.
#[test]
fn reports_nothing_for_any_input_the_adapter_committed() {
    let resolver = tiny();
    let inputs = committed();
    assert!(inputs.len() > 20, "{inputs:?}");
    for input in inputs {
        let conversion = run(&resolver, &input);
        assert_eq!(
            reports(&conversion.findings),
            Vec::<(String, String)>::new(),
            "{input}"
        );
    }
}

#[test]
fn produces_the_graph_for_a_document_no_tree_can_be_built_from() {
    let two_rooted = run(
        &variant(
            "fixtures/in/two.xml",
            "</catalog>",
            "</catalog>\n<catalog/>",
        ),
        "two.xml",
    );
    assert_eq!(graph(&two_rooted), graph(&run(&tiny(), "two.xml")));
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
    variants(vec![
        (NOTE_QUERY, NOTE, format!("rdf:value \"{adapter}\"")),
        (ORACLE, QUERY_FINDING, query_finding(oracle)),
    ])
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

/// An address is followed from the record through the whole source, so one
/// that leaves the record is compared by the node it reaches out there, as one
/// that stays is by the node it reaches inside.
#[test]
fn passes_a_refinement_that_leaves_the_record_spelled_another_correct_way() {
    let (outcome, said) = judged(&spelled("../item[2]", "following-sibling::item[1]"), "pass");
    assert_eq!(outcome, "passed", "{said}");
}

/// Without this, a comparison that made every address leaving the record equal
/// would pass `passes_a_refinement_that_leaves_the_record_spelled_another_correct_way`.
#[test]
fn fails_a_refinement_that_leaves_the_record_for_another_node() {
    let (outcome, said) = judged(&spelled("../item[2]", "parent::catalog"), "pass");
    assert_eq!(outcome, "failed", "{said}");
    assert!(said.contains("findings differ"), "{said}");
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
        &variants(vec![
            (NOTE_QUERY, NOTE, "rdf:value \"nowhere\"".to_owned()),
            (ORACLE, QUERY_FINDING, query_finding("nowhere")),
            (
                ORACLE,
                "@prefix ex:  <urn:example:catalog#> .",
                REPORTED.to_owned(),
            ),
        ]),
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
