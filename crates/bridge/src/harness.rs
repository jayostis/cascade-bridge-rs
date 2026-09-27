use crate::annotation;
use crate::error::{Error, Result};
use crate::load::{as_subject, list, objects, one, subject, term_value, values, Adapter};
use crate::rdf::{canonical_lines, canonical_parts};
use crate::records::Supplied;
use crate::resolver::Resolver;
use crate::run::{convert, prepare, Conversion, Prepared, Source};
use crate::terms::{
    BRIDGE_ARRIVED_AS, BRIDGE_CONVERSION, BRIDGE_DATASET, BRIDGE_ENVELOPE,
    BRIDGE_EXPECTED_FINDINGS, BRIDGE_EXPECTED_GRAPH, BRIDGE_FACTS, BRIDGE_IDENTITY_RELATION_TEST,
    BRIDGE_INPUT, BRIDGE_INPUT_ONLY, BRIDGE_ISOMORPHIC, BRIDGE_SAME_RECORD, BRIDGE_SELECTOR,
    BRIDGE_SPARQL_1_1, MF_ACTION, MF_ENTRIES, MF_NAME, MF_RESULT, PROV_AGENT,
    PROV_QUALIFIED_ASSOCIATION, PROV_SPECIALIZATION_OF, RDF_TYPE,
};
use oxigraph::model::{NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Duration;
// std's clock panics on wasm32-unknown-unknown, where this one asks the host.
use web_time::Instant;

pub(crate) const OFFERED_PROFILES: [&str; 1] = [BRIDGE_SPARQL_1_1];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Outcome {
    Passed,
    Failed,
    CantTell,
    Untested,
    Inapplicable,
}

impl Outcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::CantTell => "cantTell",
            Self::Untested => "untested",
            Self::Inapplicable => "inapplicable",
        }
    }
}

pub(crate) struct EntryResult {
    pub(crate) entry: Term,
    pub(crate) name: String,
    pub(crate) outcome: Outcome,
    pub(crate) description: String,
    pub(crate) elapsed: Duration,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct RunOptions {
    pub(crate) datasets: bool,
}

/// The widths a description cuts a graph's line and a finding at.
const LINE: usize = 160;
const FINDING: usize = 1200;

fn sample<'a>(lines: impl IntoIterator<Item = &'a String>, n: usize, width: usize) -> String {
    lines
        .into_iter()
        .take(n)
        .map(|line| {
            if line.chars().count() > width {
                format!("{}…", line.chars().take(width).collect::<String>())
            } else {
                line.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn beyond(these: &[String], those: &[String]) -> Vec<String> {
    let mut spare: HashMap<&str, usize> = HashMap::new();
    for one in those {
        *spare.entry(one.as_str()).or_default() += 1;
    }
    these
        .iter()
        .filter(|one| match spare.get_mut(one.as_str()) {
            Some(count) if *count > 0 => {
                *count -= 1;
                false
            }
            _ => true,
        })
        .cloned()
        .collect()
}

fn without_the_release(quads: Vec<Quad>) -> Vec<Quad> {
    let associations: HashSet<Term> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == PROV_QUALIFIED_ASSOCIATION)
        .map(|q| q.object.clone())
        .collect();
    let releases: HashSet<Term> = quads
        .iter()
        .filter(|q| q.predicate.as_str() == PROV_AGENT)
        .filter(|q| associations.contains(&Term::from(q.subject.clone())))
        .map(|q| q.object.clone())
        .collect();
    quads
        .into_iter()
        .filter(|q| !releases.contains(&Term::from(q.subject.clone())))
        .collect()
}

fn graph_at(bytes: &[u8], iri: &str) -> Result<Vec<Quad>> {
    let mut quads = Vec::new();
    for quad in RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(iri)?
        .for_slice(bytes)
    {
        quads.push(quad.map_err(|e| Error::msg(format!("{iri}: {e}")))?);
    }
    Ok(quads)
}

struct Entry<'a> {
    adapter: &'a Adapter,
    resolver: &'a dyn Resolver,
    setup: &'a Prepared,
    node: NamedOrBlankNode,
}

impl Entry<'_> {
    /// The conversion an action, or one bridge:conversion of it, describes, and the bytes it read.
    fn converted(&self, action: Option<&NamedOrBlankNode>) -> Result<(Conversion, Vec<u8>)> {
        let graph = &self.adapter.graph;
        let said = |predicate| {
            action
                .map(|a| one(graph, a, predicate))
                .transpose()
                .map(Option::flatten)
        };
        let input = said(BRIDGE_INPUT)?
            .ok_or_else(|| Error::msg("the entry's action names no bridge:input"))?;
        let envelope = said(BRIDGE_ENVELOPE)?;
        let facts = said(BRIDGE_FACTS)?
            .map(|iri| Ok::<_, Error>((self.resolver.read(&iri)?, iri)))
            .transpose()?;
        let bytes = self.resolver.read(&input)?;
        let run = convert(
            self.setup,
            Source {
                iri: &input,
                envelope: envelope.as_deref(),
                bytes: &bytes,
                facts: facts.as_ref().map(|(turtle, iri)| Supplied { iri, turtle }),
            },
        )?;
        Ok((run, bytes))
    }

    /// The one record a conversion names: of the version whose arrival stood at its
    /// bridge:selector, or, where it names none, of the one version arriving at all.
    fn record_named(&self, conversion: &NamedOrBlankNode) -> Result<String> {
        let graph = &self.adapter.graph;
        let selector = one(graph, conversion, BRIDGE_SELECTOR)?;
        let (run, _) = self.converted(Some(conversion))?;
        let said = |subject: &NamedOrBlankNode, predicate: &str| -> Vec<String> {
            run.quads
                .iter()
                .filter(|q| &q.subject == subject && q.predicate.as_str() == predicate)
                .map(|q| term_value(&q.object))
                .collect()
        };
        let records: BTreeSet<String> = run
            .quads
            .iter()
            .filter(|q| q.predicate.as_str() == BRIDGE_ARRIVED_AS)
            .filter(|q| {
                selector
                    .as_ref()
                    .is_none_or(|at| said(&q.subject, BRIDGE_SELECTOR).contains(at))
            })
            .filter_map(|q| as_subject(&q.object))
            .flat_map(|version| said(&version, PROV_SPECIALIZATION_OF))
            .collect();
        match (records.len(), records.first()) {
            (1, Some(record)) => Ok(record.clone()),
            (named, _) => Err(Error::msg(format!(
                "a conversion{} names {named} record(s); each conversion of an identity relation test names one",
                selector.map_or_else(String::new, |at| format!(" at {at}"))
            ))),
        }
    }

    fn related(&self, action: Option<&NamedOrBlankNode>) -> Result<(Outcome, String)> {
        let graph = &self.adapter.graph;
        let conversions: Vec<NamedOrBlankNode> = action
            .map(|a| objects(graph, a, BRIDGE_CONVERSION))
            .transpose()?
            .unwrap_or_default()
            .iter()
            .filter_map(as_subject)
            .collect();
        let [first, second] = conversions.as_slice() else {
            return Err(Error::msg(format!(
                "the entry's action names {} bridge:conversion; an identity relation test names two",
                conversions.len()
            )));
        };
        let result = objects(graph, &self.node, MF_RESULT)?
            .first()
            .and_then(as_subject);
        let same = match result
            .as_ref()
            .map(|r| one(graph, r, BRIDGE_SAME_RECORD))
            .transpose()?
            .flatten()
            .as_deref()
        {
            Some("true" | "1") => true,
            Some("false" | "0") => false,
            _ => {
                return Err(Error::msg(
                    "the entry's result carries no bridge:sameRecord",
                ))
            }
        };
        let (first, second) = (self.record_named(first)?, self.record_named(second)?);
        let outcome = if (first == second) == same {
            Outcome::Passed
        } else {
            Outcome::Failed
        };
        Ok((
            outcome,
            format!(
                "the conversions name {first} and {second}, which must be {}",
                if same { "one record" } else { "two records" }
            ),
        ))
    }

    fn judge(&self, type_iri: &str) -> Result<(Outcome, String)> {
        let graph = &self.adapter.graph;
        let action = objects(graph, &self.node, MF_ACTION)?
            .first()
            .and_then(as_subject);
        if type_iri == BRIDGE_IDENTITY_RELATION_TEST {
            return self.related(action.as_ref());
        }
        let (run, bytes) = self.converted(action.as_ref())?;
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
                    run.triples(),
                    run.annotations()
                ),
            ));
        }

        let result = objects(graph, &self.node, MF_RESULT)?
            .first()
            .and_then(as_subject);
        let graph_iri = result
            .as_ref()
            .map(|r| one(graph, r, BRIDGE_EXPECTED_GRAPH))
            .transpose()?
            .flatten()
            .ok_or_else(|| Error::msg("the entry's result names no bridge:expectedGraph"))?;
        let annotations = run.annotations();
        let expected = graph_at(&self.resolver.read(&graph_iri)?, &graph_iri)?;
        let expected = canonical_lines(without_the_release(expected))?;
        let produced = canonical_lines(without_the_release(run.quads))?;
        let graph_missing: Vec<String> = expected.difference(&produced).cloned().collect();
        let graph_extra: Vec<String> = produced.difference(&expected).cloned().collect();
        let graph_ok = graph_missing.is_empty() && graph_extra.is_empty();

        let findings_iri = result
            .as_ref()
            .map(|r| one(graph, r, BRIDGE_EXPECTED_FINDINGS))
            .transpose()?
            .flatten();
        let mut findings_ok = true;
        let mut findings_text =
            "findings not compared: the entry names no bridge:expectedFindings".to_owned();
        if let Some(iri) = findings_iri {
            let want = graph_at(&self.resolver.read(&iri)?, &iri)?;
            let wanted = annotation::annotations(&want);
            // Two addresses that select one node are one address.
            let source = self.setup.syntax.decode(&bytes)?;
            let spelled = self.setup.syntax.respelling(&source);
            let want = spelled.respelled(want);
            let got = spelled.respelled(run.findings);
            let missed: Vec<String> = want
                .missed
                .iter()
                .map(|said| format!("expected {said}"))
                .chain(got.missed.iter().map(|said| format!("produced {said}")))
                .collect();
            if !missed.is_empty() {
                findings_ok = false;
                findings_text = format!(
                    "findings not compared: {} address(es) select other than one node of bridge:input, which is what a finding is about ({})",
                    missed.len(),
                    sample(&missed, 4, LINE)
                );
            } else {
                let want = canonical_parts(want.findings)?;
                let got = canonical_parts(got.findings)?;
                let missing = beyond(&want, &got);
                let extra = beyond(&got, &want);
                findings_ok = missing.is_empty() && extra.is_empty();
                findings_text = if findings_ok {
                    format!("findings isomorphic ({wanted} annotation(s))")
                } else {
                    format!(
                        "findings differ: {annotations} annotation(s) produced, {wanted} expected; {} finding(s) missing, {} extra (missing: {}; extra: {})",
                        missing.len(),
                        extra.len(),
                        sample(&missing, 1, FINDING),
                        sample(&extra, 1, FINDING)
                    )
                };
            }
        }

        let graph_text = if graph_ok {
            format!("graph isomorphic ({} triples)", expected.len())
        } else {
            format!(
                "graph differs: {} missing, {} extra (missing: {}; extra: {})",
                graph_missing.len(),
                graph_extra.len(),
                sample(&graph_missing, 2, LINE),
                sample(&graph_extra, 2, LINE)
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

pub(crate) fn run_manifest(
    adapter: &Adapter,
    resolver: &dyn Resolver,
    options: RunOptions,
) -> Result<Vec<EntryResult>> {
    let graph = &adapter.graph;
    let manifest = subject(&adapter.manifest)?;
    let entries = list(graph, objects(graph, &manifest, MF_ENTRIES)?.first())?;

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
            Some(node) => (values(graph, node, RDF_TYPE)?, one(graph, node, MF_NAME)),
            None => (Vec::new(), Ok(None)),
        };
        types.sort();
        let known = [
            BRIDGE_ISOMORPHIC,
            BRIDGE_INPUT_ONLY,
            BRIDGE_DATASET,
            BRIDGE_IDENTITY_RELATION_TEST,
        ];
        let type_iri = known
            .iter()
            .find(|t| types.iter().any(|got| got == *t))
            .map(|t| (*t).to_owned())
            .or_else(|| types.first().cloned())
            .unwrap_or_default();
        let (name, unnamed) = match name {
            Ok(name) => (name.unwrap_or_else(|| term_value(&entry)), None),
            Err(e) => (term_value(&entry), Some(e)),
        };

        let (outcome, description) = match (&setup, &node, unnamed) {
            (None, _, _) => (
                Outcome::Inapplicable,
                format!(
                    "the adapter requires {}, which this Bridge does not offer",
                    unoffered.join(", ")
                ),
            ),
            (Some(Err(e)), _, _) => (
                Outcome::Failed,
                format!("the adapter could not be prepared: {e}"),
            ),
            (Some(Ok(_)), None, _) => (
                Outcome::Inapplicable,
                "the entry is a literal, not a test".to_owned(),
            ),
            (Some(Ok(_)), Some(_), Some(e)) => (Outcome::Failed, format!("error: {e}")),
            (Some(Ok(_)), Some(_), None) if type_iri == BRIDGE_DATASET => (
                Outcome::Untested,
                if options.datasets {
                    "--datasets was given, but streaming a referenced dataset is not implemented in this Bridge yet".to_owned()
                } else {
                    "datasets are not fetched; pass --datasets to run them".to_owned()
                },
            ),
            (Some(Ok(_)), Some(_), None)
                if ![
                    BRIDGE_ISOMORPHIC,
                    BRIDGE_INPUT_ONLY,
                    BRIDGE_IDENTITY_RELATION_TEST,
                ]
                .contains(&type_iri.as_str()) =>
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
            (Some(Ok(setup)), Some(node), None) => {
                let entry = Entry {
                    adapter,
                    resolver,
                    setup,
                    node: node.clone(),
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
            outcome,
            description,
            elapsed: start.elapsed(),
        });
    }
    Ok(results)
}

#[cfg(test)]
mod identity;
#[cfg(test)]
mod manifest;
#[cfg(test)]
mod tests {
    use super::beyond;

    fn findings(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| (*line).to_owned()).collect()
    }

    #[test]
    fn counts_a_finding_produced_twice_and_expected_once_as_one_extra() {
        let once = findings(&["a finding"]);
        let twice = findings(&["a finding", "a finding"]);
        assert_eq!(beyond(&twice, &once), ["a finding"]);
        assert_eq!(beyond(&once, &twice), Vec::<String>::new());
        assert_eq!(beyond(&twice, &twice), Vec::<String>::new());
    }
}
