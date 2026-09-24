use crate::annotation::{self, Record};
use crate::error::{Error, Result};
use crate::load::{turtle, Adapter};
use crate::resolver::Resolver;
use crate::shapes::{Document, Shapes};
use crate::terms::{
    BRIDGE_PREDICATE_NOT_DECLARED, OWL_ANNOTATION_PROPERTY, OWL_DATATYPE_PROPERTY,
    OWL_OBJECT_PROPERTY, RDF_PROPERTY, RDF_TYPE, SH_VIOLATION,
};
use oxiri::Iri;
use oxrdf::vocab::rdf;
use oxrdf::{NamedOrBlankNodeRef, Quad, TermRef};
use std::collections::{BTreeSet, HashSet};

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
            let (graph, _) = turtle(&resolver.read_vocabulary(&iri)?, &iri)?;
            documents.push((iri, graph));
        }
        let mut exempt = stamps;
        exempt.insert(RDF_TYPE.to_owned());
        Ok(Some(Self {
            declared: declared(&documents),
            shapes: Shapes::of(&documents)?,
            exempt,
        }))
    }

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

    fn undeclared<'a>(&self, quads: &'a [Quad]) -> BTreeSet<&'a str> {
        quads
            .iter()
            .map(|quad| quad.predicate.as_str())
            .filter(|predicate| !self.exempt.contains(*predicate))
            .filter(|predicate| !self.declared.contains(*predicate))
            .collect()
    }
}

fn declared(documents: &[Document]) -> HashSet<String> {
    let mut declared = HashSet::new();
    for (_, graph) in documents {
        for triple in graph.triples_for_predicate(rdf::TYPE) {
            let declares = matches!(triple.object, TermRef::NamedNode(class)
                if DECLARES_A_PREDICATE.contains(&class.as_str()));
            if let (true, NamedOrBlankNodeRef::NamedNode(named)) = (declares, triple.subject) {
                declared.insert(named.as_str().to_owned());
            }
        }
    }
    declared
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

pub fn unvalidated_output(adapter: &Adapter, resolver: &dyn Resolver) -> Option<String> {
    require_vocabularies(adapter, resolver)
        .err()
        .map(|refusal| format!("{refusal}, so the graph is not validated against them"))
}
