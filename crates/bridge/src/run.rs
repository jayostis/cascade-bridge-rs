use crate::accounting::{gap_scheme, Accounting};
use crate::annotation::{self, Record};
use crate::error::{Error, Result};
use crate::lift::{admitting, Admission, Lift, Paths, Reading, Unit};
use crate::load::{subject, value, Adapter};
use crate::query::{Form, Query};
use crate::rdf::FINDINGS_PREFIXES;
use crate::records::{self, Document, Release, Supplied};
use crate::resolver::Resolver;
use crate::syntax::{Addresses, Syntax};
use crate::terms::SCHEMA_ENCODING_FORMAT;
use crate::validate::Schema;
use crate::vocabulary::Vocabulary;
use oxigraph::model::Quad;
use oxigraph::sparql::QueryResults;
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::{HashMap, HashSet};

struct Envelope {
    iri: String,
    admission: Admission,
    document_schema: Option<Box<dyn Schema>>,
}

pub(crate) struct Prepared {
    pub(crate) syntax: Syntax,
    unit: Option<String>,
    mappings: Vec<Query>,
    findings_queries: Vec<Query>,
    document_table_queries: Vec<Query>,
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
    identifier: Option<String>,
    version: Option<String>,
}

impl Prepared {
    fn named_envelope(&self, iri: &str) -> Result<usize> {
        self.envelopes
            .iter()
            .position(|envelope| envelope.iri == iri)
            .ok_or_else(|| Error::msg(format!("the adapter declares no envelope {iri}")))
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
    pub(crate) facts: Option<Supplied<'a>>,
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
    let document_table_queries = queries(
        resolver,
        &adapter.document_table_queries,
        "document table query",
    )?;
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
    let vocabulary = Vocabulary::read(&adapter.vocabulary_files, resolver)?;
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
        document_table_queries,
        detect,
        tables,
        prefixes,
        findings_prefixes,
        envelopes,
        source_schema,
        vocabulary,
        accounting,
        gap_severities,
        identifier: adapter.identifier.clone(),
        version: adapter.version.clone(),
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
            admission: syntax.admission(envelope)?,
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
    let reading = Reading {
        element: prepared.unit.as_deref(),
        envelopes: prepared
            .envelopes
            .iter()
            .map(|envelope| &envelope.admission)
            .collect(),
        named,
    };
    let arrived_in = Document::new(source.bytes, source.facts)?;
    let table = document_table(prepared, &text, &reading, &arrived_in)?;
    let arrived_in = arrived_in.with_table(table);
    let mut lift = syntax.lift(&text, &reading, paths)?;
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
        let (produced, findings) = unit_converted(
            prepared,
            source.iri,
            &document,
            &unit,
            followed.as_ref(),
            &arrived_in,
        )?;
        conversion.quads.extend(produced);
        conversion.findings.extend(findings);
    }
    conversion.quads.extend(arrived_in.described(
        &Release {
            label: prepared.identifier.as_deref(),
            version: prepared.version.as_deref(),
        },
        &Release {
            label: Some(crate::command::NAME),
            version: Some(env!("CARGO_PKG_VERSION")),
        },
    ));
    let envelope = admitting(lift.as_ref(), &reading).map(|chosen| &prepared.envelopes[chosen]);
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
    arrived_in: &Document,
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
    unit.store.extend(arrived_in.dataset(&selector))?;
    let produced = arrived_in.arrived(records::versioned(mapped(prepared, unit)?)?, &selector);
    if let Some(vocabulary) = &prepared.vocabulary {
        findings.extend(vocabulary.findings(&record, &produced)?);
    }
    let queried = queried(prepared, &record, unit)?;
    let document = Record {
        source: iri,
        selector: document,
        selector_type,
    };
    let unresolved = followed.unresolved(&document, &unit.text, &queried)?;
    findings.extend(queried);
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

fn document_table(
    prepared: &Prepared,
    text: &str,
    reading: &Reading<'_>,
    arrived_in: &Document,
) -> Result<Vec<Quad>> {
    let mut table = Vec::new();
    if prepared.document_table_queries.is_empty() {
        return Ok(table);
    }
    let mut lift = prepared.syntax.lift(text, reading, Paths::Dropped)?;
    while let Some(unit) = lift.next_unit()? {
        unit.store.extend(prepared.tables.iter().cloned())?;
        unit.store.extend(arrived_in.dataset(&unit.selector()))?;
        for query in &prepared.document_table_queries {
            table.extend(query.graph(&unit.store)?);
        }
    }
    Ok(table)
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
    let selector = lift.document_selector(envelope.admission.root.as_deref());
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
mod json_source;
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
            .chain(&adapter.document_table_queries)
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
