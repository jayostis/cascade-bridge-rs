mod common;

use cascade_bridge::{load_adapter, prepare, run_manifest, RunOptions};
use common::{tiny, Variant, CRATE};

const MANIFEST: &str = "fixtures/manifest.ttl";

#[test]
fn a_mapping_the_crate_names_and_the_directory_lacks_is_refused_by_its_name() {
    let variant = Variant::of(tiny()).replacing_exactly(
        CRATE,
        "mapping/item-tag.rq",
        "mapping/missing.rq",
        1,
    );
    let adapter = load_adapter(&variant).expect("the crate loads");
    let refused = prepare(&adapter, &variant)
        .err()
        .expect("a missing mapping is refused")
        .to_string();
    assert!(refused.contains("missing.rq"), "{refused}");
}

#[test]
fn a_manifest_that_is_not_turtle_is_refused_by_its_name() {
    let variant = Variant::of(tiny()).replacing_exactly(
        MANIFEST,
        "<> a mf:Manifest ;",
        "<> a mf:Manifest ; {",
        1,
    );
    let refused = load_adapter(&variant)
        .err()
        .expect("a manifest that is not Turtle is refused")
        .to_string();
    assert!(refused.contains("manifest.ttl"), "{refused}");
}

#[test]
fn a_table_that_is_not_turtle_is_refused_by_its_name() {
    let variant = Variant::of(tiny())
        .replacing_exactly(
            CRATE,
            r#""bridge:testManifest": { "@id": "fixtures/manifest.ttl" }"#,
            r#""bridge:testManifest": { "@id": "fixtures/manifest.ttl" },
      "bridge:table": { "@id": "vocab/broken-table.ttl" }"#,
            1,
        )
        .replacing_exactly(
            CRATE,
            r#""@id": "bridge:sparql-1.1","#,
            r#""@id": "vocab/broken-table.ttl",
      "@type": "File",
      "encodingFormat": "text/turtle"
    },
    {
      "@id": "bridge:sparql-1.1","#,
            1,
        )
        .with(
            "vocab/broken-table.ttl",
            "<urn:example:a> <urn:example:b> {",
        );
    let adapter = load_adapter(&variant).expect("the crate loads");
    let refused = prepare(&adapter, &variant)
        .err()
        .expect("a table that is not Turtle is refused")
        .to_string();
    assert!(refused.contains("broken-table.ttl"), "{refused}");
}

#[test]
fn an_expected_graph_that_is_not_turtle_is_reported_by_its_name() {
    let variant = Variant::of(tiny()).with(
        "fixtures/expected/two.ttl",
        "<urn:example:a> <urn:example:b> {",
    );
    let adapter = load_adapter(&variant).expect("the crate loads");
    let results = run_manifest(&adapter, &variant, RunOptions::default()).expect("the manifest");
    let reported: Vec<&str> = results.iter().map(|r| r.description.as_str()).collect();
    assert!(
        reported.iter().any(|d| d.contains("expected/two.ttl")),
        "{reported:#?}"
    );
}
