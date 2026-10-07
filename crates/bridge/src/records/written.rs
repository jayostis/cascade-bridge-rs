use super::{canonical_nquads, ni_name, PLACEHOLDER};
use crate::fixtures::{converted_with_facts, fixture, objects, tiny, versioned, Variant, CRATE};
use crate::terms::{
    BRIDGE_ARRIVED_AS, BRIDGE_SELECTOR, PAV_LAST_UPDATE_ON, PAV_VERSION, PROV_ACTIVITY, PROV_AGENT,
    PROV_WAS_DERIVED_FROM, PROV_WAS_GENERATED_BY, RDFS_LABEL, RDF_TYPE,
};
use oxrdf::{NamedNode, Quad, Term};
use std::collections::BTreeSet;

const EX: &str = "urn:example:catalog#";

fn two() -> String {
    ni_name(&fixture("tests/tiny-adapter/fixtures/in/two.xml"))
}

fn said(quads: &[Quad], subject: &str, predicate: &str) -> BTreeSet<String> {
    objects(quads, subject, predicate)
        .into_iter()
        .map(|term| match term {
            Term::Literal(literal) => literal.value().to_owned(),
            other => other.to_string(),
        })
        .collect()
}

fn subjects(quads: &[Quad], predicate: &str) -> BTreeSet<String> {
    quads
        .iter()
        .filter(|quad| quad.predicate.as_str() == predicate)
        .map(|quad| quad.subject.to_string())
        .collect()
}

/// The tiny adapter with its item mapping replaced by one reading what the dataset holds.
fn reading(construct: &str, matching: &str) -> Variant {
    Variant::of(tiny()).with(
        "mapping/item.rq",
        format!(
            "PREFIX fx: <http://sparql.xyz/facade-x/ns/>
PREFIX xyz: <http://sparql.xyz/facade-x/data/>
PREFIX bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#>
PREFIX ex: <{EX}>
CONSTRUCT {{ {construct} }}
WHERE {{
  ?item a fx:root, xyz:item ; xyz:id ?id .
  BIND(IRI(CONCAT(\"urn:example:item:\", ?id)) AS ?s)
  {matching}
}}"
        ),
    )
}

#[test]
fn loads_the_facts_the_document_s_name_and_the_record_s_selector_into_each_record_s_dataset() {
    let resolver = reading(
        "?s ex:server ?server ; ex:document ?document ; ex:at ?at .",
        "bridge:thisDocument bridge:serverBaseUrl ?server ; bridge:sha256 ?document .
         bridge:thisRecord bridge:selector ?at .",
    );
    let quads = converted_with_facts(&resolver, "two.xml").quads;
    for (item, at) in [("1", "/catalog/item[1]"), ("2", "/catalog/item[2]")] {
        let item = format!("<urn:example:item:{item}>");
        assert_eq!(
            said(&quads, &item, &format!("{EX}server")),
            BTreeSet::from(["http://shop.example/Catalog".to_owned()]),
            "the supplied server, normalised"
        );
        assert_eq!(
            said(&quads, &item, &format!("{EX}document")),
            BTreeSet::from([two()])
        );
        assert_eq!(
            said(&quads, &item, &format!("{EX}at")),
            BTreeSet::from([at.to_owned()])
        );
    }
}

#[test]
fn loads_the_table_every_record_of_the_document_constructs_beside_each_record() {
    let resolver = reading("?s ex:seen ?other .", "?holder ex:holds ?other .")
        .with(
            "mapping/table.rq",
            format!(
                "PREFIX fx: <http://sparql.xyz/facade-x/ns/>
PREFIX xyz: <http://sparql.xyz/facade-x/data/>
PREFIX ex: <{EX}>
CONSTRUCT {{ [] ex:holds ?id }} WHERE {{ ?item a fx:root, xyz:item ; xyz:id ?id }}"
            ),
        )
        .replacing(
            CRATE,
            "\"bridge:detectQuery\": \"bridge:detectQuery\",",
            "\"bridge:detectQuery\": \"bridge:detectQuery\",\n      \"bridge:documentTableQuery\": \"bridge:documentTableQuery\",",
        )
        .replacing(
            CRATE,
            "\"bridge:detectQuery\": { \"@id\": \"mapping/detect.rq\" },",
            "\"bridge:detectQuery\": { \"@id\": \"mapping/detect.rq\" },\n      \"bridge:documentTableQuery\": { \"@id\": \"mapping/table.rq\" },",
        );
    let quads = converted_with_facts(&resolver, "two.xml").quads;
    for item in ["<urn:example:item:1>", "<urn:example:item:2>"] {
        assert_eq!(
            said(&quads, item, &format!("{EX}seen")),
            BTreeSet::from(["1".to_owned(), "2".to_owned()]),
            "{item} reads what every record of its document holds"
        );
    }
}

#[test]
fn folds_each_version_s_arrival_into_one_node_naming_its_selector_its_document_and_the_import() {
    let quads = converted_with_facts(&versioned(), "two.xml").quads;
    let imports: Vec<String> = quads
        .iter()
        .filter(|quad| quad.predicate.as_str() == RDF_TYPE)
        .filter(|quad| matches!(&quad.object, Term::NamedNode(class) if class.as_str() == PROV_ACTIVITY))
        .map(|quad| quad.subject.to_string())
        .collect();
    let [import] = imports.as_slice() else {
        panic!("one import: {imports:?}");
    };
    let arrivals = subjects(&quads, BRIDGE_ARRIVED_AS);
    assert_eq!(arrivals.len(), 2, "{arrivals:?}");
    let mut arrived: Vec<(String, String)> = Vec::new();
    for arrival in &arrivals {
        let version = said(&quads, arrival, BRIDGE_ARRIVED_AS);
        assert_eq!(version.len(), 1, "{arrival} arrived as {version:?}");
        assert!(
            version.iter().all(|v| v.starts_with("<ni:///sha-256;")),
            "{version:?}"
        );
        assert_eq!(
            said(&quads, arrival, PROV_WAS_DERIVED_FROM),
            BTreeSet::from([format!("<{}>", two())])
        );
        assert_eq!(
            said(&quads, arrival, PROV_WAS_GENERATED_BY),
            BTreeSet::from([import.clone()])
        );
        let selector = said(&quads, arrival, BRIDGE_SELECTOR);
        let written = said(&quads, arrival, PAV_VERSION);
        assert_eq!((selector.len(), written.len()), (1, 1), "{arrival}");
        arrived.extend(selector.into_iter().zip(written));
    }
    arrived.sort();
    assert_eq!(
        arrived,
        [
            ("/catalog/item[1]".to_owned(), "1".to_owned()),
            ("/catalog/item[2]".to_owned(), "2".to_owned())
        ],
        "each arrival carries what the mapping wrote of it, and its record's selector"
    );
}

#[test]
fn keeps_an_attribution_a_mapping_writes_of_the_document_beside_the_supplied_facts() {
    let resolver = reading(
        "?document <http://www.w3.org/ns/prov#qualifiedAttribution> [
           <http://www.w3.org/ns/prov#agent> [ <http://www.w3.org/2000/01/rdf-schema#label> \"a clinic\" ] ] .",
        "bridge:thisDocument bridge:sha256 ?sha256 . BIND(IRI(?sha256) AS ?document)",
    );
    let quads = converted_with_facts(&resolver, "two.xml").quads;
    let agents: BTreeSet<String> = objects(
        &quads,
        &format!("<{}>", two()),
        "http://www.w3.org/ns/prov#qualifiedAttribution",
    )
    .iter()
    .flat_map(|attribution| said(&quads, &attribution.to_string(), PROV_AGENT))
    .flat_map(|agent| said(&quads, &agent, RDFS_LABEL))
    .collect();
    assert!(agents.contains("a clinic"), "{agents:?}");
}

#[test]
fn writes_the_same_graph_byte_for_byte_in_every_run() {
    let resolver = versioned().with(
        "fixtures/facts/catalog.ttl",
        "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix prov: <http://www.w3.org/ns/prov#> .
@prefix bridge: <https://ns.cascadeprotocol.org/bridge/v1-draft#> .
bridge:thisDocument prov:qualifiedAttribution [ prov:agent [ rdfs:label \"a shop\" ] ] .",
    );
    let written = |_| {
        converted_with_facts(&resolver, "two.xml")
            .quads
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
    };
    assert_eq!(written(1), written(2));
}

#[test]
fn keeps_the_selector_a_mapping_wrote_on_an_arrival_and_adds_none() {
    let resolver = versioned().replacing(
        "mapping/item.rq",
        "pav:version ?id .",
        "pav:version ?id ; bridge:selector \"/held\" .",
    );
    let quads = converted_with_facts(&resolver, "two.xml").quads;
    let arrivals = subjects(&quads, BRIDGE_ARRIVED_AS);
    assert_eq!(arrivals.len(), 2, "{arrivals:?}");
    for arrival in &arrivals {
        assert_eq!(
            said(&quads, arrival, BRIDGE_SELECTOR),
            BTreeSet::from(["/held".to_owned()]),
            "{arrival}"
        );
    }
}

#[test]
fn keeps_an_arrival_s_last_update_in_the_source_s_own_text() {
    let resolver = versioned()
        .replacing(
            "mapping/item.rq",
            "pav:version ?id .",
            "pav:version ?id ; pav:lastUpdateOn ?updated .",
        )
        .replacing(
            "mapping/item.rq",
            "rdf:_1 ?title . }",
            "rdf:_1 ?title . }
  BIND(STRDT(?title, <http://www.w3.org/2001/XMLSchema#dateTime>) AS ?updated)",
        )
        .replacing(
            "fixtures/in/two.xml",
            "<title>First</title>",
            "<title>2026-01-01T00:00:00.000+01:00</title>",
        );
    let quads = converted_with_facts(&resolver, "two.xml").quads;
    let written: BTreeSet<String> = quads
        .iter()
        .filter(|quad| quad.predicate.as_str() == PAV_LAST_UPDATE_ON)
        .map(|quad| quad.object.to_string())
        .collect();
    assert_eq!(
        written,
        BTreeSet::from([
            "\"2026-01-01T00:00:00.000+01:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime>"
                .to_owned()
        ])
    );
}

#[test]
fn leaves_a_version_s_last_update_as_its_name_was_hashed_over() {
    let resolver = versioned()
        .replacing(
            "mapping/item.rq",
            "ex:title ?title .",
            "ex:title ?title ; pav:lastUpdateOn ?updated .",
        )
        .replacing(
            "mapping/item.rq",
            "rdf:_1 ?title . }",
            "rdf:_1 ?title . }
  BIND(STRDT(?title, <http://www.w3.org/2001/XMLSchema#dateTime>) AS ?updated)",
        )
        .replacing(
            "fixtures/in/two.xml",
            "<title>First</title>",
            "<title>2026-01-01T00:00:00.000+01:00</title>",
        );
    let quads = converted_with_facts(&resolver, "two.xml").quads;
    let names = BTreeSet::from_iter(quads.iter().filter_map(|quad| match &quad.object {
        Term::NamedNode(version) if quad.predicate.as_str() == BRIDGE_ARRIVED_AS => {
            Some(version.clone())
        }
        _ => None,
    }));
    assert_eq!(names.len(), 2, "{names:?}");
    for name in names {
        let content: Vec<Quad> = quads
            .iter()
            .filter(|quad| quad.subject == name.clone().into())
            .map(|quad| {
                Quad::new(
                    NamedNode::new_unchecked(PLACEHOLDER),
                    quad.predicate.clone(),
                    quad.object.clone(),
                    quad.graph_name.clone(),
                )
            })
            .collect();
        assert_eq!(
            ni_name(canonical_nquads(content).expect("canonical").as_bytes()),
            name.as_str()
        );
    }
}
