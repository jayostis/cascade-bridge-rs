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
use crate::decode::{decode, XML_SPACE};
use crate::error::{Error, Result};
use crate::lift::{lift_text, Paths, Valued};
use crate::load::{subject, value, Adapter};
use crate::rdf::{
    BRIDGE_CARRIED_IN_PART, BRIDGE_LOOKUP_IN, BRIDGE_LOOKUP_NAMES_GAP, BRIDGE_NAMES_GAP,
    BRIDGE_NO_HOME, BRIDGE_NO_PREDICATE, BRIDGE_PATH_ENTRY, BRIDGE_SOURCE_LACKS_REQUIRED,
    BRIDGE_SOURCE_PATH, BRIDGE_VALUE_NOT_MAPPED, BRIDGE_VERDICT, RDF_TYPE, SCHEMA_ENCODING_FORMAT,
    SH_INFO, SH_RESULT_SEVERITY, SH_VIOLATION, SH_WARNING, SKOS_BROADER, SKOS_CONCEPT_SCHEME,
    SKOS_NOTATION,
};
use crate::resolver::Resolver;
use crate::validate::{self, Schema};
use crate::xpath;
use oxigraph::model::{GraphName, NamedOrBlankNode, Quad, Term};
use oxigraph::sparql::{PreparedSparqlQuery, QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use oxrdfio::{RdfFormat, RdfParser};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
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
    accounting: Option<Accounting>,
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

/// One path of the source, what the adapter does with it, and the gap it opens
/// where it opens one.
struct Entry {
    path: String,
    verdict: Option<String>,
    gap: Option<String>,
    /// The concept map the values at this path are looked up in, and the gap a
    /// value it holds no notation for bodies.
    map: Option<String>,
    miss: Option<String>,
}

/// What a concept of an adapter's gap scheme declares about itself.
#[derive(Default)]
struct Gap {
    kind: Option<String>,
    severity: Option<String>,
}

/// The verdicts that may name a gap at all.
const NAMES_A_GAP: [&str; 2] = [BRIDGE_NO_HOME, BRIDGE_CARRIED_IN_PART];

/// The kinds of gap true of the path rather than of what a record happens to
/// hold at it. Only some of a path's occurrences are carried with loss or left
/// unmapped, and an entry cannot say which, so a kind true of what a record
/// holds is left to a findings query, which can count and compare.
const REPORTS: [&str; 2] = [BRIDGE_NO_PREDICATE, BRIDGE_SOURCE_LACKS_REQUIRED];

/// The severities a gap concept may declare, which are the severities the
/// adapter profile's shape for a source finding accepts.
const SEVERITIES: [&str; 3] = [SH_INFO, SH_WARNING, SH_VIOLATION];

/// A gap a path's entry reports, and the severity its concept gives it.
struct Reported {
    gap: String,
    severity: String,
}

/// A concept map an entry looks a path's values up in: the notations the one
/// scheme the file holds carries, the gap a value outside them bodies, and the
/// severity that gap's concept gives it. Nothing else about a concept is a
/// Bridge's business; the term each one matches is the mapping query's.
struct Lookup {
    gap: String,
    severity: String,
    notations: Arc<HashSet<String>>,
}

/// A value's key: the value lowercased and trimmed of XML's S production,
/// which is the form a `skos:notation` is written in.
fn key(value: &str) -> String {
    value.trim_matches(XML_SPACE).to_lowercase()
}

/// What an adapter's accounting says about the paths of a record: which of
/// them it carries at all, which of them an entry reports a gap at, and which
/// of them an entry looks the values at up.
struct Accounting {
    paths: HashSet<String>,
    reported: HashMap<String, Vec<Reported>>,
    lookups: HashMap<String, Vec<Lookup>>,
}

impl Accounting {
    /// A gap an entry names and the scheme cannot say the kind of is refused
    /// here rather than passed over. Whether it reports is not a question this
    /// Bridge can answer about such a gap, and answering "it does not" drops a
    /// finding for a reason no reader of the output can see.
    fn of(
        entries: Vec<Entry>,
        scheme: &HashMap<String, Gap>,
        iri: &str,
        resolver: &dyn Resolver,
    ) -> Result<Self> {
        let mut paths = HashSet::new();
        let mut reported: HashMap<String, Vec<Reported>> = HashMap::new();
        let mut lookups: HashMap<String, Vec<Lookup>> = HashMap::new();
        let mut maps: HashMap<String, Arc<HashSet<String>>> = HashMap::new();
        for entry in entries {
            if let (Some(verdict), Some(gap)) = (&entry.verdict, &entry.gap) {
                if NAMES_A_GAP.contains(&verdict.as_str()) {
                    let declared = scheme.get(gap.as_str()).ok_or_else(|| {
                        Error::msg(format!(
                            "{iri}: the entry for {} names {gap}, which the adapter's \
                             bridge:gapScheme does not declare",
                            entry.path
                        ))
                    })?;
                    let kind = declared.kind.as_deref().ok_or_else(|| {
                        Error::msg(format!(
                            "{iri}: the entry for {} names {gap}, which declares no skos:broader, \
                             so what kind of gap it is cannot be read",
                            entry.path
                        ))
                    })?;
                    if REPORTS.contains(&kind) {
                        reported
                            .entry(entry.path.clone())
                            .or_default()
                            .push(Reported {
                                gap: gap.clone(),
                                severity: declared
                                    .severity
                                    .clone()
                                    .unwrap_or_else(|| SH_INFO.to_owned()),
                            });
                    }
                }
            }
            match (&entry.map, &entry.miss) {
                (None, None) => {}
                (Some(_), None) | (None, Some(_)) => {
                    return Err(Error::msg(format!(
                        "{iri}: the entry for {} declares one half of a lookup; bridge:lookupIn \
                         and bridge:lookupNamesGap are declared together or not at all",
                        entry.path
                    )))
                }
                (Some(map), Some(gap)) => {
                    let declared = scheme.get(gap.as_str()).ok_or_else(|| {
                        Error::msg(format!(
                            "{iri}: the lookup of the entry for {} names {gap}, which the \
                             adapter's bridge:gapScheme does not declare",
                            entry.path
                        ))
                    })?;
                    let kind = declared.kind.as_deref().ok_or_else(|| {
                        Error::msg(format!(
                            "{iri}: the lookup of the entry for {} names {gap}, which declares no \
                             skos:broader, so what kind of gap it is cannot be read",
                            entry.path
                        ))
                    })?;
                    if kind != BRIDGE_VALUE_NOT_MAPPED {
                        return Err(Error::msg(format!(
                            "{iri}: the lookup of the entry for {} names {gap}, a gap of kind \
                             {kind}; a value a concept map holds no notation for is a gap of kind \
                             {BRIDGE_VALUE_NOT_MAPPED}",
                            entry.path
                        )));
                    }
                    if !maps.contains_key(map) {
                        let notations = concept_map(resolver, map).map_err(|e| {
                            Error::msg(format!(
                                "{iri}: the entry for {} looks its values up in {e}",
                                entry.path
                            ))
                        })?;
                        maps.insert(map.clone(), Arc::new(notations));
                    }
                    lookups.entry(entry.path.clone()).or_default().push(Lookup {
                        gap: gap.clone(),
                        severity: declared
                            .severity
                            .clone()
                            .unwrap_or_else(|| SH_INFO.to_owned()),
                        notations: maps[map].clone(),
                    });
                }
            }
            paths.insert(entry.path);
        }
        // A graph has no order of its own, so a run's output does not depend on
        // which way a hash happened to fall.
        for gaps in reported.values_mut() {
            gaps.sort_by(|one, two| one.gap.cmp(&two.gap));
        }
        for found in lookups.values_mut() {
            found.sort_by(|one, two| one.gap.cmp(&two.gap));
        }
        Ok(Self {
            paths,
            reported,
            lookups,
        })
    }
}

/// The one IRI an entry declares for a predicate, where it declares any. Two
/// leave the parse order deciding what the entry says, and a literal says
/// nothing an entry can be read by.
fn declared(objects: &[Term], iri: &str, path: &str, predicate: &str) -> Result<Option<String>> {
    if let [first, second, ..] = objects {
        return Err(Error::msg(format!(
            "{iri}: the entry for {path} declares the {predicate} {first} and {second}; an entry \
             declares at most one"
        )));
    }
    let Some(object) = objects.first() else {
        return Ok(None);
    };
    let Term::NamedNode(named) = object else {
        return Err(Error::msg(format!("{iri}: {object} is no {predicate}")));
    };
    Ok(Some(named.as_str().to_owned()))
}

/// The entries an accounting carries, each a `bridge:PathEntry` with a literal
/// `bridge:sourcePath`. A crate that names an accounting and cannot show it is
/// refused here, where a crate that names none is never asked.
///
/// A verdict and a gap are read from an entry and from nothing else, so what
/// neither is read from is refused for neither. A `bridge:sourcePath` is the
/// standing exception: dropping one that is no literal would leave the census
/// reporting the path as unaccounted, which is a wrong finding and not an
/// absent one.
fn entries(resolver: &dyn Resolver, iri: &str) -> Result<Vec<Entry>> {
    let bytes = resolver
        .read(iri)
        .map_err(|e| Error::msg(format!("{iri}: {e}")))?;
    let mut typed = HashSet::new();
    let mut named = Vec::new();
    let mut verdicts: HashMap<String, Vec<Term>> = HashMap::new();
    let mut gaps: HashMap<String, Vec<Term>> = HashMap::new();
    let mut maps: HashMap<String, Vec<Term>> = HashMap::new();
    let mut misses: HashMap<String, Vec<Term>> = HashMap::new();
    for quad in RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(iri)?
        .for_slice(&bytes)
    {
        let quad = quad.map_err(|e| Error::msg(format!("{iri}: {e}")))?;
        let subject = quad.subject.to_string();
        match quad.predicate.as_str() {
            RDF_TYPE if matches!(&quad.object, Term::NamedNode(entry) if entry.as_str() == BRIDGE_PATH_ENTRY) =>
            {
                typed.insert(subject);
            }
            BRIDGE_SOURCE_PATH => {
                let Term::Literal(path) = &quad.object else {
                    return Err(Error::msg(format!("{iri}: {} is no path", quad.object)));
                };
                named.push((subject, path.value().to_owned()));
            }
            BRIDGE_VERDICT => verdicts.entry(subject).or_default().push(quad.object),
            BRIDGE_NAMES_GAP => gaps.entry(subject).or_default().push(quad.object),
            BRIDGE_LOOKUP_IN => maps.entry(subject).or_default().push(quad.object),
            BRIDGE_LOOKUP_NAMES_GAP => misses.entry(subject).or_default().push(quad.object),
            _ => {}
        }
    }
    let mut entries = Vec::new();
    for (subject, path) in named {
        if !typed.contains(&subject) {
            continue;
        }
        let said = |by: &HashMap<String, Vec<Term>>, predicate: &str| {
            let objects: &[Term] = by.get(&subject).map(Vec::as_slice).unwrap_or_default();
            declared(objects, iri, &path, predicate)
        };
        entries.push(Entry {
            verdict: said(&verdicts, "verdict")?,
            gap: said(&gaps, "gap")?,
            map: said(&maps, "bridge:lookupIn")?,
            miss: said(&misses, "bridge:lookupNamesGap")?,
            path: path.clone(),
        });
    }
    Ok(entries)
}

/// Each concept of an adapter's gap scheme, by the IRI an entry names it by. A
/// crate that names a scheme and cannot show it is refused here, as its
/// accounting is, and so is one whose concept declares a severity outside the
/// three the specification fixes: the finding it would carry is one the
/// adapter profile's own shape for a source finding refuses. One declaring two
/// severities is refused for the neighbouring reason — keeping either leaves
/// the parse order deciding how loud the gap is — and one declaring two
/// `skos:broader` for the same reason one step harder, where the parse order
/// would decide whether the gap reports at all.
fn gap_scheme(resolver: &dyn Resolver, iri: &str) -> Result<HashMap<String, Gap>> {
    let bytes = resolver
        .read(iri)
        .map_err(|e| Error::msg(format!("{iri}: {e}")))?;
    let mut scheme: HashMap<String, Gap> = HashMap::new();
    for quad in RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(iri)?
        .for_slice(&bytes)
    {
        let quad = quad.map_err(|e| Error::msg(format!("{iri}: {e}")))?;
        let NamedOrBlankNode::NamedNode(concept) = &quad.subject else {
            continue;
        };
        let predicate = quad.predicate.as_str();
        if predicate != SKOS_BROADER && predicate != SH_RESULT_SEVERITY {
            continue;
        }
        let Term::NamedNode(object) = &quad.object else {
            return Err(Error::msg(format!(
                "{iri}: {concept} declares {predicate} {}, which is no IRI",
                quad.object
            )));
        };
        let declared = scheme.entry(concept.as_str().to_owned()).or_default();
        if predicate == SKOS_BROADER {
            if let Some(already) = &declared.kind {
                return Err(Error::msg(format!(
                    "{iri}: {concept} declares skos:broader {already} and {object}; a gap declares \
                     at most one"
                )));
            }
            declared.kind = Some(object.as_str().to_owned());
        } else {
            if !SEVERITIES.contains(&object.as_str()) {
                return Err(Error::msg(format!(
                    "{iri}: {concept} declares sh:resultSeverity {object}; a gap's severity is \
                     sh:Info, sh:Warning or sh:Violation"
                )));
            }
            if let Some(already) = &declared.severity {
                return Err(Error::msg(format!(
                    "{iri}: {concept} declares sh:resultSeverity {already} and {object}; a gap \
                     declares at most one"
                )));
            }
            declared.severity = Some(object.as_str().to_owned());
        }
    }
    Ok(scheme)
}

/// Every `skos:notation` a concept map carries, which is the whole of what a
/// Bridge reads from one. A file holding other than one `skos:ConceptScheme` is
/// refused: which scheme a value is looked up in would otherwise be the parse
/// order's to decide, or nothing's.
fn concept_map(resolver: &dyn Resolver, iri: &str) -> Result<HashSet<String>> {
    let bytes = resolver
        .read(iri)
        .map_err(|e| Error::msg(format!("{iri}: {e}")))?;
    let mut schemes = HashSet::new();
    let mut notations = HashSet::new();
    for quad in RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(iri)?
        .for_slice(&bytes)
    {
        let quad = quad.map_err(|e| Error::msg(format!("{iri}: {e}")))?;
        match quad.predicate.as_str() {
            RDF_TYPE if matches!(&quad.object, Term::NamedNode(scheme) if scheme.as_str() == SKOS_CONCEPT_SCHEME) =>
            {
                schemes.insert(quad.subject.to_string());
            }
            SKOS_NOTATION => {
                let Term::Literal(notation) = &quad.object else {
                    return Err(Error::msg(format!(
                        "{iri}: {} is no skos:notation",
                        quad.object
                    )));
                };
                notations.insert(notation.value().to_owned());
            }
            _ => {}
        }
    }
    if schemes.len() != 1 {
        return Err(Error::msg(format!(
            "{iri}: the file holds {} concept schemes; a bridge:lookupIn names a file holding one",
            schemes.len()
        )));
    }
    Ok(notations)
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

    let scheme = adapter
        .gap_scheme
        .as_deref()
        .map(|iri| gap_scheme(resolver, iri))
        .transpose()?
        .unwrap_or_default();

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
        accounting: adapter
            .source_accounting
            .as_deref()
            .map(|iri| Accounting::of(entries(resolver, iri)?, &scheme, iri, resolver))
            .transpose()?,
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
    let paths = match &prepared.accounting {
        Some(accounting) => Paths::Kept {
            valued: accounting.lookups.keys().cloned().collect(),
        },
        None => Paths::Dropped,
    };
    let mut lift = lift_text(Cow::Borrowed(&text), Some(&prepared.unit), paths)?;
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
        let mark = findings.len();
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
        if let Some(accounting) = &prepared.accounting {
            for occurrence in unit.occurrences() {
                if !accounting.paths.contains(&occurrence.path) {
                    findings.extend(annotation::unaccounted(
                        &record,
                        &occurrence.path,
                        occurrence.within.as_deref(),
                        occurrence.count,
                    )?);
                }
                for reported in accounting
                    .reported
                    .get(&occurrence.path)
                    .into_iter()
                    .flatten()
                {
                    findings.extend(annotation::gap(
                        &record,
                        &reported.gap,
                        &occurrence.path,
                        occurrence.within.as_deref(),
                        &reported.severity,
                        occurrence.count,
                    )?);
                }
            }
            // A graph has no order of its own, as the gaps above have none.
            let mut held: Vec<&Valued> = unit.values().iter().collect();
            held.sort_by(|one, two| (&one.path, &one.value).cmp(&(&two.path, &two.value)));
            for valued in held {
                let key = key(&valued.value);
                if key.is_empty() {
                    continue;
                }
                for lookup in accounting
                    .lookups
                    .get(&valued.path)
                    .into_iter()
                    .flatten()
                    .filter(|lookup| !lookup.notations.contains(&key))
                {
                    findings.extend(annotation::lookup(
                        &record,
                        &lookup.gap,
                        &valued.value,
                        valued.within.as_deref(),
                        &lookup.severity,
                        valued.count,
                    )?);
                }
            }
        }
        ms.findings += at.elapsed();

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

        let at = Instant::now();
        let reports = xpath::unresolved(&record, &unit.xml, &findings[mark..])?;
        findings.extend(reports);
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
        let mark = findings.len();
        for broken in schema.errors(&text)? {
            findings.extend(annotation::violation(
                &record,
                &broken.body(),
                broken.within(),
            )?);
        }
        let reports = xpath::unresolved(&record, &text, &findings[mark..])?;
        findings.extend(reports);
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
