// What a produced graph is read against, and what a result it failed becomes.
// The shapes and the ontology are the Cascade vocabulary's, read from the
// checkout the engine command was given and from nowhere else; each SHACL
// result is one finding about the record whose graph it was, carrying the
// result itself rather than a sentence about it; and a predicate no ontology
// declares is a finding of the same shape. Validation reports and never
// refuses: the graph is produced whatever the shapes say of it.
mod common;

use cascade_bridge::{load_adapter, prepare, Conversion, Resolver};
use common::{
    address, annotations, converted, node, says, tiny, tiny_with_vocabularies, Variant, CRATE, OA,
    SH,
};
use oxrdf::Quad;
use std::collections::BTreeSet;

/// The namespace the tiny adapter maps into, which the checkout's ontology
/// declares and the checkout's shapes constrain.
const EX: &str = "urn:example:catalog#";

/// The gap kind an undeclared predicate's finding names as its body.
const PREDICATE_NOT_DECLARED: &str =
    "https://ns.cascadeprotocol.org/bridge/v1-draft#predicateNotDeclared";

/// The shapes file the crate names by its path in the checkout, which is a
/// path no file of the adapter answers to.
const SHAPES: &str = "ontologies/catalog/v1/catalog.shapes.ttl";

/// The conversion and the IRI of the document it was made from.
fn run(resolver: &dyn Resolver, input: &str) -> (Conversion, String) {
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    (converted(resolver, input), iri)
}

fn read_against_the_vocabulary(input: &str) -> (Conversion, String) {
    run(&tiny_with_vocabularies(), input)
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
            let (record, refined) = address(findings, &annotation);
            Finding {
                body: says(findings, &annotation, &format!("{OA}hasBody")),
                path: says(findings, &annotation, &format!("{SH}resultPath")),
                severity: says(findings, &annotation, &format!("{SH}resultSeverity")),
                focus: says(findings, &annotation, &format!("{SH}focusNode")),
                motivation: says(findings, &annotation, &format!("{OA}motivatedBy")),
                source: says(findings, &target, &format!("{OA}hasSource")),
                record,
                refined,
            }
        })
        .collect();
    rows.sort();
    rows
}

/// A finding the checkout's shapes draw on the first record of a document.
fn on_the_first_record(body: &str, severity: &str, focus: &str, document: &str) -> Finding {
    Finding {
        body: format!("{SH}{body}"),
        path: format!("{EX}code"),
        severity: format!("{SH}{severity}"),
        focus: focus.to_owned(),
        motivation: format!("{OA}classifying"),
        source: document.to_owned(),
        record: "/catalog/item[1]".to_owned(),
        refined: String::new(),
    }
}

/// What `output-fails-a-shape.xml` draws: its item's code is longer than the
/// shape allows.
fn code_too_long(document: &str) -> Finding {
    on_the_first_record(
        "MaxLengthConstraintComponent",
        "Warning",
        "urn:example:item:1",
        document,
    )
}

#[test]
fn makes_one_finding_of_the_result_a_record_s_graph_failed_a_shape_on() {
    let (conversion, document) = read_against_the_vocabulary("output-fails-a-shape.xml");
    assert_eq!(
        output_findings(&conversion.findings),
        [code_too_long(&document)]
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
    let (conversion, _) = read_against_the_vocabulary("output-fails-two-shapes.xml");
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
    let (conversion, document) = read_against_the_vocabulary("output-fails-two-shapes.xml");
    assert_eq!(
        output_findings(&conversion.findings),
        [
            code_too_long(&document),
            on_the_first_record("MinCountConstraintComponent", "Violation", "", &document),
        ],
        "each finding carries the severity of the result it was made from"
    );
    let selectors: BTreeSet<String> = reported(&conversion.findings)
        .iter()
        .map(|annotation| selector_of(&conversion.findings, annotation))
        .collect();
    assert_eq!(
        selectors.len(),
        2,
        "two findings on one record are two alternatives of each other where they share a selector"
    );
}

#[test]
fn reports_a_predicate_no_ontology_declares_and_leaves_rdf_type_and_the_stamp_alone() {
    let (conversion, _) = read_against_the_vocabulary("a-predicate-no-ontology-declares.xml");
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
    let (without, _) = run(&tiny(), "output-fails-a-shape.xml");
    assert_eq!(
        output_findings(&without.findings),
        [],
        "a resolver given no checkout reads the graph against nothing"
    );

    let (with, document) = read_against_the_vocabulary("output-fails-a-shape.xml");
    assert_eq!(
        output_findings(&with.findings),
        [code_too_long(&document)],
        "the shapes stand in the checkout the command named and nowhere in the adapter"
    );
}

/// What preparing the tiny adapter says of a crate naming this vocabulary
/// file, which is nothing where it prepares.
fn refusal(file: &str) -> String {
    let resolver = Variant::of(tiny_with_vocabularies()).replacing(CRATE, SHAPES, file);
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

#[test]
fn refuses_a_shape_whose_severity_no_finding_can_carry() {
    // A severity of the vocabulary's own invention, which SHACL allows and the
    // specification's finding shape has no room for.
    let resolver = Variant::of(tiny_with_vocabularies()).replacing(
        "catalog.shapes.ttl",
        "sh:severity sh:Warning",
        "sh:severity ex:Critical",
    );
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

#[test]
fn reports_a_predicate_one_entry_of_the_manifest_stamps_with() {
    // A stamp predicate on one entry of the manifest, which the specification
    // gives that entry alone and no conversion.
    let entry = "<#pass> a bridge:IsomorphicConversionTest ;";
    let stamped = Variant::of(tiny_with_vocabularies()).replacing(
        "fixtures/manifest.ttl",
        entry,
        format!("{entry}\n  bridge:stampPredicate <{EX}colour> ;"),
    );
    let (conversion, _) = run(&stamped, "a-predicate-no-ontology-declares.xml");
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
