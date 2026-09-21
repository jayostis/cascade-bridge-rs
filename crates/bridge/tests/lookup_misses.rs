// An accounting entry may say where its path's values are looked up, and a
// Bridge then reports each distinct value of that path the concept map holds no
// notation for — the gap kind a verdict could never name, because it is true of
// a value and not of the path.
//
// A lookup finding is per value rather than per path: it is addressed at that
// value's first occurrence, carries the value as the record wrote it, and counts
// the nodes holding that value. So the walk that keeps a path's first occurrence
// and its count has to keep values too, and an element's value is not known when
// its start tag is read: it arrives between the start and the end, in as many
// pieces as the parser hands it over in.
use cascade_bridge::{
    convert, load_adapter, prepare, Conversion, DirectoryResolver, Resolver, Source,
};
use oxrdf::{Quad, Term};
use std::path::PathBuf;

const OA: &str = "http://www.w3.org/ns/oa#";
const SH: &str = "http://www.w3.org/ns/shacl#";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";
const BRIDGE: &str = "https://ns.cascadeprotocol.org/bridge/v1-draft#";

/// The three files a case varies, and the two gaps the committed gap scheme
/// declares that a lookup case names.
const ACCOUNTING: &str = "vocab/catalog-accounting.ttl";
const GAP_SCHEME: &str = "vocab/catalog-gaps.ttl";
const CONCEPT_MAP: &str = "vocab/catalog-statuses.ttl";
const STATUS_GAP: &str = "urn:example:catalog#statusOutsideTheTable";
const NOTE_GAP: &str = "urn:example:catalog#noteHasNoTerm";

/// The two halves of a lookup declaration, as an entry writes them: the concept
/// map relative to the accounting that names it, and the gap its misses body.
const LOOKUP_IN: &str = "bridge:lookupIn <catalog-statuses.ttl>";
const LOOKUP_NAMES_GAP: &str = "bridge:lookupNamesGap ex:statusOutsideTheTable";

fn tiny() -> DirectoryResolver {
    DirectoryResolver::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter"))
        .expect("resolver")
}

/// The whole run, from the crate to the findings, as a result: a concept map
/// may be refused at any stage of it, and which stage is not this crate's to
/// say.
fn conversion(resolver: &dyn Resolver, input: &str) -> cascade_bridge::Result<Conversion> {
    let adapter = load_adapter(resolver)?;
    let prepared = prepare(&adapter, resolver)?;
    let iri = format!("{}fixtures/in/{input}", resolver.root());
    let xml = resolver.read(&iri)?;
    convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            xml: &xml,
        },
    )
}

fn findings(resolver: &dyn Resolver, input: &str) -> Vec<Quad> {
    conversion(resolver, input).expect("conversion").findings
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

/// A term as an address or a body is read: an IRI or a literal by what it says,
/// anything else by how N-Triples writes it.
fn says(quads: &[Quad], subject: &str, predicate: &str) -> String {
    match one(quads, subject, predicate) {
        Some(Term::NamedNode(named)) => named.as_str().to_owned(),
        Some(Term::Literal(literal)) => literal.value().to_owned(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Every lookup finding a run produced, by the node it is, in the order the run
/// wrote them. A lookup's gap is of the one kind no verdict may report, so its
/// body tells a lookup finding from a census finding and from an entry's.
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

/// Each lookup finding as what it says and where it says it: the value it
/// names, the record it is about, and the occurrence inside that record it is
/// addressed at.
fn missed(findings: &[Quad]) -> Vec<(String, String, String)> {
    let mut rows: Vec<(String, String, String)> = lookups(findings)
        .iter()
        .map(|annotation| {
            let target = node(findings, annotation, &format!("{OA}hasTarget"));
            let selector = node(findings, &target, &format!("{OA}hasSelector"));
            let refinement = node(findings, &selector, &format!("{OA}refinedBy"));
            (
                says(findings, annotation, &format!("{SH}value")),
                says(findings, &selector, RDF_VALUE),
                says(findings, &refinement, RDF_VALUE),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// A row of `missed`, spelled as a test writes one.
fn row(value: &str, record: &str, within: &str) -> (String, String, String) {
    (value.to_owned(), record.to_owned(), within.to_owned())
}

/// The values a run's lookup findings name, in the order the run wrote them.
fn in_order(findings: &[Quad]) -> Vec<String> {
    lookups(findings)
        .iter()
        .map(|annotation| says(findings, annotation, &format!("{SH}value")))
        .collect()
}

/// How many nodes of the record hold this value, where the finding says.
fn count(findings: &[Quad], annotation: &str) -> Option<String> {
    match one(findings, annotation, &format!("{BRIDGE}occurrences"))? {
        Term::Literal(literal) => Some(literal.value().to_owned()),
        other => Some(other.to_string()),
    }
}

/// The one lookup finding about this value.
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

/// What every annotation a run produced bodies, so a finding standing beside
/// another at one node can be counted.
fn bodies(findings: &[Quad]) -> Vec<String> {
    let mut named: Vec<String> = findings
        .iter()
        .filter(|q| q.predicate.as_str() == RDF_TYPE)
        .filter(
            |q| matches!(&q.object, Term::NamedNode(n) if n.as_str() == format!("{OA}Annotation")),
        )
        .map(|q| says(findings, &q.subject.to_string(), &format!("{OA}hasBody")))
        .collect();
    named.sort();
    named
}

/// The tiny adapter with its accounting, its gap scheme or its concept map
/// replaced, so a declaration, a kind and a map can each be varied without any
/// of them being committed.
#[derive(Default)]
struct Adapted {
    accounting: Option<String>,
    gaps: Option<String>,
    map: Option<String>,
    directory: Option<DirectoryResolver>,
}

impl Adapted {
    fn declaring(accounting: String) -> Self {
        Self {
            accounting: Some(accounting),
            ..Self::default()
        }
    }

    fn gaps(mut self, gaps: String) -> Self {
        self.gaps = Some(gaps);
        self
    }

    fn map(mut self, map: String) -> Self {
        self.map = Some(map);
        self
    }

    fn built(mut self) -> Self {
        self.directory = Some(tiny());
        self
    }

    fn directory(&self) -> &DirectoryResolver {
        self.directory.as_ref().expect("built")
    }
}

impl Resolver for Adapted {
    fn root(&self) -> &str {
        self.directory().root()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        for (file, body) in [
            (ACCOUNTING, &self.accounting),
            (GAP_SCHEME, &self.gaps),
            (CONCEPT_MAP, &self.map),
        ] {
            if let (true, Some(body)) = (iri.ends_with(file), body) {
                return Ok(body.as_bytes().to_vec());
            }
        }
        self.directory().read(iri)
    }
}

const ACCOUNTING_PREAMBLE: &str = "@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#> .\n@prefix ex:     <urn:example:catalog#> .\n";

const GAPS_PREAMBLE: &str = "@prefix skos:   <http://www.w3.org/2004/02/skos/core#> .\n@prefix sh:     <http://www.w3.org/ns/shacl#> .\n@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#> .\n@prefix ex:     <urn:example:catalog#> .\n";

const MAP_PREAMBLE: &str =
    "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n@prefix ex:   <urn:example:catalog#> .\n";

/// One entry, spelled as an accounting spells it, carrying only the
/// declarations a case turns on.
fn entry(path: &str, verdict: &str, declarations: &[&str]) -> String {
    let said: String = declarations
        .iter()
        .map(|declaration| format!(" ;\n   {declaration}"))
        .collect();
    format!(
        "\n[] a bridge:PathEntry ;\n   bridge:sourcePath \"{path}\" ;\n   bridge:verdict bridge:{verdict}{said} .\n"
    )
}

/// The entry a case is about: a path whose values the committed concept map is
/// looked up in, under whichever verdict the case names.
fn looks_up(path: &str, verdict: &str) -> String {
    entry(path, verdict, &[LOOKUP_IN, LOOKUP_NAMES_GAP])
}

fn accounting(entries: &[String]) -> String {
    format!("{ACCOUNTING_PREAMBLE}{}", entries.concat())
}

/// An accounting whose one entry looks its notes up, which is every case about
/// what a record holds rather than about what an entry says.
fn notes() -> Adapted {
    Adapted::declaring(accounting(&[looks_up("/item/note", "consumed")])).built()
}

/// One gap concept, under the kind and at the severity a case turns on.
fn concept(name: &str, kind: &str, severity: Option<&str>) -> String {
    let declared = match severity {
        Some(severity) => format!(" ;\n  sh:resultSeverity sh:{severity}"),
        None => String::new(),
    };
    format!("\nex:{name} a skos:Concept ;\n  skos:inScheme ex:gaps ;\n  skos:broader bridge:{kind}{declared} .\n")
}

fn gap_scheme(concepts: &[String]) -> String {
    format!(
        "{GAPS_PREAMBLE}\nex:gaps a skos:ConceptScheme .\n{}",
        concepts.concat()
    )
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
fn writes_the_count_as_an_xsd_integer() {
    let found = findings(&notes(), "lookup-a-value-at-three-nodes.xml");
    let occurrences = one(
        &found,
        &about(&found, "Retired"),
        &format!("{BRIDGE}occurrences"),
    )
    .expect("a count");
    let Term::Literal(literal) = occurrences else {
        panic!("a count is a literal: {occurrences}");
    };
    assert_eq!(literal.datatype().as_str(), XSD_INTEGER);
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
        "a key is the value case-folded and whitespace-trimmed, and a notation is written that way"
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
    let shelves =
        Adapted::declaring(accounting(&[looks_up("/item/shelf/mark", "carried")])).built();
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
    let shelves = Adapted::declaring(accounting(&[looks_up("/item/shelf", "carried")])).built();
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
    let attributes = Adapted::declaring(accounting(&[
        looks_up("/item/shelf/@status", "carried"),
        looks_up("/item/@id", "carried"),
    ]))
    .built();
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
        let severity = Adapted::declaring(accounting(&[looks_up("/item/note", "consumed")]))
            .gaps(gap_scheme(&[concept(
                "statusOutsideTheTable",
                "valueNotMapped",
                declared,
            )]))
            .built();
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
        let stated = Adapted::declaring(accounting(&[looks_up("/item/note", verdict)])).built();
        assert_eq!(
            missed(&findings(&stated, "lookup-a-value-at-three-nodes.xml")),
            [row("Retired", "/catalog/item[1]", "note[2]")],
            "a verdict is about the path and a lookup is about the values it holds: bridge:{verdict}"
        );
    }
}

#[test]
fn stands_a_lookup_finding_beside_the_gap_the_same_entry_names_and_the_query_constructs() {
    let both = Adapted::declaring(accounting(&[entry(
        "/item/note",
        "noHome",
        &[
            "bridge:namesGap ex:noteHasNoTerm",
            LOOKUP_IN,
            LOOKUP_NAMES_GAP,
        ],
    )]))
    .built();
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
    let half =
        Adapted::declaring(accounting(&[entry("/item/note", "consumed", &[LOOKUP_IN])])).built();
    let Err(refusal) = conversion(&half, "two.xml") else {
        panic!("a map with no gap to report into is read as an entry declaring no lookup");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("/item/note"), "{refusal}");
}

#[test]
fn refuses_an_entry_naming_a_gap_for_misses_and_no_map_to_look_in() {
    let half = Adapted::declaring(accounting(&[entry(
        "/item/note",
        "consumed",
        &[LOOKUP_NAMES_GAP],
    )]))
    .built();
    let Err(refusal) = conversion(&half, "two.xml") else {
        panic!("a gap with no map to miss in is read as an entry declaring no lookup");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("/item/note"), "{refusal}");
}

#[test]
fn refuses_an_entry_whose_lookup_names_a_gap_the_gap_scheme_does_not_declare() {
    let unwritten = Adapted::declaring(accounting(&[entry(
        "/item/note",
        "consumed",
        &[LOOKUP_IN, "bridge:lookupNamesGap ex:noGapAnyoneDeclared"],
    )]))
    .built();
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
        let mistyped = Adapted::declaring(accounting(&[looks_up("/item/note", "consumed")]))
            .gaps(gap_scheme(&[concept("statusOutsideTheTable", kind, None)]))
            .built();
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
        let both = Adapted::declaring(accounting(&[entry(
            "/item/note",
            "consumed",
            &[LOOKUP_IN, LOOKUP_NAMES_GAP, twice],
        )]))
        .built();
        let Err(refusal) = conversion(&both, "two.xml") else {
            panic!("an entry declaring {twice} is read at whichever the parse saw last");
        };
        let refusal = refusal.to_string();
        assert!(refusal.contains("/item/note"), "{refusal}");
    }
}

/// Neither Turtle nor anything else: a concept map that cannot be read at all.
const UNPARSEABLE: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#\n";

#[test]
fn refuses_a_concept_map_that_does_not_parse() {
    let broken = Adapted::declaring(accounting(&[looks_up("/item/note", "consumed")]))
        .map(UNPARSEABLE.to_owned())
        .built();
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
        let wrong = Adapted::declaring(accounting(&[looks_up("/item/note", "consumed")]))
            .map(concept_map(scheme, &[maps("current")]))
            .built();
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
fn looks_a_value_up_in_the_one_scheme_the_named_file_holds() {
    let narrowed = Adapted::declaring(accounting(&[looks_up("/item/note", "consumed")]))
        .map(concept_map(
            "ex:statuses a skos:ConceptScheme .",
            &[maps("current"), maps("pending")],
        ))
        .built();
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
