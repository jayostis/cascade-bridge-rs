// Executing a test manifest. Each entry is judged by the rule its type
// carries, as the rdfs:comment on that type in the specification's vocabulary
// states it.
use crate::annotation;
use crate::error::{Error, Result};
use crate::load::{as_subject, list, objects, subject, term_value, value, values, Adapter};
use crate::rdf::{
    canonical_lines, BRIDGE_DATASET, BRIDGE_ENVELOPE, BRIDGE_EXPECTED_FINDINGS,
    BRIDGE_EXPECTED_GRAPH, BRIDGE_INPUT, BRIDGE_INPUT_ONLY, BRIDGE_ISOMORPHIC, BRIDGE_SPARQL_1_1,
    BRIDGE_STAMP_PREDICATE, MF_ACTION, MF_ENTRIES, MF_NAME, MF_RESULT, RDF_TYPE,
};
use crate::resolver::Resolver;
use crate::run::{convert, prepare, Prepared, Source};
use oxigraph::model::{NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::HashSet;
use std::time::{Duration, Instant};

pub const OFFERED_PROFILES: [&str; 1] = [BRIDGE_SPARQL_1_1];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Passed,
    Failed,
    CantTell,
    Untested,
    Inapplicable,
}

impl Outcome {
    /// The EARL outcome's local name, which is also what the command prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::CantTell => "cantTell",
            Self::Untested => "untested",
            Self::Inapplicable => "inapplicable",
        }
    }
}

pub struct EntryResult {
    /// The entry as the manifest lists it. Only an IRI names a test outside
    /// the manifest; a blank node or a literal does not.
    pub entry: Term,
    pub name: String,
    pub type_iri: String,
    pub outcome: Outcome,
    pub description: String,
    pub elapsed: Duration,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RunOptions {
    pub datasets: bool,
}

fn sample<'a>(lines: impl IntoIterator<Item = &'a String>, n: usize) -> String {
    lines
        .into_iter()
        .take(n)
        .map(|line| {
            if line.chars().count() > 160 {
                format!("{}…", line.chars().take(160).collect::<String>())
            } else {
                line.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn without(quads: Vec<Quad>, ignore: &HashSet<String>) -> Vec<Quad> {
    quads
        .into_iter()
        .filter(|q| !ignore.contains(q.predicate.as_str()))
        .collect()
}

/// A graph a result names, parsed against its own IRI, so a finding writing
/// its document as a relative reference names the same document the entry does.
fn graph_at(bytes: &[u8], iri: &str) -> Result<Vec<Quad>> {
    let mut quads = Vec::new();
    for quad in RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(iri)?
        .for_slice(bytes)
    {
        quads.push(quad?);
    }
    Ok(quads)
}

struct Entry<'a> {
    adapter: &'a Adapter,
    resolver: &'a dyn Resolver,
    setup: &'a Prepared,
    node: NamedOrBlankNode,
    manifest_ignore: &'a [String],
}

impl Entry<'_> {
    /// The rule of bridge:IsomorphicConversionTest and bridge:InputOnlyTest,
    /// which share everything up to the judgement.
    fn judge(&self, type_iri: &str) -> Result<(Outcome, String)> {
        let graph = &self.adapter.graph;
        let action = objects(graph, &self.node, MF_ACTION)?
            .first()
            .and_then(as_subject);
        let input = action
            .as_ref()
            .map(|a| value(graph, a, BRIDGE_INPUT))
            .transpose()?
            .flatten()
            .ok_or_else(|| Error::msg("the entry's action names no bridge:input"))?;
        let envelope = action
            .as_ref()
            .map(|a| value(graph, a, BRIDGE_ENVELOPE))
            .transpose()?
            .flatten();
        let run = convert(
            self.setup,
            Source {
                iri: &input,
                envelope: envelope.as_deref(),
                xml: &self.resolver.read(&input)?,
            },
        )?;
        let detect = if run.detected == Some(false) {
            "; the detect query is false for this input (reported, not judged)"
        } else {
            ""
        };

        if type_iri == BRIDGE_INPUT_ONLY {
            return Ok((
                Outcome::CantTell,
                format!(
                    "input-only: {} unit(s), {} triples and {} finding(s) recorded, not judged (bridge:InputOnlyTest){detect}",
                    run.units,
                    run.quads.len(),
                    run.annotations()
                ),
            ));
        }

        let result = objects(graph, &self.node, MF_RESULT)?
            .first()
            .and_then(as_subject);
        let graph_iri = result
            .as_ref()
            .map(|r| value(graph, r, BRIDGE_EXPECTED_GRAPH))
            .transpose()?
            .flatten()
            .ok_or_else(|| Error::msg("the entry's result names no bridge:expectedGraph"))?;
        let own = values(graph, &self.node, BRIDGE_STAMP_PREDICATE)?;
        let ignore: HashSet<String> = if own.is_empty() {
            self.manifest_ignore.iter().cloned().collect()
        } else {
            own.into_iter().collect()
        };

        let annotations = run.annotations();
        let expected = graph_at(&self.resolver.read(&graph_iri)?, &graph_iri)?;
        let expected = canonical_lines(without(expected, &ignore))?;
        let produced = canonical_lines(without(run.quads, &ignore))?;
        let graph_missing: Vec<String> = expected.difference(&produced).cloned().collect();
        let graph_extra: Vec<String> = produced.difference(&expected).cloned().collect();
        let graph_ok = graph_missing.is_empty() && graph_extra.is_empty();

        let findings_iri = result
            .as_ref()
            .map(|r| value(graph, r, BRIDGE_EXPECTED_FINDINGS))
            .transpose()?
            .flatten();
        let mut findings_ok = true;
        let mut findings_text =
            "findings not compared: the entry names no bridge:expectedFindings".to_owned();
        if let Some(iri) = findings_iri {
            let want = graph_at(&self.resolver.read(&iri)?, &iri)?;
            let wanted = annotation::annotations(&want);
            let want = canonical_lines(want)?;
            let got = canonical_lines(run.findings)?;
            let missing: Vec<String> = want.difference(&got).cloned().collect();
            let extra: Vec<String> = got.difference(&want).cloned().collect();
            findings_ok = missing.is_empty() && extra.is_empty();
            findings_text = if findings_ok {
                format!("findings isomorphic ({wanted} annotation(s))")
            } else {
                format!(
                    "findings differ: {annotations} annotation(s) produced, {wanted} expected; {} line(s) missing, {} extra (missing: {}; extra: {})",
                    missing.len(),
                    extra.len(),
                    sample(&missing, 2),
                    sample(&extra, 2)
                )
            };
        }

        let graph_text = if graph_ok {
            format!("graph isomorphic ({} triples)", expected.len())
        } else {
            format!(
                "graph differs: {} missing, {} extra (missing: {}; extra: {})",
                graph_missing.len(),
                graph_extra.len(),
                sample(&graph_missing, 2),
                sample(&graph_extra, 2)
            )
        };
        let outcome = if graph_ok && findings_ok {
            Outcome::Passed
        } else {
            Outcome::Failed
        };
        Ok((outcome, format!("{graph_text}; {findings_text}{detect}")))
    }
}

pub fn run_manifest(
    adapter: &Adapter,
    resolver: &dyn Resolver,
    options: RunOptions,
) -> Result<Vec<EntryResult>> {
    let graph = &adapter.graph;
    let manifest = subject(&adapter.manifest)?;
    let entries = list(graph, objects(graph, &manifest, MF_ENTRIES)?.first())?;
    let manifest_ignore = values(graph, &manifest, BRIDGE_STAMP_PREDICATE)?;

    let unoffered: Vec<String> = adapter
        .required_profiles
        .iter()
        .filter(|p| !OFFERED_PROFILES.contains(&p.as_str()))
        .cloned()
        .collect();
    let setup = unoffered.is_empty().then(|| prepare(adapter, resolver));

    let mut results = Vec::new();
    for entry in entries {
        let start = Instant::now();
        let node = as_subject(&entry);
        let (mut types, name) = match &node {
            Some(node) => (values(graph, node, RDF_TYPE)?, value(graph, node, MF_NAME)?),
            None => (Vec::new(), None),
        };
        types.sort();
        let known = [BRIDGE_ISOMORPHIC, BRIDGE_INPUT_ONLY, BRIDGE_DATASET];
        let type_iri = known
            .iter()
            .find(|t| types.iter().any(|got| got == *t))
            .map(|t| (*t).to_owned())
            .or_else(|| types.first().cloned())
            .unwrap_or_default();
        let name = name.unwrap_or_else(|| term_value(&entry));

        let (outcome, description) = match (&setup, &node) {
            (None, _) => (
                Outcome::Inapplicable,
                format!(
                    "the adapter requires {}, which this Bridge does not offer",
                    unoffered.join(", ")
                ),
            ),
            (Some(Err(e)), _) => (
                Outcome::Failed,
                format!("the adapter could not be prepared: {e}"),
            ),
            (Some(Ok(_)), None) => (
                Outcome::Inapplicable,
                "the entry is a literal, not a test".to_owned(),
            ),
            (Some(Ok(_)), Some(_)) if type_iri == BRIDGE_DATASET => (
                Outcome::Untested,
                if options.datasets {
                    "--datasets was given, but streaming a referenced dataset is not implemented in this Bridge yet".to_owned()
                } else {
                    "datasets are not fetched; pass --datasets to run them".to_owned()
                },
            ),
            (Some(Ok(_)), Some(_))
                if type_iri != BRIDGE_ISOMORPHIC && type_iri != BRIDGE_INPUT_ONLY =>
            {
                (
                    Outcome::Inapplicable,
                    format!(
                        "entry type {} is not one this Bridge knows",
                        if type_iri.is_empty() {
                            "(none)"
                        } else {
                            &type_iri
                        }
                    ),
                )
            }
            (Some(Ok(setup)), Some(node)) => {
                let entry = Entry {
                    adapter,
                    resolver,
                    setup,
                    node: node.clone(),
                    manifest_ignore: &manifest_ignore,
                };
                match entry.judge(&type_iri) {
                    Ok(verdict) => verdict,
                    Err(e) => (Outcome::Failed, format!("error: {e}")),
                }
            }
        };

        results.push(EntryResult {
            entry,
            name,
            type_iri,
            outcome,
            description,
            elapsed: start.elapsed(),
        });
    }
    Ok(results)
}
