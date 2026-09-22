// The SHACL engine, behind the interface the rest of this crate sees: shapes
// compiled from bytes, and a graph already in memory read against them, one row
// per result.
//
// Nothing here opens a file or starts a thread of its own, so the browser
// export stays reachable and another engine takes this one's place by answering
// the same two calls.
use crate::error::{Error, Result};
use crate::rdf::{SH_INFO, SH_SEVERITY, SH_VIOLATION, SH_WARNING};
use oxrdf::Quad;
use oxrdfio::{RdfFormat, RdfParser};
use rudof_rdf::rdf_core::term::Object;
use rudof_rdf::rdf_core::{BuildRDF, RDFFormat, SHACLPath};
use rudof_rdf::rdf_impl::{OxigraphInMemory, ReaderMode};
use shacl::ir::IRSchema;
use shacl::rdf::ShaclParser;
use shacl::types::Severity;
use shacl::validator::processor::{GraphValidation, ShaclProcessor};
use shacl::validator::{ShaclConfig, ShaclValidationMode};
use std::io::Cursor;

/// A file of the vocabulary: the IRI it was read from, which is the base its
/// own relative terms are read against, and its bytes.
pub(crate) type Document = (String, Vec<u8>);

/// One result, in the terms a finding carries it in.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Drawn {
    pub(crate) component: String,
    pub(crate) severity: String,
    pub(crate) path: Option<String>,
    /// The node the result is about, where it is a node a reader can look up.
    /// A blank node's label belongs to this run and names nothing outside it.
    pub(crate) focus: Option<String>,
}

pub(crate) struct Shapes {
    schema: IRSchema,
}

impl Shapes {
    /// The shapes every document draws, as one shapes graph.
    pub(crate) fn of(documents: &[Document]) -> Result<Self> {
        let mut graph = OxigraphInMemory::new();
        for (iri, bytes) in documents {
            declares_a_reported_severity(iri, bytes)?;
            graph
                .merge_from_reader(
                    &mut Cursor::new(bytes),
                    iri,
                    &RDFFormat::Turtle,
                    Some(iri),
                    &ReaderMode::default(),
                )
                .map_err(|e| Error::msg(format!("{iri}: {e}")))?;
        }
        let parsed = ShaclParser::new(graph)
            .parse()
            .map_err(|e| Error::msg(format!("the vocabulary's shapes: {e}")))?;
        Ok(Self {
            schema: IRSchema::compile(&parsed)
                .map_err(|e| Error::msg(format!("the vocabulary's shapes: {e}")))?,
        })
    }

    /// Every result the graph draws, in an order of its own rather than the one
    /// the engine's workers happened to finish in.
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

/// The severities a finding carries, which are the specification's finding
/// shape's and no more.
const REPORTED: [&str; 3] = [SH_INFO, SH_WARNING, SH_VIOLATION];

/// A shapes graph may give a shape a severity of its own; a finding has no room
/// for one. The whole vocabulary is in hand here, so the run ends on the
/// vocabulary rather than on the first record to fail that shape.
fn declares_a_reported_severity(iri: &str, bytes: &[u8]) -> Result<()> {
    for quad in RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(iri)?
        .for_slice(bytes)
    {
        let quad = quad.map_err(|e| Error::msg(format!("{iri}: {e}")))?;
        if quad.predicate.as_str() != SH_SEVERITY {
            continue;
        }
        let reported = matches!(&quad.object, oxrdf::Term::NamedNode(named)
            if REPORTED.contains(&named.as_str()));
        if !reported {
            return Err(Error::msg(format!(
                "{iri}: a shape declares sh:severity {}; a finding carries sh:Info, sh:Warning or \
                 sh:Violation",
                quad.object
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

/// A path written as one IRI. A sequence, an alternative or a quantified path
/// is not one, and `sh:resultPath` on a finding is.
fn predicate(path: &SHACLPath) -> Option<String> {
    match path {
        SHACLPath::Predicate { pred } => Some(pred.as_str().to_owned()),
        _ => None,
    }
}
