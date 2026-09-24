// What a produced graph is read against: the ontology and shapes files the
// crate's bridge:vocabularyFile names, resolved against the checkout the engine
// command was given and read through the host from there and nowhere else.
use crate::annotation::{self, Record};
use crate::error::{Error, Result};
use crate::load::Adapter;
use crate::rdf::{
    BRIDGE_PREDICATE_NOT_DECLARED, OWL_ANNOTATION_PROPERTY, OWL_DATATYPE_PROPERTY,
    OWL_OBJECT_PROPERTY, RDF_PROPERTY, RDF_TYPE, SH_VIOLATION,
};
use crate::resolver::Resolver;
use crate::shapes::{Document, Shapes};
use oxiri::Iri;
use oxrdf::{NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::{BTreeSet, HashSet};

/// The classes a declaration types a predicate with.
const DECLARES_A_PREDICATE: [&str; 4] = [
    RDF_PROPERTY,
    OWL_DATATYPE_PROPERTY,
    OWL_OBJECT_PROPERTY,
    OWL_ANNOTATION_PROPERTY,
];

pub(crate) struct Vocabulary {
    shapes: Shapes,
    declared: HashSet<String>,
    /// The predicates a record's graph writes that no ontology is asked about.
    exempt: HashSet<String>,
}

impl Vocabulary {
    /// The vocabulary the files name, where the command named a checkout to
    /// read them from and the crate named any.
    pub(crate) fn read(
        files: &[String],
        stamps: HashSet<String>,
        resolver: &dyn Resolver,
    ) -> Result<Option<Self>> {
        let Some(directory) = resolver.vocabularies() else {
            return Ok(None);
        };
        if files.is_empty() {
            return Ok(None);
        }
        let checkout = Iri::parse(directory)?;
        let mut documents: Vec<Document> = Vec::new();
        for file in files {
            let iri = checkout
                .resolve(file)
                .map_err(|e| Error::msg(format!("the crate names {file}: {e}")))?
                .into_inner();
            let bytes = resolver.read_vocabulary(&iri)?;
            documents.push((iri, bytes));
        }
        let mut exempt = stamps;
        exempt.insert(RDF_TYPE.to_owned());
        Ok(Some(Self {
            declared: declared(&documents)?,
            shapes: Shapes::of(&documents)?,
            exempt,
        }))
    }

    /// Every finding the vocabulary draws on one record's produced graph.
    pub(crate) fn findings(&self, record: &Record<'_>, quads: &[Quad]) -> Result<Vec<Quad>> {
        let mut findings = Vec::new();
        for result in self.shapes.results(quads)? {
            findings.extend(annotation::drawn(
                record,
                &result.component,
                result.path.as_deref(),
                result.focus.as_deref(),
                &result.severity,
            )?);
        }
        for predicate in self.undeclared(quads) {
            findings.extend(annotation::drawn(
                record,
                BRIDGE_PREDICATE_NOT_DECLARED,
                Some(predicate),
                None,
                SH_VIOLATION,
            )?);
        }
        Ok(findings)
    }

    /// The predicates the graph writes that no file of the vocabulary declares,
    /// each once however many triples wrote it.
    fn undeclared<'a>(&self, quads: &'a [Quad]) -> BTreeSet<&'a str> {
        quads
            .iter()
            .map(|quad| quad.predicate.as_str())
            .filter(|predicate| !self.exempt.contains(*predicate))
            .filter(|predicate| !self.declared.contains(*predicate))
            .collect()
    }
}

fn declared(documents: &[Document]) -> Result<HashSet<String>> {
    let mut declared = HashSet::new();
    for (iri, bytes) in documents {
        for quad in RdfParser::from_format(RdfFormat::Turtle)
            .with_base_iri(iri)?
            .for_slice(bytes)
        {
            let quad = quad.map_err(|e| Error::msg(format!("{iri}: {e}")))?;
            if quad.predicate.as_str() != RDF_TYPE {
                continue;
            }
            let declares = matches!(&quad.object, Term::NamedNode(class)
                if DECLARES_A_PREDICATE.contains(&class.as_str()));
            if let (true, NamedOrBlankNode::NamedNode(named)) = (declares, &quad.subject) {
                declared.insert(named.as_str().to_owned());
            }
        }
    }
    Ok(declared)
}

pub fn require_vocabularies(adapter: &Adapter, resolver: &dyn Resolver) -> Result<()> {
    if adapter.vocabulary_files.is_empty() || resolver.vocabularies().is_some() {
        return Ok(());
    }
    Err(Error::msg(format!(
        "the crate names {} bridge:vocabularyFile, and the command was given no \
         --vocabularies directory to read them from",
        adapter.vocabulary_files.len()
    )))
}
