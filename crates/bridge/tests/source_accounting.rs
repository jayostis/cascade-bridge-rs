// An adapter accounts for each path of its source, and a Bridge reports the
// paths a record carries that the accounting says nothing about: one finding
// per distinct path per record, addressed at that path's first occurrence.
// This is the only finding a Bridge invents that no query wrote.
//
// A crate naming no accounting gets no census, which is every adapter that
// exists, so the whole of it has to be additive.
//
// No XPath evaluator is a dependency of this crate, so these assert the path
// a census names and the address it is written at rather than what evaluating
// either selects.
use cascade_bridge::{
    convert, load_adapter, prepare, Conversion, DirectoryResolver, Resolver, Source,
};
use oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use oxrdf::{Dataset, Quad, Term};
use oxrdfio::{RdfFormat, RdfParser};
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

/// The whole run, from the crate to the findings, as a result: an accounting
/// may be refused at any stage of it, and which stage is not this crate's to
/// say.
fn conversion(resolver: &dyn Resolver, input: &str) -> cascade_bridge::Result<Conversion> {
    let adapter = load_adapter(resolver)?;
    let prepared = prepare(&adapter, resolver)?;
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    let xml = resolver.read(&iri)?;
    convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            xml: &xml,
        },
    )
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

/// RDFC-1.0 canonical N-Quads: two graphs are isomorphic exactly when these
/// are equal, which is how the harness compares a run's findings with the
/// findings a fixture expects of it.
fn canonical(quads: impl IntoIterator<Item = Quad>) -> BTreeSet<String> {
    let mut dataset = Dataset::new();
    for quad in quads {
        dataset.insert(&quad);
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
        hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
    });
    dataset.iter().map(|quad| quad.to_string()).collect()
}

/// The findings a committed fixture expects of an input, parsed where it
/// stands so its relative source resolves to the input's own IRI.
fn expected(name: &str) -> BTreeSet<String> {
    let resolver = tiny();
    let iri = format!("{}fixtures/findings/{name}", resolver.root());
    let bytes = resolver.read(&iri).expect("the expected findings");
    canonical(
        RdfParser::from_format(RdfFormat::Turtle)
            .with_base_iri(&iri)
            .expect("base")
            .for_slice(&bytes)
            .map(|quad| quad.expect("turtle")),
    )
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

#[test]
fn leaves_a_crate_that_names_no_accounting_the_findings_it_has_today() {
    let unaccounted = Unaccounted::new();
    for (input, expects) in [("two.xml", "two.ttl"), ("order.xml", "order.ttl")] {
        assert_eq!(
            canonical(findings(&unaccounted, input)),
            expected(expects),
            "{input} through a crate naming no accounting"
        );
    }
    assert_eq!(
        annotations(&findings(&unaccounted, "unaccounted-child.xml")),
        0,
        "a path no entry accounts for draws nothing where there is no accounting"
    );

    assert_eq!(
        paths(&findings(&tiny(), "unaccounted-child.xml"))
            .into_iter()
            .collect::<Vec<String>>(),
        ["/catalog/item/novelty"],
        "the accounting is what the census is read from"
    );
}

#[test]
fn reports_a_path_the_accounting_omits_at_its_first_occurrence_in_the_record() {
    let findings = findings(&tiny(), "unaccounted-child.xml");
    assert_eq!(
        census(&findings),
        [(
            "/catalog/item/novelty".to_owned(),
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
    let target = node(&findings, &annotation, &format!("{OA}hasTarget"));
    assert!(
        says(&findings, &target, &format!("{OA}hasSource"))
            .ends_with("fixtures/in/unaccounted-child.xml"),
        "{findings:?}"
    );
}

#[test]
fn reports_a_path_a_record_carries_five_times_once_addressed_at_the_first() {
    assert_eq!(
        census(&findings(&tiny(), "unaccounted-five-times.xml")),
        [(
            "/catalog/item/novelty".to_owned(),
            "/catalog/item[1]".to_owned(),
            "novelty[1]".to_owned()
        )]
    );
}

#[test]
fn reports_a_path_two_records_carry_once_in_each() {
    assert_eq!(
        census(&findings(&tiny(), "unaccounted-in-each-record.xml")),
        [
            (
                "/catalog/item/novelty".to_owned(),
                "/catalog/item[1]".to_owned(),
                "novelty[1]".to_owned()
            ),
            (
                "/catalog/item/novelty".to_owned(),
                "/catalog/item[2]".to_owned(),
                "novelty[1]".to_owned()
            )
        ]
    );
}

#[test]
fn ends_an_attribute_s_path_in_the_attribute_s_own_name() {
    let findings = findings(&tiny(), "unaccounted-attribute.xml");
    let rows = census(&findings);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].0, "/catalog/item/@colour");
    assert_eq!(rows[0].1, "/catalog/item[1]");
}

/// A step of a path in the namespaced fixture's namespace, as the lift writes
/// one where no prefix can be bound.
fn step(local: &str) -> String {
    format!("*[local-name()='{local}' and namespace-uri()='{CATALOG}']")
}

#[test]
fn writes_a_namespaced_path_as_the_lift_writes_a_step_of_a_record_s_own_address() {
    let record = format!("/{}/{}", step("catalog"), step("item"));
    assert_eq!(
        census(&findings(&tiny(), "namespaced.xml")),
        [(
            format!("{record}/{}", step("bogus")),
            format!("{record}[1]"),
            format!("{}[1]", step("bogus"))
        )]
    );
}

/// The paths of `every-verdict.xml`'s record, one per verdict the accounting
/// can give. The record's own path is left out: whether a record carries its
/// own path is the specification's to settle.
const EVERY_VERDICT: [&str; 7] = [
    "/catalog/item/@id",
    "/catalog/item/@internal",
    "/catalog/item/title",
    "/catalog/item/supersededTitle",
    "/catalog/item/summary",
    "/catalog/item/checked",
    "/catalog/item/note",
];

#[test]
fn reports_nothing_for_a_record_every_path_of_which_has_an_entry_whatever_its_verdict() {
    assert_eq!(census(&findings(&tiny(), "every-verdict.xml")), Vec::new());

    for path in EVERY_VERDICT {
        assert_eq!(
            paths(&findings(&Accounting::without(path), "every-verdict.xml"))
                .into_iter()
                .collect::<Vec<String>>(),
            [path.to_owned()],
            "the entry for {path} is what silences it"
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
fn accounts_for_no_path_with_an_empty_accounting_where_an_absent_one_accounts_for_every_path() {
    assert_eq!(
        census(&findings(&Unaccounted::new(), "two.xml")),
        Vec::new()
    );

    let reported = paths(&findings(&Accounting::of(NOTHING_ACCOUNTED), "two.xml"));
    for path in [
        "/catalog/item/@id",
        "/catalog/item/title",
        "/catalog/item/note",
    ] {
        assert!(
            reported.contains(path),
            "an accounting of nothing accounts for {path}: {reported:?}"
        );
    }
    // The record's own path is the specification's to settle, so it is neither
    // required here nor refused.
    for path in &reported {
        assert!(
            path == "/catalog/item" || path.starts_with("/catalog/item/"),
            "a path the record does not carry: {reported:?}"
        );
    }
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
