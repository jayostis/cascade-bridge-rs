use crate::earl::{earl_report, ReportSubject};
use crate::error::{Error, Result};
use crate::harness::{run_manifest, RunOptions, OFFERED_PROFILES};
use crate::load::{self, adapter_of, instances, load_adapter, CRATE};
use crate::rdf::{serialise, serialise_at};
use crate::records::Supplied;
use crate::resolver::{key, Maps};
use crate::run::{self, prepare, Prepared, Source};
use crate::terms::{
    BRIDGE_ADAPTER, BRIDGE_CASCADE_VOCABULARY_PIN, BRIDGE_CRATE_FILE, BRIDGE_ENVELOPE,
    BRIDGE_LOAD_FILE, BRIDGE_SOURCE_MEDIA_TYPE, BRIDGE_TEST_MANIFEST, BRIDGE_VOCABULARY_FILE,
    RDF_TYPE, SCHEMA_IDENTIFIER, SCHEMA_MEDIA_OBJECT, SCHEMA_VERSION,
};
use crate::vocabulary::{require_vocabularies, unvalidated_output};
use oxrdf::{GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term, TermRef};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

pub use crate::harness::Outcome;
pub use crate::rdf::GraphFormat as Format;

pub const NAME: &str = "Cascade Bridge for Rust";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const BRIDGE: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#";

/// Each file by its path, as the crate or a `bridge:vocabularyFile` writes it.
pub type Files = BTreeMap<String, Vec<u8>>;

/// A map of files and the IRI its paths resolve against, ending in "/".
#[derive(Clone, Copy)]
pub struct Named<'a> {
    pub iri: &'a str,
    pub files: &'a Files,
}

#[derive(Clone, Copy)]
pub struct Facts<'a> {
    pub iri: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Clone, Copy)]
pub struct Document<'a> {
    pub iri: &'a str,
    pub bytes: &'a [u8],
    pub envelope: Option<&'a str>,
    pub facts: Option<Facts<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    /// The crate's root entity.
    pub iri: String,
    pub identifier: Option<String>,
    pub version: Option<String>,
    pub source_media_type: Option<String>,
    pub envelopes: Vec<String>,
    pub load_files: Vec<String>,
    pub crate_files: Vec<String>,
    pub vocabulary_pin: Option<String>,
    pub vocabulary_files: Vec<String>,
}

pub fn describe(adapter_iri: &str, metadata: &[u8]) -> Result<Description> {
    let adapter = adapter_of(metadata, adapter_iri)?;
    let named_by = |subject: &str| -> Vec<String> {
        let Ok(subject) = NamedNode::new(subject) else {
            return Vec::new();
        };
        adapter
            .graph
            .triples_for_subject(subject.as_ref())
            .filter(|triple| {
                triple.predicate.as_str().starts_with(BRIDGE)
                    && triple.predicate.as_str() != BRIDGE_TEST_MANIFEST
            })
            .filter_map(|triple| match triple.object {
                TermRef::NamedNode(node) => key(adapter_iri, node.as_str()),
                _ => None,
            })
            .collect()
    };
    let mut load_files: BTreeSet<String> = std::iter::once(adapter.root.as_str())
        .chain(
            adapter
                .envelopes
                .iter()
                .map(|envelope| envelope.iri.as_str()),
        )
        .flat_map(named_by)
        .collect();
    load_files.insert(CRATE.to_owned());
    let mut crate_files: BTreeSet<String> = instances(&adapter.graph, SCHEMA_MEDIA_OBJECT)?
        .into_iter()
        .filter_map(|file| match file {
            NamedOrBlankNode::NamedNode(node) => key(adapter_iri, node.as_str()),
            NamedOrBlankNode::BlankNode(_) => None,
        })
        .collect();
    crate_files.insert(CRATE.to_owned());
    Ok(Description {
        iri: adapter.root,
        identifier: adapter.identifier,
        version: adapter.version,
        source_media_type: adapter.source_media_type,
        envelopes: adapter
            .envelopes
            .into_iter()
            .map(|envelope| envelope.iri)
            .collect(),
        load_files: load_files.into_iter().collect(),
        crate_files: crate_files.into_iter().collect(),
        vocabulary_pin: adapter.cascade_vocabulary_pin,
        vocabulary_files: adapter.vocabulary_files,
    })
}

impl Description {
    pub fn graph(&self, format: Format) -> Result<Vec<u8>> {
        let prefixes = [
            ("schema", "http://schema.org/"),
            ("bridge", "https://ns.cascadeprotocol.org/bridge/v1-draft#"),
        ]
        .map(|(name, namespace)| (name.to_owned(), namespace.to_owned()));
        Ok(serialise(&described(self)?, format, &prefixes)?.into_bytes())
    }
}

/// What `describe` read, as the graph the specification's library cases compare.
fn described(description: &Description) -> Result<Vec<Quad>> {
    let adapter = NamedOrBlankNode::from(NamedNode::new(&description.iri)?);
    let literal = |value: &String| Term::from(Literal::new_simple_literal(value));
    let mut said: Vec<(&str, Term)> = vec![(RDF_TYPE, NamedNode::new(BRIDGE_ADAPTER)?.into())];
    if let Some(identifier) = &description.identifier {
        said.push((SCHEMA_IDENTIFIER, literal(identifier)));
    }
    if let Some(version) = &description.version {
        said.push((SCHEMA_VERSION, literal(version)));
    }
    if let Some(media_type) = &description.source_media_type {
        said.push((BRIDGE_SOURCE_MEDIA_TYPE, literal(media_type)));
    }
    for envelope in &description.envelopes {
        said.push((BRIDGE_ENVELOPE, NamedNode::new(envelope)?.into()));
    }
    if let Some(pin) = &description.vocabulary_pin {
        said.push((BRIDGE_CASCADE_VOCABULARY_PIN, NamedNode::new(pin)?.into()));
    }
    for file in &description.vocabulary_files {
        said.push((BRIDGE_VOCABULARY_FILE, literal(file)));
    }
    for file in &description.load_files {
        said.push((BRIDGE_LOAD_FILE, literal(file)));
    }
    for file in &description.crate_files {
        said.push((BRIDGE_CRATE_FILE, literal(file)));
    }
    said.into_iter()
        .map(|(predicate, object)| {
            Ok(Quad::new(
                adapter.clone(),
                NamedNode::new(predicate)?,
                object,
                GraphName::DefaultGraph,
            ))
        })
        .collect()
}

pub struct Loaded {
    adapter: load::Adapter,
    prepared: Prepared,
    unvalidated: Option<String>,
}

pub fn load(adapter: Named<'_>, vocabulary: Option<Named<'_>>) -> Result<Loaded> {
    let maps = Maps {
        adapter,
        vocabulary,
    };
    let loaded = load_adapter(&maps)?;
    let unoffered: Vec<&str> = loaded
        .required_profiles
        .iter()
        .map(String::as_str)
        .filter(|profile| !OFFERED_PROFILES.contains(profile))
        .collect();
    if !unoffered.is_empty() {
        return Err(Error::adapter(format!(
            "the adapter requires {}, which this Bridge does not offer",
            unoffered.join(", ")
        )));
    }
    let prepared = prepare(&loaded, &maps)?;
    Ok(Loaded {
        unvalidated: unvalidated_output(&loaded, &maps),
        adapter: loaded,
        prepared,
    })
}

impl Loaded {
    /// The crate's root entity.
    pub fn iri(&self) -> &str {
        &self.adapter.root
    }

    pub fn identifier(&self) -> Option<&str> {
        self.adapter.identifier.as_deref()
    }

    pub fn accepts(&self, document: &Document<'_>) -> Result<bool> {
        run::accepts(&self.prepared, source(document))
    }

    pub fn convert(&self, document: &Document<'_>) -> Result<Conversion> {
        let mut run = run::convert(&self.prepared, source(document))?;
        if self.unvalidated.is_some() {
            run.findings.clear();
        }
        Ok(Conversion {
            records: run.units,
            triples: run.triples(),
            finding_count: run.annotations(),
            detected: run.detected,
            unvalidated: self.unvalidated.clone(),
            prefixes: self.prepared.prefixes.clone(),
            findings_prefixes: self.prepared.findings_prefixes.clone(),
            quads: run.quads,
            findings: run.findings,
        })
    }
}

fn source<'a>(document: &Document<'a>) -> Source<'a> {
    Source {
        iri: document.iri,
        envelope: document.envelope,
        bytes: document.bytes,
        facts: document.facts.map(|facts| Supplied {
            iri: facts.iri,
            turtle: facts.bytes,
        }),
    }
}

pub struct Conversion {
    pub records: usize,
    pub triples: usize,
    pub finding_count: usize,
    /// The detect query's answer, where the adapter names one.
    pub detected: Option<bool>,
    /// Why the graph was not validated against the vocabulary's files, where it was not.
    pub unvalidated: Option<String>,
    prefixes: Vec<(String, String)>,
    findings_prefixes: Vec<(String, String)>,
    quads: Vec<Quad>,
    findings: Vec<Quad>,
}

impl Conversion {
    pub fn graph(&self, format: Format) -> Result<Vec<u8>> {
        Ok(serialise(&self.quads, format, &self.prefixes)?.into_bytes())
    }

    /// Every IRI the file standing at `relative_to` can name relative to itself is named that way.
    pub fn findings(&self, format: Format, relative_to: Option<&str>) -> Result<Vec<u8>> {
        Ok(
            serialise_at(&self.findings, format, &self.findings_prefixes, relative_to)?
                .into_bytes(),
        )
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TestOptions {
    pub datasets: bool,
}

#[derive(Debug, Clone)]
pub struct TestEntry {
    pub name: String,
    pub outcome: Outcome,
    pub description: String,
    pub elapsed: Duration,
}

#[derive(Debug, Clone)]
pub struct TestReport {
    pub earl: Vec<u8>,
    pub entries: Vec<TestEntry>,
    pub profiles: Vec<String>,
}

pub fn test(
    adapter: Named<'_>,
    vocabulary: Option<Named<'_>>,
    options: TestOptions,
) -> Result<TestReport> {
    let maps = Maps {
        adapter,
        vocabulary,
    };
    let loaded = load_adapter(&maps)?;
    require_vocabularies(&loaded, &maps)?;
    let results = run_manifest(
        &loaded,
        &maps,
        RunOptions {
            datasets: options.datasets,
        },
    )?;
    let subject = ReportSubject {
        iri: "https://github.com/jayostis/cascade-bridge-rs".to_owned(),
        name: NAME.to_owned(),
        version: VERSION.to_owned(),
    };
    Ok(TestReport {
        earl: earl_report(&results, &subject)?.into_bytes(),
        entries: results
            .into_iter()
            .map(|result| TestEntry {
                name: result.name,
                outcome: result.outcome,
                description: result.description,
                elapsed: result.elapsed,
            })
            .collect(),
        profiles: OFFERED_PROFILES.iter().map(|&p| p.to_owned()).collect(),
    })
}
