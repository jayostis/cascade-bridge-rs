use crate::accounting::{gap_scheme, Accounting};
use crate::annotation::{self, Record};
use crate::error::{Error, Result};
use crate::lift::{Lift, Paths, Unit};
use crate::load::{subject, value, values, Adapter};
use crate::query::{Form, Query};
use crate::rdf::FINDINGS_PREFIXES;
use crate::resolver::Resolver;
use crate::syntax::{Addresses, Syntax};
use crate::terms::{BRIDGE_STAMP_PREDICATE, SCHEMA_ENCODING_FORMAT};
use crate::validate::Schema;
use crate::vocabulary::Vocabulary;
use oxigraph::model::Quad;
use oxigraph::sparql::QueryResults;
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::{HashMap, HashSet};

struct Envelope {
    iri: String,
    document_root: Option<String>,
    document_schema: Option<Box<dyn Schema>>,
}

pub(crate) struct Prepared {
    pub(crate) syntax: Syntax,
    unit: String,
    mappings: Vec<Query>,
    findings_queries: Vec<Query>,
    detect: Option<Query>,
    tables: Vec<Quad>,
    /// Every name the mappings give a namespace, the first binding of a name winning.
    pub(crate) prefixes: Vec<(String, String)>,
    pub(crate) findings_prefixes: Vec<(String, String)>,
    envelopes: Vec<Envelope>,
    source_schema: Option<Box<dyn Schema>>,
    vocabulary: Option<Vocabulary>,
    accounting: Option<Accounting>,
    /// By gap concept IRI, for a findings query's annotation that declares no severity.
    gap_severities: HashMap<String, String>,
}

impl Prepared {
    fn named_envelope(&self, iri: &str) -> Result<&Envelope> {
        self.envelopes
            .iter()
            .find(|envelope| envelope.iri == iri)
            .ok_or_else(|| Error::msg(format!("the adapter declares no envelope {iri}")))
    }

    fn envelope_of(&self, document_root: Option<&str>) -> Option<&Envelope> {
        let root = document_root?;
        self.envelopes
            .iter()
            .find(|envelope| envelope.document_root.as_deref() == Some(root))
    }
}

pub(crate) struct Conversion {
    pub(crate) quads: Vec<Quad>,
    pub(crate) findings: Vec<Quad>,
    pub(crate) units: usize,
    pub(crate) detected: Option<bool>,
}

impl Conversion {
    pub(crate) fn annotations(&self) -> usize {
        annotation::annotations(&self.findings)
    }

    /// `quads` holds a triple constructed for every record once per record, the graph once.
    pub(crate) fn triples(&self) -> usize {
        self.quads.iter().collect::<HashSet<&Quad>>().len()
    }
}

pub(crate) struct Source<'a> {
    pub(crate) iri: &'a str,
    pub(crate) envelope: Option<&'a str>,
    pub(crate) bytes: &'a [u8],
}

/// The manifest's own: an entry's replaces it only where the harness compares.
fn stamps(adapter: &Adapter) -> Result<HashSet<String>> {
    Ok(values(
        &adapter.graph,
        &subject(&adapter.manifest)?,
        BRIDGE_STAMP_PREDICATE,
    )?
    .into_iter()
    .collect())
}

pub(crate) fn prepare(adapter: &Adapter, resolver: &dyn Resolver) -> Result<Prepared> {
    if adapter.mappings.is_empty() {
        return Err(Error::msg("the adapter names no bridge:mapping"));
    }
    let syntax = Syntax::of(adapter.source_media_type.as_deref())?;
    let unit = syntax.records(adapter)?;
    let tables = tables(adapter, resolver)?;
    let mappings = queries(resolver, &adapter.mappings, "mapping")?;
    let findings_queries = queries(resolver, &adapter.findings_queries, "findings query")?;
    let detect = adapter
        .detect_query
        .as_deref()
        .map(|iri| Query::read(resolver, iri, Form::Ask, "detect query"))
        .transpose()?;
    let source_schema = adapter
        .source_schema
        .as_deref()
        .map(|iri| syntax.schema(iri, resolver))
        .transpose()?;
    let envelopes = envelopes(adapter, syntax, resolver)?;
    let (scheme, gap_prefixes) = adapter
        .gap_scheme
        .as_deref()
        .map(|iri| gap_scheme(resolver, iri))
        .transpose()?
        .unwrap_or_default();
    let prefixes = prefixes(&mappings);
    let findings_prefixes = findings_prefixes(gap_prefixes);
    let gap_severities = scheme
        .iter()
        .filter_map(|(concept, gap)| Some((concept.clone(), gap.severity.clone()?)))
        .collect();
    let vocabulary = Vocabulary::read(&adapter.vocabulary_files, stamps(adapter)?, resolver)?;
    let accounting = adapter
        .source_accounting
        .as_deref()
        .map(|iri| Accounting::read(resolver, iri, &scheme))
        .transpose()?;
    Ok(Prepared {
        syntax,
        unit,
        mappings,
        findings_queries,
        detect,
        tables,
        prefixes,
        findings_prefixes,
        envelopes,
        source_schema,
        vocabulary,
        accounting,
        gap_severities,
    })
}

fn tables(adapter: &Adapter, resolver: &dyn Resolver) -> Result<Vec<Quad>> {
    let mut tables = Vec::new();
    for iri in &adapter.tables {
        let format = value(&adapter.graph, &subject(iri)?, SCHEMA_ENCODING_FORMAT)?;
        if format.as_deref() != Some("text/turtle") {
            return Err(Error::msg(format!(
                "table {iri} is {}; this Bridge loads text/turtle tables",
                format.as_deref().unwrap_or("undeclared")
            )));
        }
        let bytes = resolver.read(iri)?;
        for quad in RdfParser::from_format(RdfFormat::Turtle)
            .with_base_iri(iri)?
            .rename_blank_nodes()
            .for_slice(&bytes)
        {
            tables.push(quad.map_err(|e| Error::msg(format!("{iri}: {e}")))?);
        }
    }
    Ok(tables)
}

fn queries(resolver: &dyn Resolver, iris: &[String], what: &str) -> Result<Vec<Query>> {
    iris.iter()
        .map(|iri| Query::read(resolver, iri, Form::Construct, what))
        .collect()
}

fn envelopes(adapter: &Adapter, syntax: Syntax, resolver: &dyn Resolver) -> Result<Vec<Envelope>> {
    let mut envelopes = Vec::new();
    for envelope in &adapter.envelopes {
        envelopes.push(Envelope {
            iri: envelope.iri.clone(),
            document_root: envelope.doc_root_element_name.clone(),
            document_schema: envelope
                .document_schema
                .as_deref()
                .map(|iri| syntax.schema(iri, resolver))
                .transpose()?,
        });
    }
    Ok(envelopes)
}

fn prefixes(mappings: &[Query]) -> Vec<(String, String)> {
    let mut prefixes: Vec<(String, String)> = Vec::new();
    for (name, namespace) in mappings.iter().flat_map(|m| m.prefixes.iter()) {
        if !prefixes.iter().any(|(taken, _)| taken == name) {
            prefixes.push((name.clone(), namespace.clone()));
        }
    }
    prefixes
}

fn findings_prefixes(gap_prefixes: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut findings_prefixes: Vec<(String, String)> = FINDINGS_PREFIXES
        .iter()
        .map(|(name, namespace)| ((*name).to_owned(), (*namespace).to_owned()))
        .collect();
    for (name, namespace) in gap_prefixes {
        if !findings_prefixes
            .iter()
            .any(|(taken, bound)| *taken == name || *bound == namespace)
        {
            findings_prefixes.push((name, namespace));
        }
    }
    findings_prefixes
}

pub(crate) fn convert(prepared: &Prepared, source: Source<'_>) -> Result<Conversion> {
    let named = source
        .envelope
        .map(|iri| prepared.named_envelope(iri))
        .transpose()?;
    let syntax = prepared.syntax;
    let text = syntax.decode(source.bytes)?;
    let paths = prepared
        .accounting
        .as_ref()
        .map_or(Paths::Dropped, Accounting::paths);
    let mut lift = syntax.lift(&text, &prepared.unit, paths)?;
    let followed = syntax.addresses();
    let mut conversion = Conversion {
        quads: Vec::new(),
        findings: Vec::new(),
        units: 0,
        detected: None,
    };
    while let Some(unit) = lift.next_unit()? {
        conversion.units += 1;
        // A record was read, so the document root was: no envelope is needed yet.
        let document = lift.document_selector(None);
        let (produced, findings) =
            unit_converted(prepared, source.iri, &document, &unit, followed.as_ref())?;
        conversion.quads.extend(produced);
        conversion.findings.extend(findings);
    }
    let envelope = named.or_else(|| prepared.envelope_of(lift.document_root()));
    conversion.findings.extend(document_findings(
        prepared,
        envelope,
        source.iri,
        lift.as_ref(),
        &text,
    )?);
    conversion.detected = prepared
        .detect
        .as_ref()
        .map(|detect| detected(detect, lift))
        .transpose()?;
    Ok(conversion)
}

fn unit_converted(
    prepared: &Prepared,
    iri: &str,
    document: &str,
    unit: &Unit,
    followed: &dyn Addresses,
) -> Result<(Vec<Quad>, Vec<Quad>)> {
    let selector = unit.selector();
    let selector_type = prepared.syntax.selector_type();
    let record = Record {
        source: iri,
        selector: &selector,
        selector_type,
    };
    let mut findings = validated(prepared.source_schema.as_deref(), &record, &unit.text)?;
    if let Some(accounting) = &prepared.accounting {
        findings.extend(accounting.findings(&record, unit)?);
    }
    let produced = mapped(prepared, unit)?;
    if let Some(vocabulary) = &prepared.vocabulary {
        findings.extend(vocabulary.findings(&record, &produced)?);
    }
    findings.extend(queried(prepared, &record, unit)?);
    let document = Record {
        source: iri,
        selector: document,
        selector_type,
    };
    let unresolved = followed.unresolved(&document, &unit.text, &findings)?;
    findings.extend(unresolved);
    Ok((produced, findings))
}

fn validated(schema: Option<&dyn Schema>, record: &Record<'_>, text: &str) -> Result<Vec<Quad>> {
    let mut findings = Vec::new();
    let Some(schema) = schema else {
        return Ok(findings);
    };
    for broken in schema.errors(text)? {
        findings.extend(annotation::violation(
            record,
            broken.body(),
            broken.within(),
        )?);
    }
    Ok(findings)
}

fn mapped(prepared: &Prepared, unit: &Unit) -> Result<Vec<Quad>> {
    unit.store.extend(prepared.tables.iter().cloned())?;
    let mut produced = Vec::new();
    for mapping in &prepared.mappings {
        produced.extend(mapping.graph(&unit.store)?);
    }
    Ok(produced)
}

fn queried(prepared: &Prepared, record: &Record<'_>, unit: &Unit) -> Result<Vec<Quad>> {
    let mut findings = Vec::new();
    for findings_query in &prepared.findings_queries {
        let constructed = findings_query.graph(&unit.store)?;
        findings.extend(annotation::about(
            record,
            &findings_query.iri,
            constructed,
            &prepared.gap_severities,
        )?);
    }
    Ok(findings)
}

/// This Bridge wrote these addresses, so they are not followed as an adapter's are.
fn document_findings(
    prepared: &Prepared,
    envelope: Option<&Envelope>,
    iri: &str,
    lift: &dyn Lift,
    text: &str,
) -> Result<Vec<Quad>> {
    let Some((envelope, schema)) =
        envelope.and_then(|envelope| Some((envelope, envelope.document_schema.as_deref()?)))
    else {
        return Ok(Vec::new());
    };
    let selector = lift.document_selector(envelope.document_root.as_deref());
    let record = Record {
        source: iri,
        selector: &selector,
        selector_type: prepared.syntax.selector_type(),
    };
    validated(Some(schema), &record, text)
}

fn detected(detect: &Query, lift: Box<dyn Lift + '_>) -> Result<bool> {
    let skeleton = lift.into_skeleton()?;
    let QueryResults::Boolean(answer) = detect.on(&skeleton)? else {
        return Err(Error::msg(format!(
            "detect query {} is not an ASK",
            detect.iri
        )));
    };
    Ok(answer)
}

#[cfg(test)]
mod adapter_files;
#[cfg(test)]
mod address;
#[cfg(test)]
mod entry_gaps;
#[cfg(test)]
mod fetching;
#[cfg(test)]
mod finding_bodies;
#[cfg(test)]
mod findings;
#[cfg(test)]
mod lookup_misses;
#[cfg(test)]
mod output_validation;
#[cfg(test)]
mod selector;
#[cfg(test)]
mod source_accounting;
#[cfg(test)]
mod validation;
#[cfg(test)]
mod tests {
    use super::prepare;
    use crate::fixtures::tiny;
    use crate::load::load_adapter;
    use crate::{DirectoryResolver, Resolver, Result};
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    struct Counting {
        directory: DirectoryResolver,
        reads: RefCell<BTreeMap<String, usize>>,
    }

    impl Counting {
        fn count(&self, iri: &str) {
            *self.reads.borrow_mut().entry(iri.to_owned()).or_default() += 1;
        }
    }

    impl Resolver for Counting {
        fn root(&self) -> &str {
            self.directory.root()
        }

        fn vocabularies(&self) -> Option<&str> {
            self.directory.vocabularies()
        }

        fn read(&self, iri: &str) -> Result<Vec<u8>> {
            self.count(iri);
            self.directory.read(iri)
        }

        fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
            self.count(iri);
            self.directory.read_vocabulary(iri)
        }
    }

    #[test]
    fn reads_each_query_of_the_adapter_once_per_prepare() {
        let adapter = load_adapter(&tiny()).expect("adapter");
        let counting = Counting {
            directory: tiny(),
            reads: RefCell::default(),
        };
        prepare(&adapter, &counting).expect("prepared");
        let reads = counting.reads.into_inner();
        let queries: Vec<&String> = adapter
            .mappings
            .iter()
            .chain(&adapter.findings_queries)
            .chain(&adapter.detect_query)
            .collect();
        assert!(!adapter.findings_queries.is_empty() && adapter.detect_query.is_some());
        let misread: Vec<(&String, usize)> = queries
            .into_iter()
            .map(|query| (query, reads.get(query).copied().unwrap_or_default()))
            .filter(|(_, times)| *times != 1)
            .collect();
        assert_eq!(misread, Vec::<(&String, usize)>::new());
    }
}
