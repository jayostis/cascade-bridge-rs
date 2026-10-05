#![allow(dead_code)]

use crate::library::{Files, Named};
use crate::load::load_adapter;
use crate::records::Supplied;
use crate::resolver::{Maps, Resolver};
use crate::run::{convert, prepare, Conversion, Source};
use crate::Result;
use oxrdf::{Quad, Term};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn fixture(path: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

pub(crate) fn fixture_names(directory: &str) -> Vec<String> {
    fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(directory))
        .expect("the fixtures")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

pub const OA: &str = "http://www.w3.org/ns/oa#";
pub const SH: &str = "http://www.w3.org/ns/shacl#";
pub const BRIDGE: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#";
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The namespace the namespaced fixture is written in.
pub const CATALOG: &str = "urn:example:catalog";
pub const NOTE_GAP: &str = "urn:example:catalog#noteHasNoTerm";
pub const PATH_NOT_ACCOUNTED: &str =
    "https://ns.cascadeprotocol.org/bridge/v1-draft#pathNotAccounted";

pub const CRATE: &str = "ro-crate-metadata.json";
pub const ACCOUNTING: &str = "vocab/catalog-accounting.ttl";
pub const GAP_SCHEME: &str = "vocab/catalog-gaps.ttl";
pub const CONCEPT_MAP: &str = "vocab/catalog-statuses.ttl";

pub fn tiny_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter")
}

pub const TINY: &str = "https://example.org/tiny-adapter/";
pub const TINY_JSON: &str = "https://example.org/tiny-json-adapter/";
pub const VOCABULARIES: &str = "https://example.org/tiny-vocabularies/";

fn files_under(root: &Path, directory: &Path, files: &mut Files) {
    for entry in fs::read_dir(directory).expect("a directory of the fixture") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            files_under(root, &path, files);
            continue;
        }
        let key = path
            .strip_prefix(root)
            .expect("under the fixture")
            .to_string_lossy()
            .replace('\\', "/");
        files.insert(key, fs::read(&path).expect("a file of the fixture"));
    }
}

pub fn files(directory: &str) -> Files {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(directory);
    let mut files = Files::new();
    files_under(&root, &root, &mut files);
    files
}

/// An adapter's files, and a vocabulary's where one is given, each under its IRI.
pub struct Fixture {
    iri: String,
    files: Files,
    vocabulary: Option<(String, Files)>,
}

impl Fixture {
    fn maps(&self) -> Maps<'_> {
        Maps {
            adapter: Named {
                iri: &self.iri,
                files: &self.files,
            },
            vocabulary: self
                .vocabulary
                .as_ref()
                .map(|(iri, files)| Named { iri, files }),
        }
    }

    pub fn at(self, iri: &str) -> Self {
        Self {
            iri: iri.to_owned(),
            ..self
        }
    }
}

impl Resolver for Fixture {
    fn root(&self) -> &str {
        &self.iri
    }

    fn vocabularies(&self) -> Option<&str> {
        self.vocabulary.as_ref().map(|(iri, _)| iri.as_str())
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        self.maps().read(iri)
    }

    fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
        self.maps().read_vocabulary(iri)
    }
}

pub fn tiny() -> Fixture {
    Fixture {
        iri: TINY.to_owned(),
        files: files("tiny-adapter"),
        vocabulary: None,
    }
}

pub fn tiny_json() -> Fixture {
    Fixture {
        iri: TINY_JSON.to_owned(),
        files: files("tiny-json-adapter"),
        vocabulary: None,
    }
}

pub fn tiny_with_vocabularies() -> Fixture {
    Fixture {
        vocabulary: Some((VOCABULARIES.to_owned(), files("tiny-vocabularies"))),
        ..tiny()
    }
}

/// The whole run, from the crate to the conversion: which stage refuses is not a test's
/// to say.
pub fn conversion(resolver: &dyn Resolver, input: &str) -> Result<Conversion> {
    let adapter = load_adapter(resolver)?;
    let prepared = prepare(&adapter, resolver)?;
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    let xml = resolver.read(&iri)?;
    convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            bytes: &xml,
            facts: None,
        },
    )
}

pub fn converted(resolver: &dyn Resolver, input: &str) -> Conversion {
    conversion(resolver, input).expect("conversion")
}

/// The facts the tiny adapter's passing entry supplies.
pub const FACTS: &str = "fixtures/facts/catalog.ttl";

pub fn converted_with_facts(resolver: &dyn Resolver, input: &str) -> Conversion {
    let adapter = load_adapter(resolver).expect("adapter");
    let prepared = prepare(&adapter, resolver).expect("prepared");
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    let xml = resolver.read(&iri).expect("the input");
    let facts_iri = format!("{}{FACTS}", resolver.root());
    let turtle = resolver.read(&facts_iri).expect("the facts");
    convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            bytes: &xml,
            facts: Some(Supplied {
                iri: &facts_iri,
                turtle: &turtle,
            }),
        },
    )
    .expect("conversion")
}

/// The tiny adapter, each item's title written as the item's version, which arrives with its id.
pub fn versioned() -> Variant {
    Variant::of(tiny()).with(
        "mapping/item.rq",
        r#"
PREFIX rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX fx:     <http://sparql.xyz/facade-x/ns/>
PREFIX xyz:    <http://sparql.xyz/facade-x/data/>
PREFIX prov:   <http://www.w3.org/ns/prov#>
PREFIX pav:    <http://purl.org/pav/>
PREFIX bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#>
PREFIX ex:     <urn:example:catalog#>

CONSTRUCT {
  ?s a ex:Item .
  ?v prov:specializationOf ?s ; ex:title ?title .
  [] bridge:arrivedAs ?v ; pav:version ?id .
}
WHERE {
  ?item a fx:root, xyz:item ; xyz:id ?id .
  BIND(IRI(CONCAT("urn:example:item:", ?id)) AS ?s)
  BIND(IRI(CONCAT("urn:example:draft:", ?id)) AS ?v)
  OPTIONAL { ?item ?slot ?t . ?t a xyz:title ; rdf:_1 ?title . }
}
"#,
    )
}

pub fn findings(resolver: &dyn Resolver, input: &str) -> Vec<Quad> {
    converted(resolver, input).findings
}

pub fn objects(quads: &[Quad], subject: &str, predicate: &str) -> Vec<Term> {
    quads
        .iter()
        .filter(|q| q.subject.to_string() == subject && q.predicate.as_str() == predicate)
        .map(|q| q.object.clone())
        .collect()
}

/// A second is a defect of the run, never an absence.
pub fn one(quads: &[Quad], subject: &str, predicate: &str) -> Option<Term> {
    let mut all = objects(quads, subject, predicate);
    assert!(
        all.len() < 2,
        "{subject} carries {} objects for {predicate}: {all:?}",
        all.len()
    );
    all.pop()
}

pub fn node(quads: &[Quad], subject: &str, predicate: &str) -> String {
    one(quads, subject, predicate)
        .map(|term| term.to_string())
        .unwrap_or_default()
}

/// A term as an address or a body is read: an IRI or a literal by what it
/// says, anything else by how N-Triples writes it.
pub fn written(term: Term) -> String {
    match term {
        Term::NamedNode(named) => named.as_str().to_owned(),
        Term::Literal(literal) => literal.value().to_owned(),
        other => other.to_string(),
    }
}

pub fn says(quads: &[Quad], subject: &str, predicate: &str) -> String {
    one(quads, subject, predicate)
        .map(written)
        .unwrap_or_default()
}

pub fn count(findings: &[Quad], annotation: &str) -> Option<String> {
    one(findings, annotation, &format!("{BRIDGE}occurrences")).map(written)
}

pub fn annotations(findings: &[Quad]) -> Vec<String> {
    let annotation = format!("{OA}Annotation");
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == annotation))
        .map(|q| q.subject.to_string())
        .collect()
}

/// The record the target's selector names, and the step it is refined onto, empty
/// where it is refined onto none.
pub fn address(findings: &[Quad], annotation: &str) -> (String, String) {
    let target = node(findings, annotation, &format!("{OA}hasTarget"));
    let selector = node(findings, &target, &format!("{OA}hasSelector"));
    let refinement = node(findings, &selector, &format!("{OA}refinedBy"));
    (
        says(findings, &selector, RDF_VALUE),
        says(findings, &refinement, RDF_VALUE),
    )
}

/// A step in the namespaced fixture's namespace, as the lift writes one where
/// no prefix can be bound.
pub fn step(local: &str) -> String {
    format!("*[local-name()='{local}' and namespace-uri()='{CATALOG}']")
}

/// An adapter with files rewritten; a file is named by the end of its path.
pub struct Variant {
    fixture: Fixture,
}

impl Variant {
    pub fn of(fixture: Fixture) -> Self {
        Self { fixture }
    }

    fn named<'a>(&'a mut self, file: &'a str) -> impl Iterator<Item = &'a mut Vec<u8>> + 'a {
        let Fixture {
            files, vocabulary, ..
        } = &mut self.fixture;
        files
            .iter_mut()
            .chain(
                vocabulary
                    .iter_mut()
                    .flat_map(|(_, files)| files.iter_mut()),
            )
            .filter(move |(path, _)| path.ends_with(file))
            .map(|(_, bytes)| bytes)
    }

    fn replaced(mut self, file: &str, from: &str, to: &str, times: Option<usize>) -> Self {
        let mut edited = 0;
        for bytes in self.named(file) {
            let text = String::from_utf8(std::mem::take(bytes)).expect("utf-8");
            let found = text.matches(from).count();
            match times {
                Some(times) => assert_eq!(found, times, "{file} carries {from}"),
                None => assert!(found > 0, "{file} does not carry {from}"),
            }
            *bytes = text.replace(from, to).into_bytes();
            edited += 1;
        }
        assert!(edited > 0, "no file of the fixture is {file}");
        self
    }

    /// Every `from` in the file becomes `to`; a file without one is a guard
    /// that failed.
    pub fn replacing(self, file: &str, from: &str, to: impl Into<String>) -> Self {
        self.replaced(file, from, &to.into(), None)
    }

    pub fn replacing_exactly(
        self,
        file: &str,
        from: &str,
        to: impl Into<String>,
        times: usize,
    ) -> Self {
        self.replaced(file, from, &to.into(), Some(times))
    }

    /// The file's whole body, added to the adapter where no file is named so.
    pub fn with(mut self, file: &str, body: impl Into<String>) -> Self {
        let body = body.into().into_bytes();
        let mut edited = 0;
        for bytes in self.named(file) {
            bytes.clone_from(&body);
            edited += 1;
        }
        if edited == 0 {
            self.fixture.files.insert(file.to_owned(), body);
        }
        self
    }
}

impl Resolver for Variant {
    fn root(&self) -> &str {
        self.fixture.root()
    }

    fn vocabularies(&self) -> Option<&str> {
        self.fixture.vocabularies()
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        self.fixture.read(iri)
    }

    fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
        self.fixture.read_vocabulary(iri)
    }
}

pub fn with_accounting(body: &str) -> Variant {
    Variant::of(tiny()).with(ACCOUNTING, body)
}

pub fn committed() -> Vec<String> {
    let mut named: Vec<String> = std::fs::read_dir(tiny_directory().join("fixtures/in"))
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

/// Every finding at sh:Violation, as its body and where it is addressed.
pub fn violations(findings: &[Quad]) -> Vec<(String, String, String)> {
    let violation = format!("{SH}Violation");
    let mut rows: Vec<(String, String, String)> = annotations(findings)
        .into_iter()
        .filter(|annotation| {
            matches!(
                one(findings, annotation, &format!("{SH}resultSeverity")),
                Some(Term::NamedNode(n)) if n.as_str() == violation
            )
        })
        .map(|annotation| {
            let (record, within) = address(findings, &annotation);
            (
                says(findings, &annotation, &format!("{OA}hasBody")),
                record,
                within,
            )
        })
        .collect();
    rows.sort();
    rows
}

/// The tiny adapter with the accounting struck out of its crate, checked by loading it.
pub fn unaccounted() -> Variant {
    let directory = tiny();
    let iri = format!("{}{CRATE}", directory.root());
    let text = String::from_utf8(directory.read(&iri).expect("the crate")).expect("utf-8");
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| !line.contains("bridge:sourceAccounting"))
        .collect();
    assert_eq!(
        text.lines().count() - kept.len(),
        2,
        "the crate names an accounting, in its context and on its root entity"
    );
    let struck = Variant::of(directory).with(CRATE, kept.join("\n"));
    let adapter = load_adapter(&struck).expect("the crate without its accounting");
    let committed = load_adapter(&tiny()).expect("the committed crate");
    assert_eq!(adapter.source_accounting, None);
    assert_eq!(adapter.gap_scheme, committed.gap_scheme);
    assert_eq!(adapter.mappings, committed.mappings);
    assert_eq!(adapter.findings_queries, committed.findings_queries);
    struck
}

pub const ACCOUNTING_PREAMBLE: &str = "@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#> .\n@prefix ex:     <urn:example:catalog#> .\n";

pub const GAPS_PREAMBLE: &str = "@prefix skos:   <http://www.w3.org/2004/02/skos/core#> .\n@prefix sh:     <http://www.w3.org/ns/shacl#> .\n@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#> .\n@prefix ex:     <urn:example:catalog#> .\n";

pub fn entry(path: &str, verdict: &str, declarations: &[&str]) -> String {
    let said: String = declarations
        .iter()
        .map(|declaration| format!(" ;\n   {declaration}"))
        .collect();
    format!(
        "\n[] a bridge:PathEntry ;\n   bridge:sourcePath \"{path}\" ;\n   bridge:verdict bridge:{verdict}{said} .\n"
    )
}

pub fn accounting(entries: &[String]) -> String {
    format!("{ACCOUNTING_PREAMBLE}{}", entries.concat())
}

pub fn concept(name: &str, kind: &str, severity: Option<&str>) -> String {
    let declared = match severity {
        Some(severity) => format!(" ;\n  sh:resultSeverity sh:{severity}"),
        None => String::new(),
    };
    format!("\nex:{name} a skos:Concept ;\n  skos:inScheme ex:gaps ;\n  skos:broader bridge:{kind}{declared} .\n")
}

pub fn gap_scheme(concepts: &[String]) -> String {
    format!(
        "{GAPS_PREAMBLE}\nex:gaps a skos:ConceptScheme .\n{}",
        concepts.concat()
    )
}
