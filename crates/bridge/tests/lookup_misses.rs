mod common;

use common::{
    accounting, address, annotations, concept, conversion, count, entry, findings, gap_scheme,
    node, says, tiny, Variant, ACCOUNTING, ACCOUNTING_PREAMBLE, CONCEPT_MAP, GAP_SCHEME, NOTE_GAP,
    OA, SH,
};
use oxrdf::{Quad, Term};

const STATUS_GAP: &str = "urn:example:catalog#statusOutsideTheTable";

const LOOKUP_IN: &str = "bridge:lookupIn <catalog-statuses.ttl>";
const LOOKUP_NAMES_GAP: &str = "bridge:lookupNamesGap ex:statusOutsideTheTable";

/// In the order the run wrote them; a lookup's gap kind tells it from every other finding.
fn lookups(findings: &[Quad]) -> Vec<String> {
    let mut annotations: Vec<String> = Vec::new();
    for quad in findings {
        if quad.predicate.as_str() != format!("{OA}hasBody") {
            continue;
        }
        if !matches!(&quad.object, Term::NamedNode(body) if body.as_str() == STATUS_GAP) {
            continue;
        }
        let annotation = quad.subject.to_string();
        if !annotations.contains(&annotation) {
            annotations.push(annotation);
        }
    }
    annotations
}

/// Each lookup finding as its value, its record, and the occurrence it is addressed at.
fn missed(findings: &[Quad]) -> Vec<(String, String, String)> {
    let mut rows: Vec<(String, String, String)> = lookups(findings)
        .iter()
        .map(|annotation| {
            let (record, within) = address(findings, annotation);
            (
                says(findings, annotation, &format!("{SH}value")),
                record,
                within,
            )
        })
        .collect();
    rows.sort();
    rows
}

fn row(value: &str, record: &str, within: &str) -> (String, String, String) {
    (value.to_owned(), record.to_owned(), within.to_owned())
}

fn in_order(findings: &[Quad]) -> Vec<String> {
    lookups(findings)
        .iter()
        .map(|annotation| says(findings, annotation, &format!("{SH}value")))
        .collect()
}

fn about(findings: &[Quad], value: &str) -> String {
    let mut found = lookups(findings)
        .into_iter()
        .filter(|annotation| says(findings, annotation, &format!("{SH}value")) == value);
    let first = found.next();
    assert!(first.is_some(), "no lookup finding names {value}");
    assert!(
        found.next().is_none(),
        "more than one finding names {value}"
    );
    first.expect("checked above")
}

fn bodies(findings: &[Quad]) -> Vec<String> {
    let mut named: Vec<String> = annotations(findings)
        .iter()
        .map(|annotation| says(findings, annotation, &format!("{OA}hasBody")))
        .collect();
    named.sort();
    named
}

const MAP_PREAMBLE: &str =
    "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n@prefix ex:   <urn:example:catalog#> .\n";

fn looks_up(path: &str, verdict: &str) -> String {
    entry(path, verdict, &[LOOKUP_IN, LOOKUP_NAMES_GAP])
}

fn declaring(accounting: String) -> Variant {
    Variant::of(tiny()).with(ACCOUNTING, accounting)
}

/// An accounting whose one entry looks its notes up.
fn notes() -> Variant {
    declaring(accounting(&[looks_up("/item/note", "consumed")]))
}

/// One concept of a map, its notation the source phrase as a key is written.
fn maps(notation: &str) -> String {
    format!(
        "\nex:status-{notation} a skos:Concept ;\n  skos:inScheme ex:statuses ;\n  skos:notation \"{notation}\" ;\n  skos:exactMatch ex:Status .\n"
    )
}

fn concept_map(scheme: &str, concepts: &[String]) -> String {
    format!("{MAP_PREAMBLE}\n{scheme}\n{}", concepts.concat())
}

#[test]
fn reports_a_value_no_concept_of_the_scheme_carries_at_that_value_s_first_occurrence() {
    let found = findings(&notes(), "lookup-a-value-at-three-nodes.xml");
    assert_eq!(
        missed(&found),
        [row("Retired", "/catalog/item[1]", "note[2]")],
        "the path's first occurrence holds a value the map carries; the finding is the value's"
    );
}

#[test]
fn counts_the_nodes_of_the_record_holding_that_value_and_carries_no_count_for_one() {
    let three = findings(&notes(), "lookup-a-value-at-three-nodes.xml");
    assert_eq!(
        count(&three, &about(&three, "Retired")),
        Some("3".to_owned()),
        "one finding stands for every node holding the value"
    );

    let once = findings(&notes(), "every-verdict.xml");
    assert_eq!(
        missed(&once),
        [row(
            "a note the mapping has no term for",
            "/catalog/item[1]",
            "note[1]"
        )]
    );
    assert_eq!(
        count(&once, &about(&once, "a note the mapping has no term for")),
        None,
        "a count of one is what its absence says"
    );
}

#[test]
fn reports_two_different_unmapped_values_at_one_path_twice_sorted_by_value() {
    let found = findings(&notes(), "lookup-two-values-at-one-path.xml");
    assert_eq!(
        missed(&found),
        [
            row("Pending", "/catalog/item[1]", "note[2]"),
            row("Retired", "/catalog/item[1]", "note[1]")
        ]
    );
    assert_eq!(
        in_order(&found),
        ["Pending", "Retired"],
        "a graph has no order, and a run's output does not depend on which way a hash fell"
    );
}

#[test]
fn treats_a_value_differing_from_a_notation_only_by_case_or_by_edge_whitespace_as_no_miss() {
    assert_eq!(
        missed(&findings(&notes(), "lookup-near-misses.xml")),
        [row("Retired", "/catalog/item[1]", "note[4]")],
        "a key is the value lowercased and stripped of leading and trailing XML whitespace, \
         and a notation is written that way"
    );
}

#[test]
fn reports_nothing_for_a_path_the_record_does_not_hold_or_a_value_empty_once_trimmed() {
    assert_eq!(
        missed(&findings(&notes(), "lookup-absent-and-empty.xml")),
        [row("Retired", "/catalog/item[2]", "note[3]")],
        "absence is bridge:sourceLacksRequired's case and has a gap of its own"
    );
}

#[test]
fn reads_an_element_s_value_out_of_every_piece_the_parser_hands_its_text_over_in() {
    let found = findings(&notes(), "lookup-text-in-several-events.xml");
    assert_eq!(
        missed(&found),
        [row("Retired", "/catalog/item[1]", "note[1]")],
        "a CDATA section, a comment and a character reference each break a run of text in two"
    );
    assert_eq!(
        count(&found, &about(&found, "Retired")),
        Some("3".to_owned())
    );
}

#[test]
fn addresses_a_value_at_its_first_occurrence_however_deep_under_a_repeated_ancestor_it_stands() {
    let shelves = declaring(accounting(&[looks_up("/item/shelf/mark", "carried")]));
    let found = findings(&shelves, "lookup-under-a-repeated-ancestor.xml");
    assert_eq!(
        missed(&found),
        [row("Retired", "/catalog/item[1]", "shelf[2]/mark[2]")],
        "the first mark of the record, and the first mark of its shelf, both hold a mapped value"
    );
    assert_eq!(
        count(&found, &about(&found, "Retired")),
        Some("2".to_owned())
    );
}

#[test]
fn reports_nothing_at_an_element_with_an_element_child_whatever_its_text() {
    let shelves = declaring(accounting(&[looks_up("/item/shelf", "carried")]));
    let found = findings(&shelves, "lookup-element-with-an-element-child.xml");
    assert_eq!(
        missed(&found),
        [row("Retired", "/catalog/item[2]", "shelf[2]")],
        "a lookup is a leaf's: an element with an element child has no value at all"
    );
    assert_eq!(
        count(&found, &about(&found, "Retired")),
        None,
        "the shelves carrying a mark hold no value, so nothing but the plain one is counted"
    );
}

#[test]
fn addresses_an_attribute_s_lookup_finding_at_the_element_the_attribute_stands_on() {
    let attributes = declaring(accounting(&[
        looks_up("/item/shelf/@status", "carried"),
        looks_up("/item/@id", "carried"),
    ]));
    assert_eq!(
        missed(&findings(&attributes, "lookup-on-an-attribute.xml")),
        [
            row("1", "/catalog/item[1]", ""),
            row("Retired", "/catalog/item[1]", "shelf[2]")
        ],
        "an XPath selector has no form for an attribute, and an attribute of the record element \
         is refined no further"
    );
}

#[test]
fn reports_each_spelling_of_one_key_the_map_lacks_as_the_record_wrote_it() {
    let found = findings(&notes(), "lookup-two-spellings-of-one-key.xml");
    assert_eq!(
        missed(&found),
        [
            row("RETIRED", "/catalog/item[1]", "note[2]"),
            row("Retired", "/catalog/item[1]", "note[1]")
        ],
        "a finding is per distinct value, and its sh:value is the value and never the key"
    );
    assert_eq!(
        count(&found, &about(&found, "Retired")),
        Some("2".to_owned())
    );
    assert_eq!(count(&found, &about(&found, "RETIRED")), None);
}

#[test]
fn bodies_a_lookup_finding_at_the_gap_the_entry_s_lookup_names_and_motivates_it_by_classifying() {
    let found = findings(&notes(), "lookup-a-value-at-three-nodes.xml");
    let annotation = about(&found, "Retired");
    assert_eq!(
        says(&found, &annotation, &format!("{OA}hasBody")),
        STATUS_GAP
    );
    assert_eq!(
        says(&found, &annotation, &format!("{OA}motivatedBy")),
        format!("{OA}classifying")
    );
    let target = node(&found, &annotation, &format!("{OA}hasTarget"));
    assert!(
        says(&found, &target, &format!("{OA}hasSource"))
            .ends_with("fixtures/in/lookup-a-value-at-three-nodes.xml"),
        "{found:?}"
    );
}

#[test]
fn takes_the_severity_the_gap_declares_and_sh_info_where_it_declares_none() {
    for (declared, reported) in [(Some("Warning"), "Warning"), (None, "Info")] {
        let severity = declaring(accounting(&[looks_up("/item/note", "consumed")])).with(
            GAP_SCHEME,
            gap_scheme(&[concept("statusOutsideTheTable", "valueNotMapped", declared)]),
        );
        let found = findings(&severity, "lookup-a-value-at-three-nodes.xml");
        assert_eq!(
            says(
                &found,
                &about(&found, "Retired"),
                &format!("{SH}resultSeverity")
            ),
            format!("{SH}{reported}"),
            "severity is a property of the kind of problem, not of one occurrence of it"
        );
    }
}

#[test]
fn reads_a_lookup_whatever_the_entry_s_verdict_is() {
    for verdict in [
        "carried",
        "carriedInPart",
        "redundantWith",
        "consumed",
        "noHome",
        "ignored",
    ] {
        let stated = declaring(accounting(&[looks_up("/item/note", verdict)]));
        assert_eq!(
            missed(&findings(&stated, "lookup-a-value-at-three-nodes.xml")),
            [row("Retired", "/catalog/item[1]", "note[2]")],
            "a verdict is about the path and a lookup is about the values it holds: bridge:{verdict}"
        );
    }
}

#[test]
fn stands_a_lookup_finding_beside_the_gap_the_same_entry_names_and_the_query_constructs() {
    let both = declaring(accounting(&[entry(
        "/item/note",
        "noHome",
        &[
            "bridge:namesGap ex:noteHasNoTerm",
            LOOKUP_IN,
            LOOKUP_NAMES_GAP,
        ],
    )]));
    let found = findings(&both, "every-verdict.xml");
    assert_eq!(
        bodies(&found)
            .into_iter()
            .filter(|body| body == NOTE_GAP || body == STATUS_GAP)
            .collect::<Vec<String>>(),
        [NOTE_GAP, NOTE_GAP, STATUS_GAP],
        "the entry reports from both declarations, and the query's finding stands beside them"
    );
}

#[test]
fn refuses_an_entry_declaring_a_lookup_in_and_no_gap_for_its_misses() {
    let half = declaring(accounting(&[entry("/item/note", "consumed", &[LOOKUP_IN])]));
    let Err(refusal) = conversion(&half, "two.xml") else {
        panic!("a map with no gap to report into is read as an entry declaring no lookup");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("/item/note"), "{refusal}");
}

#[test]
fn refuses_an_entry_naming_a_gap_for_misses_and_no_map_to_look_in() {
    let half = declaring(accounting(&[entry(
        "/item/note",
        "consumed",
        &[LOOKUP_NAMES_GAP],
    )]));
    let Err(refusal) = conversion(&half, "two.xml") else {
        panic!("a gap with no map to miss in is read as an entry declaring no lookup");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("/item/note"), "{refusal}");
}

#[test]
fn refuses_an_entry_whose_lookup_names_a_gap_the_gap_scheme_does_not_declare() {
    let unwritten = declaring(accounting(&[entry(
        "/item/note",
        "consumed",
        &[LOOKUP_IN, "bridge:lookupNamesGap ex:noGapAnyoneDeclared"],
    )]));
    let Err(refusal) = conversion(&unwritten, "two.xml") else {
        panic!("a gap nothing declares is read as a gap a miss can be bodied at");
    };
    let refusal = refusal.to_string();
    assert!(
        refusal.contains("/item/note") && refusal.contains("noGapAnyoneDeclared"),
        "{refusal}"
    );
}

#[test]
fn refuses_an_entry_whose_lookup_names_a_gap_of_a_kind_other_than_value_not_mapped() {
    for kind in [
        "noPredicate",
        "sourceLacksRequired",
        "carriedWithLoss",
        "schemaRuleUnnamed",
    ] {
        let mistyped = declaring(accounting(&[looks_up("/item/note", "consumed")])).with(
            GAP_SCHEME,
            gap_scheme(&[concept("statusOutsideTheTable", kind, None)]),
        );
        let Err(refusal) = conversion(&mistyped, "two.xml") else {
            panic!("a lookup reports a gap of kind {kind}, which is true of something else");
        };
        let refusal = refusal.to_string();
        assert!(refusal.contains("/item/note"), "{kind}: {refusal}");
    }
}

#[test]
fn refuses_an_entry_declaring_two_maps_or_two_gaps_for_its_misses() {
    for twice in [
        "bridge:lookupIn <catalog-statuses.ttl>, <catalog-gaps.ttl>",
        "bridge:lookupNamesGap ex:statusOutsideTheTable, ex:noteHasNoTerm",
    ] {
        let both = declaring(accounting(&[entry(
            "/item/note",
            "consumed",
            &[LOOKUP_IN, LOOKUP_NAMES_GAP, twice],
        )]));
        let Err(refusal) = conversion(&both, "two.xml") else {
            panic!("an entry declaring {twice} is read at whichever the parse saw last");
        };
        let refusal = refusal.to_string();
        assert!(refusal.contains("/item/note"), "{refusal}");
    }
}

const UNPARSEABLE: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#\n";

#[test]
fn refuses_a_concept_map_that_does_not_parse() {
    let broken =
        declaring(accounting(&[looks_up("/item/note", "consumed")])).with(CONCEPT_MAP, UNPARSEABLE);
    let Err(refusal) = conversion(&broken, "two.xml") else {
        panic!("a map that is not Turtle is read as a scheme holding no notation at all");
    };
    let refusal = refusal.to_string();
    assert!(
        refusal.contains("/item/note") && refusal.contains(CONCEPT_MAP),
        "{refusal}"
    );
}

#[test]
fn refuses_a_concept_map_holding_two_concept_schemes_or_none() {
    for scheme in [
        "ex:statuses a skos:ConceptScheme .\nex:others a skos:ConceptScheme .",
        "",
    ] {
        let wrong = declaring(accounting(&[looks_up("/item/note", "consumed")]))
            .with(CONCEPT_MAP, concept_map(scheme, &[maps("current")]));
        let Err(refusal) = conversion(&wrong, "two.xml") else {
            panic!("a file holding {scheme:?} is read as though one scheme in it were the lookup");
        };
        let refusal = refusal.to_string();
        assert!(
            refusal.contains("/item/note") && refusal.contains(CONCEPT_MAP),
            "{refusal}"
        );
    }
}

#[test]
fn reports_a_miss_once_for_an_entry_writing_its_path_twice() {
    let twice = declaring(format!(
        "{ACCOUNTING_PREAMBLE}\n[] a bridge:PathEntry ;\n   bridge:sourcePath \"/item/note\", \"/item/note\" ;\n   bridge:verdict bridge:consumed ;\n   {LOOKUP_IN} ;\n   {LOOKUP_NAMES_GAP} .\n"
    ));
    assert_eq!(
        missed(&findings(&twice, "lookup-a-value-at-three-nodes.xml")),
        [row("Retired", "/catalog/item[1]", "note[2]")]
    );
}

#[test]
fn looks_a_value_up_in_the_one_scheme_the_named_file_holds() {
    let narrowed = declaring(accounting(&[looks_up("/item/note", "consumed")])).with(
        CONCEPT_MAP,
        concept_map(
            "ex:statuses a skos:ConceptScheme .",
            &[maps("current"), maps("pending")],
        ),
    );
    assert_eq!(
        missed(&findings(&narrowed, "lookup-two-values-at-one-path.xml")),
        [row("Retired", "/catalog/item[1]", "note[1]")],
        "the notations of the named file's scheme are the whole of what a Bridge reads from a map"
    );
}

#[test]
fn reports_a_value_a_no_break_space_pads_though_the_map_holds_the_unpadded_key() {
    let found = findings(&notes(), "lookup-a-value-padded-with-a-no-break-space.xml");
    assert_eq!(
        missed(&found),
        [row("current\u{a0}", "/catalog/item[1]", "note[1]")],
        "a mapping's trim is XPath's whitespace class, which is XML's S production and holds no \
         no-break space, so a key trimmed of a wider set finds a notation the mapping cannot"
    );
}
