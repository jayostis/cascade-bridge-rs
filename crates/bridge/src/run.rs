// Running an adapter on one document, per unit: lift the unit, load the tables
// beside it, run every mapping and union the graphs, then run every findings
// query and concatenate the rows. Each unit gets a store of its own, so no
// query can see another unit.
//
// Everything an adapter runs is read and parsed once, here, before any
// document: a query's text is never handed to the engine twice.
use crate::error::{Error, Result};
use crate::lift::lift_slice;
use crate::load::{subject, value, Adapter};
use crate::rdf::SCHEMA_ENCODING_FORMAT;
use crate::resolver::Resolver;
use oxigraph::model::{GraphName, Quad, Term};
use oxigraph::sparql::{PreparedSparqlQuery, QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use oxrdfio::{RdfFormat, RdfParser};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub source_field: String,
    pub reason: String,
    pub severity: String,
    pub context: String,
}

const FINDING_KEYS: [&str; 4] = ["sourceField", "reason", "severity", "context"];

impl Finding {
    /// The finding as the JSON object the sidecar holds. serde_json orders an
    /// object's members, so two equal findings have one spelling and the
    /// multiset comparison can key on it.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "sourceField": self.source_field,
            "reason": self.reason,
            "severity": self.severity,
            "context": self.context,
        })
    }
}

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
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Ms {
    pub lift: Duration,
    pub load: Duration,
    pub detect: Duration,
    pub mappings: Duration,
    pub findings: Duration,
}

pub struct Conversion {
    pub quads: Vec<Quad>,
    pub findings: Vec<Finding>,
    pub units: usize,
    /// The detect query's answer over the skeleton, when the adapter has one.
    pub detected: Option<bool>,
    pub ms: Ms,
}

/// The prologue's PREFIX declarations. SPARQL keeps them out of the algebra a
/// parser returns, and they are the only names for these namespaces anyone has
/// written down.
fn prologue_prefixes(text: &str) -> Vec<(String, String)> {
    let mut prefixes = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        if !line
            .get(..6)
            .is_some_and(|keyword| keyword.eq_ignore_ascii_case("PREFIX"))
        {
            continue;
        }
        let rest = &line[6..];
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let Some((name, namespace)) = rest.trim_start().split_once(':') else {
            continue;
        };
        if let Some(namespace) = namespace
            .trim()
            .strip_prefix('<')
            .and_then(|n| n.strip_suffix('>'))
        {
            prefixes.push((name.to_owned(), namespace.to_owned()));
        }
    }
    prefixes
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
        findings_queries.push(query(resolver, iri, Form::Select, "findings query")?);
    }
    let detect = adapter
        .detect_query
        .as_deref()
        .map(|iri| query(resolver, iri, Form::Ask, "detect query"))
        .transpose()?;

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
    })
}

fn finding(solution: &oxigraph::sparql::QuerySolution, query: &str) -> Result<Finding> {
    let mut got = Vec::with_capacity(FINDING_KEYS.len());
    for key in FINDING_KEYS {
        match solution.get(key) {
            Some(Term::Literal(l)) => got.push(l.value().to_owned()),
            Some(Term::NamedNode(n)) => got.push(n.as_str().to_owned()),
            _ => {
                return Err(Error::msg(format!(
                    "findings query {query} left ?{key} unbound or bound to a term with no lexical form, such as a blank node"
                )))
            }
        }
    }
    let mut got = got.into_iter();
    Ok(Finding {
        source_field: got.next().expect("four keys"),
        reason: got.next().expect("four keys"),
        severity: got.next().expect("four keys"),
        context: got.next().expect("four keys"),
    })
}

pub fn convert(prepared: &Prepared, xml: &[u8]) -> Result<Conversion> {
    let mut ms = Ms::default();
    let mut quads = Vec::new();
    let mut findings = Vec::new();
    let mut units = 0;

    let mut lift = lift_slice(xml, Some(&prepared.unit))?;
    loop {
        let at = Instant::now();
        let Some(store) = lift.next_unit()? else {
            break;
        };
        ms.lift += at.elapsed();
        units += 1;

        let at = Instant::now();
        store.extend(prepared.tables.iter().cloned())?;
        ms.load += at.elapsed();

        let at = Instant::now();
        for mapping in &prepared.mappings {
            let QueryResults::Graph(triples) = mapping.on(&store)? else {
                return Err(Error::msg(format!(
                    "mapping {} is not a CONSTRUCT",
                    mapping.iri
                )));
            };
            for triple in triples {
                let triple = triple?;
                quads.push(Quad::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphName::DefaultGraph,
                ));
            }
        }
        ms.mappings += at.elapsed();

        let at = Instant::now();
        for findings_query in &prepared.findings_queries {
            let QueryResults::Solutions(rows) = findings_query.on(&store)? else {
                return Err(Error::msg(format!(
                    "findings query {} is not a SELECT",
                    findings_query.iri
                )));
            };
            for row in rows {
                findings.push(finding(&row?, &findings_query.iri)?);
            }
        }
        ms.findings += at.elapsed();
    }

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
    use super::prologue_prefixes;

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
    fn reads_no_prefix_out_of_a_comment_or_a_word_that_merely_starts_with_one() {
        assert_eq!(
            prologue_prefixes("# PREFIX ex: <urn:example:catalog#>\nPREFIXES ex: <urn:x#>"),
            Vec::new()
        );
    }
}
