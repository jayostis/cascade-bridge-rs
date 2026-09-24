#![allow(dead_code)]

use crate::load::load_adapter;
use crate::run::{convert, prepare, Conversion, Source};
use crate::{DirectoryResolver, Resolver, Result};
use oxrdf::{Quad, Term};
use std::path::PathBuf;

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

pub fn vocabularies_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-vocabularies")
}

pub fn tiny() -> DirectoryResolver {
    DirectoryResolver::new(tiny_directory()).expect("resolver")
}

pub fn tiny_with_vocabularies() -> DirectoryResolver {
    tiny()
        .with_vocabularies(vocabularies_directory())
        .expect("the vocabularies directory")
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
            xml: &xml,
        },
    )
}

pub fn converted(resolver: &dyn Resolver, input: &str) -> Conversion {
    conversion(resolver, input).expect("conversion")
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

enum Edit {
    Replace {
        file: String,
        from: String,
        to: String,
        times: Option<usize>,
    },
    Whole {
        file: String,
        body: String,
    },
}

/// An adapter with files rewritten as they are read; a file is named by the end of its IRI.
pub struct Variant {
    directory: DirectoryResolver,
    edits: Vec<Edit>,
}

impl Variant {
    pub fn of(directory: DirectoryResolver) -> Self {
        Self {
            directory,
            edits: Vec::new(),
        }
    }

    /// Every `from` in the file becomes `to`; a file without one is a guard
    /// that failed.
    pub fn replacing(mut self, file: &str, from: &str, to: impl Into<String>) -> Self {
        self.edits.push(Edit::Replace {
            file: file.to_owned(),
            from: from.to_owned(),
            to: to.into(),
            times: None,
        });
        self
    }

    pub fn replacing_exactly(
        mut self,
        file: &str,
        from: &str,
        to: impl Into<String>,
        times: usize,
    ) -> Self {
        self.edits.push(Edit::Replace {
            file: file.to_owned(),
            from: from.to_owned(),
            to: to.into(),
            times: Some(times),
        });
        self
    }

    pub fn with(mut self, file: &str, body: impl Into<String>) -> Self {
        self.edits.push(Edit::Whole {
            file: file.to_owned(),
            body: body.into(),
        });
        self
    }
}

impl Resolver for Variant {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn vocabularies(&self) -> Option<&str> {
        self.directory.vocabularies()
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        self.edited(iri, || self.directory.read(iri))
    }

    fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
        self.edited(iri, || self.directory.read_vocabulary(iri))
    }
}

impl Variant {
    fn edited(&self, iri: &str, on_disk: impl Fn() -> Result<Vec<u8>>) -> Result<Vec<u8>> {
        let mut text: Option<String> = None;
        for edit in &self.edits {
            match edit {
                Edit::Whole { file, body } if iri.ends_with(file.as_str()) => {
                    // Served only where the directory would read a file: it may
                    // be missing there, and a replaced file often is, but a
                    // refusal of the path stands.
                    if let Err(refused) = on_disk() {
                        let missing = refused.to_string();
                        if !missing.contains("(os error 2)") && !missing.contains("(os error 3)") {
                            return Err(refused);
                        }
                    }
                    text = Some(body.clone());
                }
                Edit::Replace {
                    file,
                    from,
                    to,
                    times,
                } if iri.ends_with(file.as_str()) => {
                    let current = match text.take() {
                        Some(current) => current,
                        None => String::from_utf8(on_disk()?).expect("utf-8"),
                    };
                    let found = current.matches(from.as_str()).count();
                    match times {
                        Some(times) => assert_eq!(found, *times, "{file} carries {from}"),
                        None => assert!(found > 0, "{file} does not carry {from}"),
                    }
                    text = Some(current.replace(from.as_str(), to));
                }
                _ => {}
            }
        }
        match text {
            Some(text) => Ok(text.into_bytes()),
            None => on_disk(),
        }
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
