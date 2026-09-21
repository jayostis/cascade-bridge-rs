// A finding's address is an XPath, and this Bridge follows it. An address that
// selects no node, or several, names nothing the finding can be about, so the
// Bridge reports it beside the adapter's own finding and produces the graph all
// the same: verification reports, and never refuses.
use cascade_bridge::{
    canonical_lines, convert, load_adapter, prepare, Conversion, DirectoryResolver, Resolver,
    Source,
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
const NOTE: &str = "rdf:value \"note\"";

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
    Variant {
        directory: directory(),
        edits: vec![(file, from, to.to_owned())],
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

/// Every address this Bridge could not follow, as the record it was about and
/// the address as it was written.
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
            let record = objects(findings, &selector, RDF_VALUE);
            let written = objects(findings, &annotation, SH_VALUE);
            (
                record.first().map(text).unwrap_or_default(),
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
        [("/catalog/item[1]".to_owned(), "nowhere".to_owned())]
    );
    assert_eq!(graph(&strayed), graph(&plain));
}

#[test]
fn reports_an_address_that_selects_more_than_one_node() {
    let several = run(&variant(NOTE_QUERY, NOTE, "rdf:value \"*\""), "two.xml");
    assert_eq!(
        reports(&several.findings),
        [("/catalog/item[1]".to_owned(), "*".to_owned())],
        "the record holds a title and a note, and an address selects one node"
    );
}

#[test]
fn reports_an_address_that_is_no_xpath_at_all() {
    let nonsense = run(&variant(NOTE_QUERY, NOTE, "rdf:value \"(((\""), "two.xml");
    assert_eq!(
        reports(&nonsense.findings),
        [("/catalog/item[1]".to_owned(), "(((".to_owned())]
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
            ("/catalog/item[1]".to_owned(), "nowhere".to_owned()),
            ("/catalog/item[2]".to_owned(), "nowhere".to_owned())
        ],
        "the first record's two notes share one address, and one report"
    );
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
