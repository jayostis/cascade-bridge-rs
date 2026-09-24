use crate::annotation::{self, Minted, Record};
use crate::decode::{decode, XML_SPACE};
use crate::error::{Error, Result};
use crate::lift::{lift_text, Paths, Valued};
use crate::load::{subject, value, values, Adapter};
use crate::rdf::{
    BRIDGE_CARRIED_IN_PART, BRIDGE_LOOKUP_IN, BRIDGE_LOOKUP_NAMES_GAP, BRIDGE_NAMES_GAP,
    BRIDGE_NO_HOME, BRIDGE_NO_PREDICATE, BRIDGE_PATH_ENTRY, BRIDGE_SOURCE_LACKS_REQUIRED,
    BRIDGE_SOURCE_PATH, BRIDGE_STAMP_PREDICATE, BRIDGE_VALUE_NOT_MAPPED, BRIDGE_VERDICT,
    FINDINGS_PREFIXES, RDF_TYPE, SCHEMA_ENCODING_FORMAT, SH_INFO, SH_RESULT_SEVERITY, SH_VIOLATION,
    SH_WARNING, SKOS_BROADER, SKOS_CONCEPT_SCHEME, SKOS_NOTATION,
};
use crate::resolver::Resolver;
use crate::validate::{self, Schema};
use crate::vocabulary::Vocabulary;
use crate::xpath;
use oxigraph::model::{GraphName, NamedOrBlankNode, Quad, Term};
use oxigraph::sparql::{PreparedSparqlQuery, QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use oxrdfio::{RdfFormat, RdfParser};
use spargebra::algebra::{AggregateExpression, Expression, GraphPattern, OrderExpression};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
// std's clock panics on wasm32-unknown-unknown, where this one asks the host.
use web_time::Instant;

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
    pub prefixes: Vec<(String, String)>,
    prepared: PreparedSparqlQuery,
}

impl Query {
    fn on(&self, store: &Store) -> Result<QueryResults<'static>> {
        // Copies the algebra already parsed; the text is not parsed again.
        Ok(self.prepared.clone().on_store(store).execute()?)
    }

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
    pub tables: Vec<Quad>,
    /// Every name the mappings give a namespace, the first binding of a name winning.
    pub prefixes: Vec<(String, String)>,
    pub findings_prefixes: Vec<(String, String)>,
    envelopes: Vec<Envelope>,
    source_schema: Option<Schema>,
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
    pub findings: Vec<Quad>,
    pub units: usize,
    pub detected: Option<bool>,
    pub ms: Ms,
}

impl Conversion {
    pub fn annotations(&self) -> usize {
        annotation::annotations(&self.findings)
    }

    /// `quads` holds a triple constructed for every record once per record, the graph once.
    pub fn triples(&self) -> usize {
        self.quads.iter().collect::<HashSet<&Quad>>().len()
    }
}

pub struct Source<'a> {
    pub iri: &'a str,
    pub envelope: Option<&'a str>,
    pub xml: &'a [u8],
}

/// Skips whitespace and comments.
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

fn keyword<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let rest = text.get(word.len()..)?;
    (text[..word.len()].eq_ignore_ascii_case(word)
        && rest.starts_with(|c: char| c.is_whitespace() || c == '#' || c == '<'))
    .then_some(rest)
}

fn declared_name(text: &str) -> Option<(&str, &str)> {
    let (name, rest) = text.split_once(':')?;
    (!name.contains(char::is_whitespace)).then_some((name, rest))
}

fn iri_ref(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix('<')?;
    let end = rest.find('>')?;
    Some((&rest[..end], &rest[end + 1..]))
}

/// Read from the text: SPARQL keeps PREFIX declarations out of the algebra.
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

fn document_selector(document: Option<String>, described: Option<&str>) -> String {
    document
        .or_else(|| described.map(|element| format!("/{element}")))
        .unwrap_or_else(|| "/*".to_owned())
}

struct Entry {
    path: String,
    verdict: Option<String>,
    gap: Option<String>,
    /// The concept map a value is looked up in; `miss` is the gap for a value it lacks.
    map: Option<String>,
    miss: Option<String>,
}

#[derive(Default)]
struct Gap {
    kind: Option<String>,
    severity: Option<String>,
}

const NAMES_A_GAP: [&str; 2] = [BRIDGE_NO_HOME, BRIDGE_CARRIED_IN_PART];

/// The kinds of gap true of the path, not of what a record holds at it.
const REPORTS: [&str; 2] = [BRIDGE_NO_PREDICATE, BRIDGE_SOURCE_LACKS_REQUIRED];

const SEVERITIES: [&str; 3] = [SH_INFO, SH_WARNING, SH_VIOLATION];

struct Reported {
    gap: String,
    severity: String,
}

struct Lookup {
    gap: String,
    severity: String,
    notations: Arc<HashSet<String>>,
}

/// The form a `skos:notation` is written in.
fn key(value: &str) -> String {
    value.trim_matches(XML_SPACE).to_lowercase()
}

struct Accounting {
    paths: HashSet<String>,
    reported: HashMap<String, Vec<Reported>>,
    lookups: HashMap<String, Vec<Lookup>>,
}

impl Accounting {
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
        // Sorted, so a run's output does not depend on which way a hash fell.
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

/// A `bridge:sourcePath` that is no literal is refused: dropping it would report the
/// path as unaccounted.
fn entries(resolver: &dyn Resolver, iri: &str) -> Result<Vec<Entry>> {
    let bytes = resolver.read(iri)?;
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

type Prefixes = Vec<(String, String)>;

fn gap_scheme(resolver: &dyn Resolver, iri: &str) -> Result<(HashMap<String, Gap>, Prefixes)> {
    let bytes = resolver.read(iri)?;
    let mut scheme: HashMap<String, Gap> = HashMap::new();
    let mut parser = RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(iri)?
        .for_slice(&bytes);
    for quad in parser.by_ref() {
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
    let prefixes = parser
        .prefixes()
        .map(|(name, namespace)| (name.to_owned(), namespace.to_owned()))
        .collect();
    Ok((scheme, prefixes))
}

fn concept_map(resolver: &dyn Resolver, iri: &str) -> Result<HashSet<String>> {
    let bytes = resolver.read(iri)?;
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

fn holds_a_service(pattern: &GraphPattern) -> bool {
    match pattern {
        GraphPattern::Service { .. } => true,
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => false,
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => holds_a_service(left) || holds_a_service(right),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            holds_a_service(left)
                || holds_a_service(right)
                || expression.as_ref().is_some_and(asks_a_service)
        }
        GraphPattern::Filter { expr, inner } => asks_a_service(expr) || holds_a_service(inner),
        GraphPattern::Extend {
            inner, expression, ..
        } => asks_a_service(expression) || holds_a_service(inner),
        GraphPattern::OrderBy { inner, expression } => {
            holds_a_service(inner)
                || expression.iter().any(|order| match order {
                    OrderExpression::Asc(e) | OrderExpression::Desc(e) => asks_a_service(e),
                })
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            holds_a_service(inner)
                || aggregates.iter().any(|(_, aggregate)| match aggregate {
                    AggregateExpression::CountSolutions { .. } => false,
                    AggregateExpression::FunctionCall { expr, .. } => asks_a_service(expr),
                })
        }
        GraphPattern::Graph { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => holds_a_service(inner),
    }
}

fn asks_a_service(expression: &Expression) -> bool {
    match expression {
        Expression::Exists(pattern) => holds_a_service(pattern),
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => false,
        Expression::Or(one, two)
        | Expression::And(one, two)
        | Expression::Equal(one, two)
        | Expression::SameTerm(one, two)
        | Expression::Greater(one, two)
        | Expression::GreaterOrEqual(one, two)
        | Expression::Less(one, two)
        | Expression::LessOrEqual(one, two)
        | Expression::Add(one, two)
        | Expression::Subtract(one, two)
        | Expression::Multiply(one, two)
        | Expression::Divide(one, two) => asks_a_service(one) || asks_a_service(two),
        Expression::UnaryPlus(one) | Expression::UnaryMinus(one) | Expression::Not(one) => {
            asks_a_service(one)
        }
        Expression::In(one, many) => asks_a_service(one) || many.iter().any(asks_a_service),
        Expression::If(one, two, three) => {
            asks_a_service(one) || asks_a_service(two) || asks_a_service(three)
        }
        Expression::Coalesce(many) | Expression::FunctionCall(_, many) => {
            many.iter().any(asks_a_service)
        }
    }
}

fn what_fetches(query: &spargebra::Query) -> Option<&'static str> {
    let (dataset, pattern) = match query {
        spargebra::Query::Select {
            dataset, pattern, ..
        }
        | spargebra::Query::Construct {
            dataset, pattern, ..
        }
        | spargebra::Query::Describe {
            dataset, pattern, ..
        }
        | spargebra::Query::Ask {
            dataset, pattern, ..
        } => (dataset, pattern),
    };
    if holds_a_service(pattern) {
        return Some("a SERVICE pattern");
    }
    let dataset = dataset.as_ref()?;
    if dataset
        .named
        .as_ref()
        .is_some_and(|named| !named.is_empty())
    {
        return Some("a FROM NAMED clause");
    }
    (!dataset.default.is_empty()).then_some("a FROM clause")
}

fn query(resolver: &dyn Resolver, iri: &str, expected: Form, what: &str) -> Result<Query> {
    let text =
        String::from_utf8(resolver.read(iri)?).map_err(|e| Error::msg(format!("{iri}: {e}")))?;
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
    if let Some(held) = what_fetches(&parsed) {
        return Err(Error::msg(format!(
            "{what} {iri} holds {held}; a query reads the dataset built for its unit, \
             and nothing is fetched"
        )));
    }
    Ok(Query {
        iri: iri.to_owned(),
        form,
        prefixes: prologue_prefixes(&text),
        prepared: SparqlEvaluator::new().for_query(parsed),
    })
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

pub fn prepare(adapter: &Adapter, resolver: &dyn Resolver) -> Result<Prepared> {
    if adapter.mappings.is_empty() {
        return Err(Error::msg("the adapter names no bridge:mapping"));
    }
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
            tables.push(quad.map_err(|e| Error::msg(format!("{iri}: {e}")))?);
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

    let (scheme, gap_prefixes) = adapter
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

    let gap_severities = scheme
        .iter()
        .filter_map(|(concept, gap)| Some((concept.clone(), gap.severity.clone()?)))
        .collect();

    let vocabulary = Vocabulary::read(&adapter.vocabulary_files, stamps(adapter)?, resolver)?;

    Ok(Prepared {
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
        gap_severities,
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
    let text = decode(source.xml)?;
    let followed = xpath::Followed::default();
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
            // Sorted, so a run's output does not depend on which way a hash fell.
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
        let mut produced = Vec::new();
        for mapping in &prepared.mappings {
            produced.extend(mapping.graph(&unit.store)?);
        }
        ms.mappings += at.elapsed();

        let at = Instant::now();
        if let Some(vocabulary) = &prepared.vocabulary {
            findings.extend(vocabulary.findings(&record, &produced)?);
        }
        ms.validation += at.elapsed();
        quads.extend(produced);

        let at = Instant::now();
        for findings_query in &prepared.findings_queries {
            let constructed = findings_query.graph(&unit.store)?;
            findings.extend(annotation::about(
                &record,
                &findings_query.iri,
                constructed,
                &prepared.gap_severities,
            )?);
        }
        ms.findings += at.elapsed();

        let at = Instant::now();
        // A record was read, so the document element was: no envelope is needed yet.
        let element = document_selector(lift.document_selector(), None);
        let reports = followed.unresolved(
            &Record {
                source: source.iri,
                selector: &element,
            },
            &unit.xml,
            &findings[mark..],
        )?;
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
        // This Bridge wrote these addresses, so they are not followed as an adapter's are.
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
