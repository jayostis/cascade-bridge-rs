use crate::annotation::{self, Record};
use crate::decode::XML_SPACE;
use crate::error::{Error, Result};
use crate::lift::{Paths, Unit, Valued};
use crate::load::{instances, objects, subjects, turtle, Prefixes};
use crate::resolver::Resolver;
use crate::terms::{
    BRIDGE_CARRIED_IN_PART, BRIDGE_LOOKUP_IN, BRIDGE_LOOKUP_NAMES_GAP, BRIDGE_NAMES_GAP,
    BRIDGE_NO_HOME, BRIDGE_NO_PREDICATE, BRIDGE_PATH_ENTRY, BRIDGE_SOURCE_LACKS_REQUIRED,
    BRIDGE_SOURCE_PATH, BRIDGE_VALUE_NOT_MAPPED, BRIDGE_VERDICT, SH_INFO, SH_RESULT_SEVERITY,
    SH_VIOLATION, SH_WARNING, SKOS_BROADER, SKOS_CONCEPT_SCHEME, SKOS_NOTATION,
};
use oxigraph::model::{NamedOrBlankNode, Quad, Term};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

struct Entry {
    path: String,
    verdict: Option<String>,
    gap: Option<String>,
    /// The concept map a value is looked up in; `miss` is the gap for a value it lacks.
    map: Option<String>,
    miss: Option<String>,
}

#[derive(Default)]
pub(crate) struct Gap {
    kind: Option<String>,
    pub(crate) severity: Option<String>,
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

pub(crate) struct Accounting {
    paths: HashSet<String>,
    reported: HashMap<String, Vec<Reported>>,
    lookups: HashMap<String, Vec<Lookup>>,
}

impl Accounting {
    pub(crate) fn read(
        resolver: &dyn Resolver,
        iri: &str,
        scheme: &HashMap<String, Gap>,
    ) -> Result<Self> {
        Self::of(entries(resolver, iri)?, scheme, iri, resolver)
    }

    pub(crate) fn paths(&self) -> Paths {
        Paths::Kept {
            valued: self.lookups.keys().cloned().collect(),
        }
    }

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

    pub(crate) fn findings(&self, record: &Record<'_>, unit: &Unit) -> Result<Vec<Quad>> {
        let mut findings = Vec::new();
        for occurrence in unit.occurrences() {
            if !self.paths.contains(&occurrence.path) {
                findings.extend(annotation::unaccounted(
                    record,
                    &occurrence.path,
                    occurrence.within.as_deref(),
                    occurrence.count,
                )?);
            }
            for reported in self.reported.get(&occurrence.path).into_iter().flatten() {
                findings.extend(annotation::gap(
                    record,
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
            for lookup in self
                .lookups
                .get(&valued.path)
                .into_iter()
                .flatten()
                .filter(|lookup| !lookup.notations.contains(&key))
            {
                findings.extend(annotation::lookup(
                    record,
                    &lookup.gap,
                    &valued.value,
                    valued.within.as_deref(),
                    &lookup.severity,
                    valued.count,
                )?);
            }
        }
        Ok(findings)
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
    let (graph, _) = turtle(&resolver.read(iri)?, iri)?;
    let typed = instances(&graph, BRIDGE_PATH_ENTRY)?;
    let mut entries = Vec::new();
    for s in subjects(&graph, BRIDGE_SOURCE_PATH)? {
        for path in objects(&graph, &s, BRIDGE_SOURCE_PATH)? {
            let Term::Literal(path) = path else {
                return Err(Error::msg(format!("{iri}: {path} is no path")));
            };
            if !typed.contains(&s) {
                continue;
            }
            let path = path.value();
            let said = |predicate: &str, name: &str| {
                declared(&objects(&graph, &s, predicate)?, iri, path, name)
            };
            entries.push(Entry {
                verdict: said(BRIDGE_VERDICT, "verdict")?,
                gap: said(BRIDGE_NAMES_GAP, "gap")?,
                map: said(BRIDGE_LOOKUP_IN, "bridge:lookupIn")?,
                miss: said(BRIDGE_LOOKUP_NAMES_GAP, "bridge:lookupNamesGap")?,
                path: path.to_owned(),
            });
        }
    }
    Ok(entries)
}

pub(crate) fn gap_scheme(
    resolver: &dyn Resolver,
    iri: &str,
) -> Result<(HashMap<String, Gap>, Prefixes)> {
    let (graph, prefixes) = turtle(&resolver.read(iri)?, iri)?;
    let mut scheme: HashMap<String, Gap> = HashMap::new();
    for predicate in [SKOS_BROADER, SH_RESULT_SEVERITY] {
        for s in subjects(&graph, predicate)? {
            let NamedOrBlankNode::NamedNode(concept) = &s else {
                continue;
            };
            let mut declared = Vec::new();
            for object in objects(&graph, &s, predicate)? {
                let Term::NamedNode(object) = object else {
                    return Err(Error::msg(format!(
                        "{iri}: {concept} declares {predicate} {object}, which is no IRI"
                    )));
                };
                declared.push(object);
            }
            if predicate == SH_RESULT_SEVERITY {
                if let Some(object) = declared
                    .iter()
                    .find(|object| !SEVERITIES.contains(&object.as_str()))
                {
                    return Err(Error::msg(format!(
                        "{iri}: {concept} declares sh:resultSeverity {object}; a gap's severity \
                         is sh:Info, sh:Warning or sh:Violation"
                    )));
                }
            }
            let name = if predicate == SKOS_BROADER {
                "skos:broader"
            } else {
                "sh:resultSeverity"
            };
            if let [first, second, ..] = declared.as_slice() {
                return Err(Error::msg(format!(
                    "{iri}: {concept} declares {name} {first} and {second}; a gap declares at \
                     most one"
                )));
            }
            let Some(one) = declared.first() else {
                continue;
            };
            let gap = scheme.entry(concept.as_str().to_owned()).or_default();
            if predicate == SKOS_BROADER {
                gap.kind = Some(one.as_str().to_owned());
            } else {
                gap.severity = Some(one.as_str().to_owned());
            }
        }
    }
    Ok((scheme, prefixes))
}

fn concept_map(resolver: &dyn Resolver, iri: &str) -> Result<HashSet<String>> {
    let (graph, _) = turtle(&resolver.read(iri)?, iri)?;
    let mut notations = HashSet::new();
    for s in subjects(&graph, SKOS_NOTATION)? {
        for notation in objects(&graph, &s, SKOS_NOTATION)? {
            let Term::Literal(notation) = notation else {
                return Err(Error::msg(format!("{iri}: {notation} is no skos:notation")));
            };
            notations.insert(notation.value().to_owned());
        }
    }
    let schemes = instances(&graph, SKOS_CONCEPT_SCHEME)?.len();
    if schemes != 1 {
        return Err(Error::msg(format!(
            "{iri}: the file holds {schemes} concept schemes; a bridge:lookupIn names a file \
             holding one"
        )));
    }
    Ok(notations)
}
