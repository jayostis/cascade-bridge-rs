// Nothing here opens a file or starts a thread, so the browser export stays reachable.
use crate::error::{Error, Result};
use crate::terms::{SH_INFO, SH_SEVERITY, SH_VIOLATION, SH_WARNING};
use oxrdf::{Graph, NamedNodeRef, Quad, TermRef};
use rudof_rdf::rdf_core::term::Object;
use rudof_rdf::rdf_core::{BuildRDF, SHACLPath};
use rudof_rdf::rdf_impl::OxigraphInMemory;
use shacl::ir::IRSchema;
use shacl::rdf::ShaclParser;
use shacl::types::Severity;
use shacl::validator::processor::{GraphValidation, ShaclProcessor};
use shacl::validator::{ShaclConfig, ShaclValidationMode};

pub(crate) type Document = (String, Graph);

/// One result, in the terms a finding carries it in.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Drawn {
    pub(crate) component: String,
    pub(crate) severity: String,
    pub(crate) path: Option<String>,
    /// None for a blank node, whose label names nothing outside this run.
    pub(crate) focus: Option<String>,
}

pub(crate) struct Shapes {
    schema: IRSchema,
}

impl Shapes {
    pub(crate) fn of(documents: &[Document]) -> Result<Self> {
        let mut graph = OxigraphInMemory::new();
        for (iri, document) in documents {
            declares_a_reported_severity(iri, document)?;
            for triple in document {
                graph
                    .add_triple(
                        triple.subject.into_owned(),
                        triple.predicate.into_owned(),
                        triple.object.into_owned(),
                    )
                    .map_err(|e| Error::msg(format!("{iri}: {e}")))?;
            }
        }
        let parsed = ShaclParser::new(graph)
            .parse()
            .map_err(|e| Error::msg(format!("the vocabulary's shapes: {e}")))?;
        Ok(Self {
            schema: IRSchema::compile(&parsed)
                .map_err(|e| Error::msg(format!("the vocabulary's shapes: {e}")))?,
        })
    }

    /// Sorted, not in the order the engine's workers finished in.
    pub(crate) fn results(&self, quads: &[Quad]) -> Result<Vec<Drawn>> {
        let mut data = OxigraphInMemory::new();
        for quad in quads {
            data.add_triple(
                quad.subject.clone(),
                quad.predicate.clone(),
                quad.object.clone(),
            )
            .map_err(|e| Error::msg(format!("the produced graph: {e}")))?;
        }
        let report = GraphValidation::new(data.into())
            .validate(
                &self.schema,
                &ShaclValidationMode::Native,
                &ShaclConfig::new(),
            )
            .map_err(|e| Error::msg(format!("the produced graph: {e}")))?;
        let mut drawn = Vec::new();
        for result in report.results() {
            drawn.push(Drawn {
                component: term(result.constraint_component()),
                severity: severity(result.severity())?,
                path: result.path().and_then(predicate),
                focus: match result.focus_node() {
                    Object::Iri(iri) => Some(iri.as_str().to_owned()),
                    _ => None,
                },
            });
        }
        drawn.sort();
        Ok(drawn)
    }
}

const REPORTED: [&str; 3] = [SH_INFO, SH_WARNING, SH_VIOLATION];

fn declares_a_reported_severity(iri: &str, document: &Graph) -> Result<()> {
    let severity = NamedNodeRef::new_unchecked(SH_SEVERITY);
    for declared in document
        .triples_for_predicate(severity)
        .map(|triple| triple.object)
    {
        let reported = matches!(declared, TermRef::NamedNode(named)
            if REPORTED.contains(&named.as_str()));
        if !reported {
            return Err(Error::msg(format!(
                "{iri}: a shape declares sh:severity {declared}; a finding carries sh:Info, \
                 sh:Warning or sh:Violation"
            )));
        }
    }
    Ok(())
}

fn term(object: &Object) -> String {
    match object {
        Object::Iri(iri) => iri.as_str().to_owned(),
        other => other.to_string(),
    }
}

fn severity(severity: &Severity) -> Result<String> {
    match severity {
        Severity::Info => Ok(SH_INFO.to_owned()),
        Severity::Warning => Ok(SH_WARNING.to_owned()),
        Severity::Violation => Ok(SH_VIOLATION.to_owned()),
        _ => Err(Error::msg(
            "a result carries a severity no finding does: sh:Info, sh:Warning or sh:Violation",
        )),
    }
}

fn predicate(path: &SHACLPath) -> Option<String> {
    match path {
        SHACLPath::Predicate { pred } => Some(pred.as_str().to_owned()),
        _ => None,
    }
}
