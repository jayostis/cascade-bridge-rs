// The report: one earl:Assertion per manifest entry, the form every W3C test
// suite's implementation reports take.
use crate::error::Result;
use crate::harness::EntryResult;
use oxrdf::vocab::{rdf, xsd};
use oxrdf::{BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfSerializer};
use oxsdatatypes::DateTime;

const EARL: &str = "http://www.w3.org/ns/earl#";
const DCT: &str = "http://purl.org/dc/terms/";
const DOAP: &str = "http://usefulinc.com/ns/doap#";

pub struct ReportSubject {
    pub iri: String,
    pub name: String,
    pub version: String,
}

fn iri(namespace: &str, local: &str) -> Result<NamedNode> {
    Ok(NamedNode::new(format!("{namespace}{local}"))?)
}

pub fn earl_report(results: &[EntryResult], subject: &ReportSubject) -> Result<String> {
    earl_report_at(results, subject, &DateTime::now().to_string())
}

pub fn earl_report_at(
    results: &[EntryResult],
    subject: &ReportSubject,
    when: &str,
) -> Result<String> {
    let software = NamedNode::new(&subject.iri)?;
    let when = Literal::new_typed_literal(when, xsd::DATE_TIME);
    let mut quads: Vec<Quad> = Vec::new();
    let mut triple = |s: NamedOrBlankNode, p: NamedNode, o: oxrdf::Term| {
        quads.push(Quad::new(s, p, o, GraphName::DefaultGraph));
    };

    let s = NamedOrBlankNode::from(software.clone());
    triple(
        s.clone(),
        rdf::TYPE.into_owned(),
        iri(EARL, "Software")?.into(),
    );
    triple(
        s.clone(),
        rdf::TYPE.into_owned(),
        iri(DOAP, "Project")?.into(),
    );
    triple(
        s.clone(),
        iri(DOAP, "name")?,
        Literal::new_simple_literal(&subject.name).into(),
    );
    let release = BlankNode::default();
    triple(s.clone(), iri(DOAP, "release")?, release.clone().into());
    triple(
        release.into(),
        iri(DOAP, "revision")?,
        Literal::new_simple_literal(&subject.version).into(),
    );

    for result in results {
        // An entry with no IRI has no name outside its manifest, so it is
        // reported as an anonymous test carrying the entry's name: the report
        // still holds one assertion per entry.
        let test = match &result.entry {
            Term::NamedNode(test) => NamedOrBlankNode::from(test.clone()),
            _ => {
                let test = NamedOrBlankNode::from(BlankNode::default());
                triple(
                    test.clone(),
                    iri(DCT, "title")?,
                    Literal::new_simple_literal(&result.name).into(),
                );
                test
            }
        };
        let assertion = BlankNode::default();
        let test_result = BlankNode::default();
        let a = NamedOrBlankNode::from(assertion);
        triple(
            a.clone(),
            rdf::TYPE.into_owned(),
            iri(EARL, "Assertion")?.into(),
        );
        triple(a.clone(), iri(EARL, "assertedBy")?, software.clone().into());
        triple(a.clone(), iri(EARL, "subject")?, software.clone().into());
        triple(a.clone(), iri(EARL, "test")?, test.into());
        triple(
            a.clone(),
            iri(EARL, "mode")?,
            iri(EARL, "automatic")?.into(),
        );
        triple(a, iri(EARL, "result")?, test_result.clone().into());

        let r = NamedOrBlankNode::from(test_result);
        triple(
            r.clone(),
            rdf::TYPE.into_owned(),
            iri(EARL, "TestResult")?.into(),
        );
        triple(
            r.clone(),
            iri(EARL, "outcome")?,
            iri(EARL, result.outcome.as_str())?.into(),
        );
        triple(
            r.clone(),
            iri(DCT, "description")?,
            Literal::new_simple_literal(&result.description).into(),
        );
        triple(r, iri(DCT, "date")?, when.clone().into());
    }

    let mut serializer = RdfSerializer::from_format(RdfFormat::Turtle)
        .with_prefix("earl", EARL)?
        .with_prefix("dct", DCT)?
        .with_prefix("doap", DOAP)?
        .with_prefix("xsd", "http://www.w3.org/2001/XMLSchema#")?
        .for_writer(Vec::new());
    for quad in &quads {
        serializer.serialize_quad(quad)?;
    }
    Ok(String::from_utf8(serializer.finish()?)?)
}
