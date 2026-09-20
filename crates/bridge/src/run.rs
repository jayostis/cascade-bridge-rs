// Running an adapter on one document, per unit: lift the unit, validate it,
// load the tables beside it, run every mapping and union the graphs, then run
// every findings query and make its annotations about that unit. Each unit
// gets a store of its own, so no query can see another unit, and each query
// execution's blank nodes are kept apart from every other's, so two findings
// never fuse into one.
//
// Everything an adapter runs is read and parsed once, here, before any
// document: a query's text is never handed to the engine twice, and a schema
// is compiled once however many records it validates.
use crate::annotation::{self, Minted, Record};
use crate::decode::decode;
use crate::error::{Error, Result};
use crate::lift::lift_text;
use crate::load::{subject, value, Adapter};
use crate::rdf::SCHEMA_ENCODING_FORMAT;
use crate::resolver::Resolver;
use crate::validate::{self, Schema};
use oxigraph::model::{GraphName, Quad};
use oxigraph::sparql::{PreparedSparqlQuery, QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use oxrdfio::{RdfFormat, RdfParser};
use std::borrow::Cow;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// The form a query's own text declares, decided when it is parsed. Deciding
/// it from a result instead accepts a SELECT that matched nothing as a
/// CONSTRUCT that produced nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Select,
    Construct,
    Ask,
    Describe,
}

impl Form {
    fn keyword(self) -> &'static str {
        match self {
            Self::Select => "SELECT",
            Self::Construct => "CONSTRUCT",
            Self::Ask => "ASK",
            Self::Describe => "DESCRIBE",
        }
    }

    fn of(query: &spargebra::Query) -> Self {
        match query {
            spargebra::Query::Select { .. } => Self::Select,
            spargebra::Query::Construct { .. } => Self::Construct,
            spargebra::Query::Ask { .. } => Self::Ask,
            spargebra::Query::Describe { .. } => Self::Describe,
        }
    }
}

pub struct Query {
    pub iri: String,
    pub form: Form,
    /// The names the query's prologue gives namespaces, so a graph a mapping
    /// built can be written back in the mapping's own spelling.
    pub prefixes: Vec<(String, String)>,
    prepared: PreparedSparqlQuery,
}

impl Query {
    fn on(&self, store: &Store) -> Result<QueryResults<'static>> {
        // Cloning the prepared query copies the algebra the parser already
        // built; the text is not seen again.
        Ok(self.prepared.clone().on_store(store).execute()?)
    }

    /// The graph the CONSTRUCT built, with this execution's blank nodes kept
    /// apart from every other execution's.
    fn graph(&self, store: &Store) -> Result<Vec<Quad>> {
        let QueryResults::Graph(triples) = self.on(store)? else {
            return Err(Error::msg(format!(
                "{} is not a {}",
                self.iri,
                Form::Construct.keyword()
            )));
        };
        let mut quads = Vec::new();
        for triple in triples {
            let triple = triple?;
            quads.push(Quad::new(
                triple.subject,
                triple.predicate,
                triple.object,
                GraphName::DefaultGraph,
            ));
        }
        Ok(Minted::default().apart(quads))
    }
}

struct Envelope {
    iri: String,
    doc_root_element_name: Option<String>,
    document_schema: Option<Schema>,
}

pub struct Prepared {
    pub unit: String,
    pub mappings: Vec<Query>,
    pub findings_queries: Vec<Query>,
    pub detect: Option<Query>,
    /// The tables, parsed once rather than once per unit.
    pub tables: Vec<Quad>,
    /// Every name the mappings give a namespace, the first binding of a name
    /// winning, as a query's own prologue binds it.
    pub prefixes: Vec<(String, String)>,
    envelopes: Vec<Envelope>,
    source_schema: Option<Schema>,
}

impl Prepared {
    fn named_envelope(&self, iri: &str) -> Result<&Envelope> {
        self.envelopes
            .iter()
            .find(|envelope| envelope.iri == iri)
            .ok_or_else(|| Error::msg(format!("the adapter declares no envelope {iri}")))
    }

    /// The envelope a document arrived in, when the caller names none: the one
    /// whose document root element the document's own is.
    fn envelope_of(&self, document_element: Option<&str>) -> Option<&Envelope> {
        let element = document_element?;
        self.envelopes
            .iter()
            .find(|envelope| envelope.doc_root_element_name.as_deref() == Some(element))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Ms {
    pub lift: Duration,
    pub load: Duration,
    pub detect: Duration,
    pub mappings: Duration,
    pub findings: Duration,
    pub validation: Duration,
}

pub struct Conversion {
    pub quads: Vec<Quad>,
    /// Every finding about the source document, as Web Annotations.
    pub findings: Vec<Quad>,
    pub units: usize,
    /// The detect query's answer over the skeleton, when the adapter has one.
    pub detected: Option<bool>,
    pub ms: Ms,
}

impl Conversion {
    pub fn annotations(&self) -> usize {
        annotation::annotations(&self.findings)
    }

    /// How many triples the graph holds. `quads` is the records' raw union, and
    /// a triple constructed for every record stands in it once per record and
    /// in the graph once.
    pub fn triples(&self) -> usize {
        self.quads.iter().collect::<HashSet<&Quad>>().len()
    }
}

/// A document to convert: its bytes, the IRI a finding about it names, and the
/// envelope it arrived in where the caller knows it.
pub struct Source<'a> {
    pub iri: &'a str,
    pub envelope: Option<&'a str>,
    pub xml: &'a [u8],
}

/// Whitespace and comments, which stand between any two tokens of a prologue.
fn between(text: &str) -> &str {
    let mut rest = text.trim_start();
    while let Some(comment) = rest.strip_prefix('#') {
        rest = comment
            .find('\n')
            .map_or("", |end| &comment[end..])
            .trim_start();
    }
    rest
}

/// What follows the keyword, where the text begins with it.
fn keyword<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let rest = text.get(word.len()..)?;
    (text[..word.len()].eq_ignore_ascii_case(word)
        && rest.starts_with(|c: char| c.is_whitespace() || c == '#' || c == '<'))
    .then_some(rest)
}

/// A declaration's name, up to the colon that ends it, and what follows.
fn declared_name(text: &str) -> Option<(&str, &str)> {
    let (name, rest) = text.split_once(':')?;
    (!name.contains(char::is_whitespace)).then_some((name, rest))
}

/// The IRI the angle brackets hold, and what follows.
fn iri_ref(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix('<')?;
    let end = rest.find('>')?;
    Some((&rest[..end], &rest[end + 1..]))
}

/// The prologue's PREFIX declarations. SPARQL keeps them out of the algebra a
/// parser returns, and they are the only names for these namespaces anyone has
/// written down. It is read as SPARQL writes it — a run of BASE and PREFIX
/// declarations, laid out however the author laid them out, ending where the
/// query form begins.
fn prologue_prefixes(text: &str) -> Vec<(String, String)> {
    let mut prefixes = Vec::new();
    let mut rest = between(text);
    loop {
        if let Some(after) = keyword(rest, "BASE") {
            let Some((_, after)) = iri_ref(between(after)) else {
                break;
            };
            rest = between(after);
        } else if let Some(after) = keyword(rest, "PREFIX") {
            let Some((name, after)) = declared_name(between(after)) else {
                break;
            };
            let Some((namespace, after)) = iri_ref(between(after)) else {
                break;
            };
            prefixes.push((name.to_owned(), namespace.to_owned()));
            rest = between(after);
        } else {
            break;
        }
    }
    prefixes
}

/// What a finding about the document selects: the document's own element, or,
/// where the document has none, the element its envelope describes. Validation
/// reports and never refuses, so a document with no element at all is still
/// addressed, by whatever name there is for the element it was to have.
fn document_selector(document: Option<String>, described: Option<&str>) -> String {
    document
        .or_else(|| described.map(|element| format!("/{element}")))
        .unwrap_or_else(|| "/*".to_owned())
}

fn query(resolver: &dyn Resolver, iri: &str, expected: Form, what: &str) -> Result<Query> {
    let text = String::from_utf8(resolver.read(iri)?)?;
    let parsed = spargebra::SparqlParser::new()
        .with_base_iri(iri)
        .map_err(|e| Error::msg(format!("{iri}: {e}")))?
        .parse_query(&text)
        .map_err(|e| Error::msg(format!("{iri}: {e}")))?;
    let form = Form::of(&parsed);
    if form != expected {
        return Err(Error::msg(format!(
            "{what} {iri} is not a {}",
            expected.keyword()
        )));
    }
    Ok(Query {
        iri: iri.to_owned(),
        form,
        prefixes: prologue_prefixes(&text),
        prepared: SparqlEvaluator::new().for_query(parsed),
    })
}

/// Read, parse and check everything an adapter runs, once, before any
/// document.
pub fn prepare(adapter: &Adapter, resolver: &dyn Resolver) -> Result<Prepared> {
    if adapter.mappings.is_empty() {
        return Err(Error::msg("the adapter names no bridge:mapping"));
    }
    // Without a unit nothing is split off, so no mapping would run and an
    // empty result would pass for a conversion.
    let unit = adapter
        .element_name_of_each_record
        .clone()
        .ok_or_else(|| Error::msg("the adapter names no bridge:elementNameOfEachRecord"))?;
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
            tables.push(quad?);
        }
    }

    let mut mappings = Vec::new();
    for iri in &adapter.mappings {
        mappings.push(query(resolver, iri, Form::Construct, "mapping")?);
    }
    let mut findings_queries = Vec::new();
    for iri in &adapter.findings_queries {
        findings_queries.push(query(resolver, iri, Form::Construct, "findings query")?);
    }
    let detect = adapter
        .detect_query
        .as_deref()
        .map(|iri| query(resolver, iri, Form::Ask, "detect query"))
        .transpose()?;

    let source_schema = adapter
        .source_schema
        .as_deref()
        .map(|iri| validate::compile(iri, resolver))
        .transpose()?;
    let mut envelopes = Vec::new();
    for envelope in &adapter.envelopes {
        envelopes.push(Envelope {
            iri: envelope.iri.clone(),
            doc_root_element_name: envelope.doc_root_element_name.clone(),
            document_schema: envelope
                .document_schema
                .as_deref()
                .map(|iri| validate::compile(iri, resolver))
                .transpose()?,
        });
    }

    let mut prefixes: Vec<(String, String)> = Vec::new();
    for (name, namespace) in mappings.iter().flat_map(|m| m.prefixes.iter()) {
        if !prefixes.iter().any(|(taken, _)| taken == name) {
            prefixes.push((name.clone(), namespace.clone()));
        }
    }

    Ok(Prepared {
        unit,
        mappings,
        findings_queries,
        detect,
        tables,
        prefixes,
        envelopes,
        source_schema,
    })
}

pub fn convert(prepared: &Prepared, source: Source<'_>) -> Result<Conversion> {
    let mut ms = Ms::default();
    let mut quads = Vec::new();
    let mut findings = Vec::new();
    let mut units = 0;

    let named = source
        .envelope
        .map(|iri| prepared.named_envelope(iri))
        .transpose()?;
    // Decoding is the one stage that holds the whole document at once, so the
    // document schema below reads these characters rather than its own copy.
    let text = decode(source.xml)?;
    let mut lift = lift_text(Cow::Borrowed(&text), Some(&prepared.unit))?;
    loop {
        let at = Instant::now();
        let Some(unit) = lift.next_unit()? else {
            break;
        };
        ms.lift += at.elapsed();
        units += 1;
        let selector = unit.selector();
        let record = Record {
            source: source.iri,
            selector: &selector,
        };

        let at = Instant::now();
        if let Some(schema) = &prepared.source_schema {
            for broken in schema.errors(&unit.xml)? {
                findings.extend(annotation::violation(
                    &record,
                    &broken.body(),
                    broken.within(),
                )?);
            }
        }
        ms.validation += at.elapsed();

        let at = Instant::now();
        unit.store.extend(prepared.tables.iter().cloned())?;
        ms.load += at.elapsed();

        let at = Instant::now();
        for mapping in &prepared.mappings {
            quads.extend(mapping.graph(&unit.store)?);
        }
        ms.mappings += at.elapsed();

        let at = Instant::now();
        for findings_query in &prepared.findings_queries {
            let constructed = findings_query.graph(&unit.store)?;
            findings.extend(annotation::about(
                &record,
                &findings_query.iri,
                constructed,
            )?);
        }
        ms.findings += at.elapsed();
    }

    let envelope = named.or_else(|| prepared.envelope_of(lift.document_element()));
    let at = Instant::now();
    if let Some((envelope, schema)) =
        envelope.and_then(|envelope| Some((envelope, envelope.document_schema.as_ref()?)))
    {
        let selector = document_selector(
            lift.document_selector(),
            envelope.doc_root_element_name.as_deref(),
        );
        let record = Record {
            source: source.iri,
            selector: &selector,
        };
        for broken in schema.errors(&text)? {
            findings.extend(annotation::violation(
                &record,
                &broken.body(),
                broken.within(),
            )?);
        }
    }
    ms.validation += at.elapsed();

    let mut detected = None;
    if let Some(detect) = &prepared.detect {
        let at = Instant::now();
        let skeleton = lift.into_skeleton()?;
        let QueryResults::Boolean(answer) = detect.on(&skeleton)? else {
            return Err(Error::msg(format!(
                "detect query {} is not an ASK",
                detect.iri
            )));
        };
        detected = Some(answer);
        ms.detect = at.elapsed();
    }

    Ok(Conversion {
        quads,
        findings,
        units,
        detected,
        ms,
    })
}

#[cfg(test)]
mod tests {
    use super::{document_selector, prologue_prefixes};

    #[test]
    fn selects_the_element_the_envelope_describes_where_the_document_has_none() {
        assert_eq!(
            document_selector(None, Some("catalog")),
            "/catalog",
            "a finding about the document selects the envelope's document root element"
        );
        assert_eq!(
            document_selector(Some("/other".to_owned()), Some("catalog")),
            "/other"
        );
        assert_eq!(document_selector(None, None), "/*");
    }

    #[test]
    fn reads_a_prologue_however_its_keyword_and_spacing_are_written() {
        assert_eq!(
            prologue_prefixes(
                "prefix ex: <urn:example:catalog#>\n  PREFIX  g:<https://ns.example.org/g/v1#>\nCONSTRUCT { }"
            ),
            [
                ("ex".to_owned(), "urn:example:catalog#".to_owned()),
                ("g".to_owned(), "https://ns.example.org/g/v1#".to_owned()),
            ]
        );
    }

    #[test]
    fn reads_both_declarations_a_prologue_writes_on_one_line() {
        assert_eq!(
            prologue_prefixes(
                "PREFIX ex: <urn:example:catalog#> PREFIX v1: <https://ns.example.org/v1#>\nCONSTRUCT { }"
            ),
            [
                ("ex".to_owned(), "urn:example:catalog#".to_owned()),
                ("v1".to_owned(), "https://ns.example.org/v1#".to_owned()),
            ]
        );
    }

    #[test]
    fn reads_a_prefix_a_base_and_a_comment_stand_before() {
        assert_eq!(
            prologue_prefixes(
                "BASE <urn:example:> # where the names begin\nPREFIX ex: <urn:example:catalog#>\nCONSTRUCT { }"
            ),
            [("ex".to_owned(), "urn:example:catalog#".to_owned())]
        );
    }

    #[test]
    fn reads_no_prefix_out_of_a_comment_or_a_word_that_merely_starts_with_one() {
        assert_eq!(
            prologue_prefixes("# PREFIX ex: <urn:example:catalog#>\nPREFIXES ex: <urn:x#>"),
            Vec::new()
        );
    }
}
