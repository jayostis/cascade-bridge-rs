// What a findings file is as bytes rather than as a graph: the same findings
// write the same text every run, two findings that differ in nothing are still
// two, and a file that differs from another by one finding differs from it in
// that finding's lines alone. The harness canonicalises before it compares and
// so sees none of this; a committed oracle's digest, and the diff an author
// reads when they regenerate one, see nothing else.
use cascade_bridge::{
    convert, load_adapter, prepare, serialise, DirectoryResolver, GraphFormat, Resolver, Source,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// The tiny adapter with its findings query replaced where a case needs one,
/// so a record can be given findings that differ in nothing without a second
/// adapter.
struct Adapter {
    directory: DirectoryResolver,
    findings_query: Option<String>,
}

impl Resolver for Adapter {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        match &self.findings_query {
            Some(text) if iri.ends_with("mapping/item-findings.rq") => Ok(text.as_bytes().to_vec()),
            _ => self.directory.read(iri),
        }
    }
}

/// A findings query that produces one annotation per solution and binds
/// nothing into it, so a record matching twice draws two findings alike in
/// every triple.
const TWICE: &str = "
PREFIX sh:     <http://www.w3.org/ns/shacl#>
PREFIX oa:     <http://www.w3.org/ns/oa#>
PREFIX fx:     <http://sparql.xyz/facade-x/ns/>
PREFIX xyz:    <http://sparql.xyz/facade-x/data/>
PREFIX bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#>
PREFIX ex:     <urn:example:catalog#>

CONSTRUCT {
  [] a oa:Annotation ;
    oa:hasTarget [ oa:hasSource bridge:thisRecord ] ;
    oa:hasBody ex:itemHasNoTitle ;
    oa:motivatedBy oa:classifying ;
    sh:resultSeverity sh:Warning .
}
WHERE {
  ?item a fx:root, xyz:item .
  VALUES ?once { 1 2 }
}
";

/// The findings of one document, written out. The document's IRI is the same
/// whatever its text, so two texts' findings are comparable line by line.
fn findings(xml: &str, query: Option<&str>) -> String {
    let resolver = Adapter {
        directory: DirectoryResolver::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"),
        )
        .expect("resolver"),
        findings_query: query.map(str::to_owned),
    };
    let adapter = load_adapter(&resolver).expect("adapter");
    let prepared = prepare(&adapter, &resolver).expect("prepared");
    let conversion = convert(
        &prepared,
        Source {
            iri: "urn:example:document",
            envelope: None,
            xml: xml.as_bytes(),
        },
    )
    .expect("conversion");
    serialise(
        &conversion.findings,
        GraphFormat::NTriples,
        &prepared.prefixes,
    )
    .expect("the findings as text")
}

const ONE_ITEM: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                        <catalog><item id=\"1\"/></catalog>";

const TWO_ITEMS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                         <catalog><item id=\"1\"/><item id=\"2\"/></catalog>";

const THREE_ITEMS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                           <catalog><item id=\"1\"/><item id=\"2\"/><item id=\"3\"/></catalog>";

/// The labels of the blank nodes a written graph names, each once.
fn labels(written: &str) -> BTreeSet<&str> {
    written
        .split("_:")
        .skip(1)
        .map(|rest| rest.split([' ', '\n']).next().unwrap_or_default())
        .collect()
}

/// Two findings alike in every triple are two findings, and a file that wrote
/// them as one blank node would hold one. That is the reading of "the same
/// graph writes the same bytes" that costs a finding, so both halves stand in
/// one case: the same bytes each run, and two of everything in them.
#[test]
fn writes_two_findings_that_differ_in_nothing_as_two_nodes_the_same_way_each_run() {
    let one = findings(ONE_ITEM, Some(TWICE));
    let two = findings(ONE_ITEM, Some(TWICE));
    assert_eq!(
        one.matches("urn:example:catalog#itemHasNoTitle").count(),
        2,
        "{one}"
    );
    assert_eq!(
        labels(&one).len(),
        6,
        "an annotation, its target and its selector, twice over: {one}"
    );
    assert_eq!(one, two);
}

/// The lines one file holds and the other does not.
fn only_in<'a>(written: &'a str, other: &str) -> Vec<&'a str> {
    let other: BTreeSet<&str> = other.lines().collect();
    written
        .lines()
        .filter(|line| !other.contains(line))
        .collect()
}

/// What a regenerated oracle's diff is worth. A label a finding elsewhere in
/// the file moved is a line changed in a finding nobody touched, and a diff
/// where every line changed says nothing about which finding did.
#[test]
fn differs_from_the_findings_of_one_more_record_in_that_record_s_finding_alone() {
    let two = findings(TWO_ITEMS, None);
    let three = findings(THREE_ITEMS, None);
    assert_eq!(
        only_in(&two, &three),
        Vec::<&str>::new(),
        "every line of the two-record findings stands in the three-record findings:\n{two}\n{three}"
    );
    assert_eq!(
        only_in(&three, &two).len(),
        two.lines().count() / 2,
        "and what they add is one finding's worth of lines, the two records \
         drawing one each:\n{three}"
    );
}
