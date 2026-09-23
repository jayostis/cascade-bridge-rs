// What a produced graph is read against, and what a result it failed becomes.
// The shapes and the ontology are the Cascade vocabulary's, read from the
// checkout the engine command was given and from nowhere else; each SHACL
// result is one finding about the record whose graph it was, carrying the
// result itself rather than a sentence about it; and a predicate no ontology
// declares is a finding of the same shape. Validation reports and never
// refuses: the graph is produced whatever the shapes say of it.
use cascade_bridge::{
    convert, load_adapter, prepare, Conversion, DirectoryResolver, Resolver, Source,
};
use oxrdf::{Quad, Term};
use std::collections::BTreeSet;
use std::path::PathBuf;

const OA: &str = "http://www.w3.org/ns/oa#";
const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";

/// The namespace the tiny adapter maps into, which the checkout's ontology
/// declares and the checkout's shapes constrain.
const EX: &str = "urn:example:catalog#";

/// The gap kind an undeclared predicate's finding names as its body.
const PREDICATE_NOT_DECLARED: &str =
    "https://ns.cascadeprotocol.org/bridge/v1-draft#predicateNotDeclared";

/// The shapes file the crate names by its path in the checkout, which is a
/// path no file of the adapter answers to.
const SHAPES: &str = "ontologies/catalog/v1/catalog.shapes.ttl";

fn adapter() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter")
}

/// Where a checkout of `the-cascade-protocol/spec` would stand.
fn checkout() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-vocabularies")
}

fn read_against_the_vocabulary() -> DirectoryResolver {
    DirectoryResolver::new(adapter())
        .expect("resolver")
        .with_vocabularies(checkout())
        .expect("the vocabularies directory")
}

/// The conversion and the IRI of the document it was made from.
fn run(resolver: &dyn Resolver, input: &str) -> (Conversion, String) {
    let adapter = load_adapter(resolver).expect("adapter");
    let prepared = prepare(&adapter, resolver).expect("prepared");
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    let xml = resolver.read(&iri).expect("input");
    let conversion = convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            xml: &xml,
        },
    )
    .expect("conversion");
    (conversion, iri)
}

fn converted(input: &str) -> (Conversion, String) {
    run(&read_against_the_vocabulary(), input)
}

/// A term as an address or a body is read: an IRI or a literal by what it
/// says, anything else by how N-Triples writes it.
fn written(term: Term) -> String {
    match term {
        Term::NamedNode(node) => node.as_str().to_owned(),
        Term::Literal(literal) => literal.value().to_owned(),
        other => other.to_string(),
    }
}

/// The one object this subject carries for this predicate, where it carries
/// exactly one.
fn one(quads: &[Quad], subject: &str, predicate: &str) -> Option<Term> {
    let mut objects = quads
        .iter()
        .filter(|q| q.subject.to_string() == subject && q.predicate.as_str() == predicate)
        .map(|q| q.object.clone());
    let first = objects.next()?;
    match objects.next() {
        None => Some(first),
        Some(_) => None,
    }
}

fn node(quads: &[Quad], subject: &str, predicate: &str) -> String {
    one(quads, subject, predicate)
        .map(|term| term.to_string())
        .unwrap_or_default()
}

fn says(quads: &[Quad], subject: &str, predicate: &str) -> String {
    one(quads, subject, predicate)
        .map(written)
        .unwrap_or_default()
}

/// Everything the specification says a finding about the produced graph
/// carries. What a finding does not carry reads as the empty string, so a
/// focus node left unnamed is a case this can state.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Finding {
    body: String,
    path: String,
    severity: String,
    focus: String,
    motivation: String,
    source: String,
    record: String,
    refined: String,
}

/// Every annotation of a findings graph, by the node it is.
fn annotations(findings: &[Quad]) -> Vec<String> {
    let annotation = format!("{OA}Annotation");
    findings
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(|q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == annotation))
        .map(|q| q.subject.to_string())
        .collect()
}

/// The annotations the output validation made. A finding a schema or an
/// adapter's own query made names neither a SHACL constraint component nor a
/// gap kind of the produced graph, and is another stage's business.
fn reported(findings: &[Quad]) -> Vec<String> {
    annotations(findings)
        .into_iter()
        .filter(|annotation| {
            let body = says(findings, annotation, &format!("{OA}hasBody"));
            body.starts_with(SH) || body == PREDICATE_NOT_DECLARED
        })
        .collect()
}

fn selector_of(findings: &[Quad], annotation: &str) -> String {
    let target = node(findings, annotation, &format!("{OA}hasTarget"));
    node(findings, &target, &format!("{OA}hasSelector"))
}

fn output_findings(findings: &[Quad]) -> Vec<Finding> {
    let mut rows: Vec<Finding> = reported(findings)
        .into_iter()
        .map(|annotation| {
            let target = node(findings, &annotation, &format!("{OA}hasTarget"));
            let selector = node(findings, &target, &format!("{OA}hasSelector"));
            let refinement = node(findings, &selector, &format!("{OA}refinedBy"));
            Finding {
                body: says(findings, &annotation, &format!("{OA}hasBody")),
                path: says(findings, &annotation, &format!("{SH}resultPath")),
                severity: says(findings, &annotation, &format!("{SH}resultSeverity")),
                focus: says(findings, &annotation, &format!("{SH}focusNode")),
                motivation: says(findings, &annotation, &format!("{OA}motivatedBy")),
                source: says(findings, &target, &format!("{OA}hasSource")),
                record: says(findings, &selector, RDF_VALUE),
                refined: says(findings, &refinement, RDF_VALUE),
            }
        })
        .collect();
    rows.sort();
    rows
}

#[test]
fn makes_one_finding_of_the_result_a_record_s_graph_failed_a_shape_on() {
    let (conversion, document) = converted("output-fails-a-shape.xml");
    assert_eq!(
        output_findings(&conversion.findings),
        [Finding {
            body: format!("{SH}MaxLengthConstraintComponent"),
            path: format!("{EX}code"),
            severity: format!("{SH}Warning"),
            focus: "urn:example:item:1".to_owned(),
            motivation: format!("{OA}classifying"),
            source: document,
            record: "/catalog/item[1]".to_owned(),
            refined: String::new(),
        }]
    );
    assert!(
        conversion
            .quads
            .iter()
            .any(|q| q.predicate.as_str() == format!("{EX}code")),
        "the graph the shape refused is produced all the same"
    );
}

#[test]
fn names_a_focus_node_that_is_an_iri_and_leaves_a_blank_one_unnamed() {
    let (conversion, _) = converted("output-fails-two-shapes.xml");
    let focus: Vec<(String, String)> = output_findings(&conversion.findings)
        .into_iter()
        .map(|finding| (finding.body, finding.focus))
        .collect();
    assert_eq!(
        focus,
        [
            (
                format!("{SH}MaxLengthConstraintComponent"),
                "urn:example:item:1".to_owned()
            ),
            (format!("{SH}MinCountConstraintComponent"), String::new()),
        ]
    );
}

#[test]
fn makes_a_finding_of_its_own_of_each_of_two_results_about_one_record() {
    let (conversion, _) = converted("output-fails-two-shapes.xml");
    let found = reported(&conversion.findings);
    assert_eq!(
        found.len(),
        2,
        "{:?}",
        output_findings(&conversion.findings)
    );
    let selectors: BTreeSet<String> = found
        .iter()
        .map(|annotation| selector_of(&conversion.findings, annotation))
        .collect();
    assert_eq!(
        selectors.len(),
        2,
        "two findings on one record are two alternatives of each other where they share a selector"
    );
    let severities: Vec<String> = output_findings(&conversion.findings)
        .into_iter()
        .map(|finding| finding.severity)
        .collect();
    assert_eq!(
        severities,
        [format!("{SH}Warning"), format!("{SH}Violation")],
        "each finding carries the severity of the result it was made from"
    );
}

#[test]
fn reports_a_predicate_no_ontology_declares_and_leaves_rdf_type_and_the_stamp_alone() {
    let (conversion, _) = converted("a-predicate-no-ontology-declares.xml");
    let reported: Vec<(String, String, String)> = output_findings(&conversion.findings)
        .into_iter()
        .map(|finding| (finding.body, finding.path, finding.severity))
        .collect();
    assert_eq!(
        reported,
        [(
            PREDICATE_NOT_DECLARED.to_owned(),
            format!("{EX}colour"),
            format!("{SH}Violation"),
        )],
        "the record's graph writes rdf:type and the manifest's bridge:stampPredicate too"
    );
}

#[test]
fn reads_the_vocabulary_from_the_directory_the_command_was_given() {
    let (without, _) = run(
        &DirectoryResolver::new(adapter()).expect("resolver"),
        "output-fails-a-shape.xml",
    );
    assert_eq!(
        output_findings(&without.findings),
        [],
        "the command named no checkout, so there is nothing to read the graph against"
    );

    let (with, _) = converted("output-fails-a-shape.xml");
    assert_eq!(
        output_findings(&with.findings).len(),
        1,
        "the shapes stand in the checkout the command named and nowhere in the adapter"
    );
}

/// The tiny adapter naming a vocabulary file somewhere other than where its
/// shapes stand in the checkout.
struct Names {
    directory: DirectoryResolver,
    file: &'static str,
}

impl Resolver for Names {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn vocabularies(&self) -> Option<&str> {
        self.directory.vocabularies()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("ro-crate-metadata.json") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(text.contains(SHAPES), "the crate names its shapes file");
        Ok(text.replace(SHAPES, self.file).into_bytes())
    }

    fn read_vocabulary(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        self.directory.read_vocabulary(iri)
    }
}

/// What preparing the tiny adapter says of a crate naming this vocabulary
/// file, which is nothing where it prepares.
fn refusal(file: &'static str) -> String {
    let resolver = Names {
        directory: read_against_the_vocabulary(),
        file,
    };
    let adapter = load_adapter(&resolver).expect("adapter");
    prepare(&adapter, &resolver)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default()
}

#[test]
fn refuses_a_vocabulary_file_outside_the_checkout_and_outside_the_adapter() {
    let refused = refusal("../boundary.rs");
    assert!(
        refused.contains("not inside"),
        "a vocabulary file that climbs out of the checkout and the adapter was read: {refused:?}"
    );
}

#[test]
fn refuses_a_vocabulary_file_that_is_a_file_of_the_adapter() {
    let refused = refusal("../tiny-adapter/vocab/catalog-gaps.ttl");
    assert!(
        refused.contains("not inside"),
        "the adapter was read against a file of its own rather than one at the vocabulary pin: \
         {refused:?}"
    );
}

/// The checkout's shapes with a severity of the vocabulary's own invention,
/// which SHACL allows and the specification's finding shape has no room for.
struct Critical {
    directory: DirectoryResolver,
}

impl Critical {
    const DECLARED: &'static str = "sh:severity sh:Warning";

    fn invented(bytes: Vec<u8>) -> Vec<u8> {
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(
            text.contains(Self::DECLARED),
            "the shapes declare a severity of their own"
        );
        text.replace(Self::DECLARED, "sh:severity ex:Critical")
            .into_bytes()
    }
}

impl Resolver for Critical {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn vocabularies(&self) -> Option<&str> {
        self.directory.vocabularies()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        self.directory.read(iri)
    }

    fn read_vocabulary(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read_vocabulary(iri)?;
        match iri.ends_with("catalog.shapes.ttl") {
            true => Ok(Self::invented(bytes)),
            false => Ok(bytes),
        }
    }
}

#[test]
fn refuses_a_shape_whose_severity_no_finding_can_carry() {
    let resolver = Critical {
        directory: read_against_the_vocabulary(),
    };
    let adapter = load_adapter(&resolver).expect("adapter");
    let refused = prepare(&adapter, &resolver)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(
        refused.contains("sh:Info, sh:Warning or sh:Violation"),
        "a severity the specification's finding shape has no room for was compiled: {refused:?}"
    );
}

/// The manifest with a stamp predicate on one entry, which the specification
/// gives that entry alone and no conversion.
struct EntryStamp {
    directory: DirectoryResolver,
}

impl Resolver for EntryStamp {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn vocabularies(&self) -> Option<&str> {
        self.directory.vocabularies()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        let bytes = self.directory.read(iri)?;
        if !iri.ends_with("fixtures/manifest.ttl") {
            return Ok(bytes);
        }
        let text = String::from_utf8(bytes).expect("utf-8");
        let entry = "<#pass> a bridge:IsomorphicConversionTest ;";
        assert!(text.contains(entry), "the manifest lists the entry");
        Ok(text
            .replace(
                entry,
                &format!("{entry}\n  bridge:stampPredicate <{EX}colour> ;"),
            )
            .into_bytes())
    }

    fn read_vocabulary(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        self.directory.read_vocabulary(iri)
    }
}

#[test]
fn reports_a_predicate_one_entry_of_the_manifest_stamps_with() {
    let resolver = EntryStamp {
        directory: read_against_the_vocabulary(),
    };
    let (conversion, _) = run(&resolver, "a-predicate-no-ontology-declares.xml");
    let paths: Vec<String> = output_findings(&conversion.findings)
        .into_iter()
        .map(|finding| finding.path)
        .collect();
    assert_eq!(
        paths,
        [format!("{EX}colour")],
        "one entry's stamp is that entry's, not every conversion's"
    );
}
