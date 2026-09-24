// An adapter accounts for each path of its source, and a Bridge reports the
// paths a record carries that the accounting says nothing about: one finding
// per distinct path per record, addressed at that path's first occurrence.
// This is the only finding a Bridge invents that no query wrote.
//
// A crate naming no accounting gets no census, which is every adapter that
// exists, so the whole of it has to be additive.
mod common;

use cascade_bridge::{DirectoryResolver, Resolver};
use common::{conversion, on_disk, Subject};
use oxrdf::{Quad, Term};
use std::collections::BTreeSet;
use std::path::PathBuf;

const OA: &str = "http://www.w3.org/ns/oa#";
const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";
const BRIDGE: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#";

/// The body of the census finding, and the crate term that names the file it
/// is read from.
const PATH_NOT_ACCOUNTED: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#pathNotAccounted";
const SOURCE_ACCOUNTING: &str = "bridge:sourceAccounting";
const ACCOUNTING: &str = "vocab/catalog-accounting.ttl";

/// The namespace the namespaced fixture is written in.
const CATALOG: &str = "urn:example:catalog";

fn tiny() -> DirectoryResolver {
    DirectoryResolver::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"))
        .expect("resolver")
}

fn findings(resolver: &dyn Resolver, input: &str) -> Vec<Quad> {
    conversion(resolver, input).expect("conversion").findings
}

/// The one object this subject carries for this predicate, where it carries
/// exactly one.
fn one(quads: &[Quad], subject: &str, predicate: &str) -> Option<Term> {
    let mut objects = quads
        .iter()
        .filter(|q| q.subject.to_string() == subject && q.predicate.as_str() == predicate)
        .map(|q| q.object.clone());
    let first = objects.next()?;
    match objects.next() {
        None => Some(first),
        Some(_) => None,
    }
}

fn node(quads: &[Quad], subject: &str, predicate: &str) -> String {
    one(quads, subject, predicate)
        .map(|term| term.to_string())
        .unwrap_or_default()
}

/// A term as an address or a body is read: an IRI or a literal by what it
/// says, anything else by how N-Triples writes it.
fn says(quads: &[Quad], subject: &str, predicate: &str) -> String {
    match one(quads, subject, predicate) {
        Some(Term::NamedNode(named)) => named.as_str().to_owned(),
        Some(Term::Literal(literal)) => literal.value().to_owned(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Every annotation the census made, by the node it is.
fn censuses(findings: &[Quad]) -> Vec<String> {
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == PATH_NOT_ACCOUNTED))
        .map(|q| q.subject.to_string())
        .collect()
}

/// Each census finding as what it says and where it says it: the path it
/// names, the record it is about, and the occurrence inside that record it is
/// addressed at.
fn census(findings: &[Quad]) -> Vec<(String, String, String)> {
    let mut rows: Vec<(String, String, String)> = censuses(findings)
        .iter()
        .map(|annotation| {
            let target = node(findings, annotation, &format!("{OA}hasTarget"));
            let selector = node(findings, &target, &format!("{OA}hasSelector"));
            let refinement = node(findings, &selector, &format!("{OA}refinedBy"));
            (
                says(findings, annotation, &format!("{SH}value")),
                says(findings, &selector, RDF_VALUE),
                says(findings, &refinement, RDF_VALUE),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// How many nodes of the record a census finding stands for, where it says
/// so: one finding stands for every node at the path, and a path standing at
/// one node carries no count at all.
fn count(findings: &[Quad], annotation: &str) -> Option<String> {
    match one(findings, annotation, &format!("{BRIDGE}occurrences"))? {
        Term::Literal(literal) => Some(literal.value().to_owned()),
        other => Some(other.to_string()),
    }
}

/// Each census finding as the path it names, the record it is about, and the
/// count it carries.
fn counted(findings: &[Quad]) -> Vec<(String, String, Option<String>)> {
    let mut rows: Vec<(String, String, Option<String>)> = censuses(findings)
        .iter()
        .map(|annotation| {
            let target = node(findings, annotation, &format!("{OA}hasTarget"));
            let selector = node(findings, &target, &format!("{OA}hasSelector"));
            (
                says(findings, annotation, &format!("{SH}value")),
                says(findings, &selector, RDF_VALUE),
                count(findings, annotation),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// How many nodes below the record this finding's record selector is refined
/// onto, where an empty refinement in a row above could as well have been a
/// selector carrying two.
fn refinements(findings: &[Quad], annotation: &str) -> usize {
    let target = node(findings, annotation, &format!("{OA}hasTarget"));
    let selector = node(findings, &target, &format!("{OA}hasSelector"));
    findings
        .iter()
        .filter(|q| q.subject.to_string() == selector)
        .filter(|q| q.predicate.as_str() == format!("{OA}refinedBy"))
        .count()
}

/// Where a finding is addressed, as one path: the record's own selector, and
/// the steps below it the finding is refined onto.
fn address(findings: &[Quad], annotation: &str) -> String {
    let target = node(findings, annotation, &format!("{OA}hasTarget"));
    let selector = node(findings, &target, &format!("{OA}hasSelector"));
    let refinement = node(findings, &selector, &format!("{OA}refinedBy"));
    let record = says(findings, &selector, RDF_VALUE);
    match says(findings, &refinement, RDF_VALUE) {
        below if below.is_empty() => record,
        below => format!("{record}/{below}"),
    }
}

/// Where each schema violation is addressed, the record's schema and the
/// document's alike.
fn violations(findings: &[Quad]) -> BTreeSet<String> {
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{SH}resultSeverity"))
        .filter(
            |q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == format!("{SH}Violation")),
        )
        .map(|q| address(findings, &q.subject.to_string()))
        .collect()
}

/// The paths a census named, each once however many records named it.
fn paths(findings: &[Quad]) -> BTreeSet<String> {
    census(findings)
        .into_iter()
        .map(|(path, _, _)| path)
        .collect()
}

fn annotations(findings: &[Quad]) -> usize {
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(
            |q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == format!("{OA}Annotation")),
        )
        .count()
}

/// Every finding as what it says and where it says it: its body, its
/// severity, the path it names where it names one, the record it is about,
/// and the node below that record it selects. Only a finding read from the
/// accounting names a path, so striking the accounting out leaves exactly the
/// rows whose path is empty.
fn said(findings: &[Quad]) -> Vec<(String, String, String, String, String)> {
    let mut rows: Vec<(String, String, String, String, String)> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(
            |q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == format!("{OA}Annotation")),
        )
        .map(|q| q.subject.to_string())
        .map(|annotation| {
            let target = node(findings, &annotation, &format!("{OA}hasTarget"));
            let selector = node(findings, &target, &format!("{OA}hasSelector"));
            let refinement = node(findings, &selector, &format!("{OA}refinedBy"));
            (
                says(findings, &annotation, &format!("{OA}hasBody")),
                says(findings, &annotation, &format!("{SH}resultSeverity")),
                says(findings, &annotation, &format!("{SH}value")),
                says(findings, &selector, RDF_VALUE),
                says(findings, &refinement, RDF_VALUE),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// The tiny adapter with the accounting struck out of its crate, which is
/// every adapter that exists.
struct Unaccounted {
    directory: DirectoryResolver,
}

impl Unaccounted {
    fn new() -> Self {
        Self { directory: tiny() }
    }
}

impl Resolver for Unaccounted {
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

/// The tiny adapter with its accounting file replaced, so an accounting can
/// be unparseable, empty, or short of the one entry a case turns on without
/// any of those being committed.
struct Accounting {
    directory: DirectoryResolver,
    body: String,
}

impl Accounting {
    fn of(body: &str) -> Self {
        Self {
            directory: tiny(),
            body: body.to_owned(),
        }
    }

    /// The committed accounting with the entry for one path taken out.
    fn without(path: &str) -> Self {
        let resolver = tiny();
        let iri = format!("{}{ACCOUNTING}", resolver.root());
        let text = String::from_utf8(resolver.read(&iri).expect("the accounting")).expect("utf-8");
        let entry = format!("bridge:sourcePath \"{path}\"");
        let kept: Vec<&str> = text
            .split("\n\n")
            .filter(|block| !block.contains(&entry))
            .collect();
        assert_eq!(
            text.split("\n\n").count() - kept.len(),
            1,
            "the accounting holds one entry for {path}"
        );
        Self {
            directory: resolver,
            body: kept.join("\n\n"),
        }
    }
}

impl Resolver for Accounting {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        if iri.ends_with(ACCOUNTING) {
            return Ok(self.body.as_bytes().to_vec());
        }
        self.directory.read(iri)
    }
}

/// The tiny adapter naming an accounting no file answers to, which is a crate
/// that says what it accounts for and cannot show it.
struct Missing {
    directory: DirectoryResolver,
}

const UNWRITTEN: &str = "vocab/an-accounting-nobody-wrote.ttl";

impl Resolver for Missing {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("ro-crate-metadata.json") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        assert_eq!(
            text.matches(ACCOUNTING).count(),
            2,
            "the crate names an accounting, and gives the file its media type"
        );
        Ok(text.replace(ACCOUNTING, UNWRITTEN).into_bytes())
    }
}

#[test]
fn leaves_a_crate_that_names_no_accounting_the_findings_it_has_today() {
    let unaccounted = Subject::of(Unaccounted::new());
    for input in ["two.xml", "order.xml", "every-verdict.xml"] {
        let named_no_path: Vec<(String, String, String, String, String)> =
            said(&on_disk(input).findings)
                .into_iter()
                .filter(|(_, _, path, _, _)| path.is_empty())
                .collect();
        assert_eq!(
            said(&unaccounted.findings(input)),
            named_no_path,
            "{input} through a crate naming no accounting"
        );
    }
    assert_eq!(
        annotations(&unaccounted.findings("unaccounted-child.xml")),
        0,
        "a path no entry accounts for draws nothing where there is no accounting"
    );

    assert_eq!(
        paths(&on_disk("unaccounted-child.xml").findings)
            .into_iter()
            .collect::<Vec<String>>(),
        ["/item/novelty"],
        "the accounting is what the census is read from"
    );
}

#[test]
fn reports_a_path_the_accounting_omits_at_its_first_occurrence_in_the_record() {
    let findings = on_disk("unaccounted-child.xml").findings;
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
    let found = on_disk("unaccounted-five-times.xml").findings;
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
        counted(&on_disk("unaccounted-twice-then-once.xml").findings),
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
        census(&on_disk("unaccounted-in-each-record.xml").findings),
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
        census(&on_disk("unaccounted-attribute.xml").findings),
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
    let found = on_disk("unaccounted-under-a-repeated-parent.xml").findings;
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
    let found = on_disk("unaccounted-under-a-repeated-parent.xml").findings;
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
        address(&found, &deep),
        format!("{label}/deep[1]"),
        "a schema finding and a census finding count a record's elements in walks of their own"
    );
}

/// A step of a path in the namespaced fixture's namespace, as the lift writes
/// one where no prefix can be bound.
fn step(local: &str) -> String {
    format!("*[local-name()='{local}' and namespace-uri()='{CATALOG}']")
}

#[test]
fn writes_a_namespaced_path_as_the_lift_writes_a_step_of_a_record_s_own_address() {
    let record = format!("/{}/{}[1]", step("catalog"), step("item"));
    let bogus = format!("/{}/{}", step("item"), step("bogus"));
    let within = format!("{}[1]", step("bogus"));
    assert_eq!(
        census(&on_disk("namespaced.xml").findings),
        [(bogus.clone(), record.clone(), within.clone())],
        "the accounting names both colours of the element, and neither is reported"
    );

    // The element carries a namespaced colour and an unnamespaced one. A last
    // step of "@colour" would make them one path, and the entry left standing
    // would silence the one struck out.
    let namespaced = format!("{bogus}/@{}", step("colour"));
    assert_eq!(
        census(&findings(
            &Accounting::without(&namespaced),
            "namespaced.xml"
        )),
        [
            (bogus, record.clone(), within.clone()),
            (namespaced, record, within)
        ]
    );
}

/// The paths of `every-verdict.xml`'s record, each with the step below the
/// record a finding about it selects. An attribute of the record element
/// selects nothing below the record: the target's selector already names the
/// element it stands on.
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
    assert_eq!(census(&on_disk("every-verdict.xml").findings), Vec::new());

    for (path, within) in EVERY_VERDICT {
        let found = findings(&Accounting::without(path), "every-verdict.xml");
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

/// Neither Turtle nor anything else: an accounting that cannot be read at all.
const UNPARSEABLE: &str = "@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#\n";

/// An accounting begun and left empty, which accounts for nothing.
const NOTHING_ACCOUNTED: &str = "# no path of the catalog format is accounted for yet\n";

#[test]
fn refuses_an_unparseable_accounting_where_an_absent_one_is_read_in_silence() {
    assert_eq!(
        census(&findings(&Unaccounted::new(), "two.xml")),
        Vec::new()
    );

    let Err(refusal) = conversion(&Accounting::of(UNPARSEABLE), "two.xml") else {
        panic!("an accounting that is not Turtle is read as an accounting of nothing");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(ACCOUNTING), "{refusal}");
}

#[test]
fn refuses_an_accounting_the_crate_names_and_nothing_answers_to() {
    assert_eq!(
        census(&findings(&Unaccounted::new(), "two.xml")),
        Vec::new()
    );

    let missing = Missing { directory: tiny() };
    if conversion(&missing, "two.xml").is_ok() {
        panic!("a crate that says what it accounts for and cannot show it is read in silence");
    }
}

#[test]
fn accounts_for_no_path_with_an_empty_accounting_where_an_absent_one_accounts_for_every_path() {
    assert_eq!(
        census(&findings(&Unaccounted::new(), "two.xml")),
        Vec::new()
    );

    let reported = findings(&Accounting::of(NOTHING_ACCOUNTED), "two.xml");
    assert_eq!(
        paths(&reported).into_iter().collect::<Vec<String>>(),
        ["/item/@id", "/item/note", "/item/title"]
    );
    assert_eq!(
        census(&reported).len(),
        4,
        "three paths of the record with a title and a note, one of the record without"
    );
}

/// A path hung on a node that says nothing about what it is. Whether such a
/// file is well formed is the accounting's shapes' to say, and a Bridge reads
/// what it is handed.
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
        paths(&findings(&Accounting::of(NO_PATH_ENTRY), "two.xml"))
            .into_iter()
            .collect::<Vec<String>>(),
        ["/item/@id", "/item/note", "/item/title"],
        "the path the file names is the one it would have silenced"
    );
}

#[test]
fn refuses_a_source_path_that_is_no_literal_where_dropping_it_would_report_the_path() {
    let Err(refusal) = conversion(&Accounting::of(A_PATH_THAT_IS_NO_LITERAL), "two.xml") else {
        panic!("a path that is not a string is read past in silence");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains(ACCOUNTING), "{refusal}");
}

#[test]
fn names_the_census_body_the_specification_fixed_and_no_other() {
    let findings = on_disk("unaccounted-child.xml").findings;
    let bodies: Vec<String> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{OA}hasBody"))
        .map(|q| q.object.to_string())
        .filter(|body| body.contains(BRIDGE))
        .collect();
    assert_eq!(bodies, [format!("<{PATH_NOT_ACCOUNTED}>")]);
}
