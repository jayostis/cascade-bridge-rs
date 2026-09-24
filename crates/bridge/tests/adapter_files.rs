mod common;

use cascade_bridge::{load_adapter, prepare, run_manifest, Resolver, RunOptions};
use common::{accounting, entry, tiny, with_accounting, Variant, ACCOUNTING, CRATE, GAP_SCHEME};

const MANIFEST: &str = "fixtures/manifest.ttl";

fn refusal(variant: &Variant) -> String {
    let adapter = load_adapter(variant).expect("the crate loads");
    prepare(&adapter, variant)
        .err()
        .expect("the adapter is refused")
        .to_string()
}

/// A refusal that opens with the file's name and names it nowhere else.
fn names_once(refused: &str, iri: &str) {
    assert!(refused.starts_with(&format!("{iri}: ")), "{refused}");
    assert_eq!(refused.matches(iri).count(), 1, "{refused}");
}

#[test]
fn a_mapping_the_crate_names_and_the_directory_lacks_is_refused_by_its_name() {
    let variant = Variant::of(tiny()).replacing_exactly(
        CRATE,
        "mapping/item-tag.rq",
        "mapping/missing.rq",
        1,
    );
    names_once(
        &refusal(&variant),
        &format!("{}mapping/missing.rq", variant.root()),
    );
}

#[test]
fn a_file_the_crate_or_its_accounting_names_and_the_directory_lacks_is_named_once() {
    let root = tiny().root().to_owned();
    let accounting_iri = format!("{root}{ACCOUNTING}");
    let lookup = accounting(&[entry(
        "/item/note",
        "consumed",
        &[
            "bridge:lookupIn <missing-statuses.ttl>",
            "bridge:lookupNamesGap ex:statusOutsideTheTable",
        ],
    )]);
    let cases = [
        (
            Variant::of(tiny()).replacing_exactly(
                CRATE,
                ACCOUNTING,
                "vocab/missing-accounting.ttl",
                2,
            ),
            String::new(),
            format!("{root}vocab/missing-accounting.ttl"),
        ),
        (
            Variant::of(tiny()).replacing_exactly(CRATE, GAP_SCHEME, "vocab/missing-gaps.ttl", 1),
            String::new(),
            format!("{root}vocab/missing-gaps.ttl"),
        ),
        (
            with_accounting(&lookup),
            format!("{accounting_iri}: the entry for /item/note looks its values up in "),
            format!("{root}vocab/missing-statuses.ttl"),
        ),
    ];
    for (variant, context, missing) in cases {
        let refused = refusal(&variant);
        let named = refused
            .strip_prefix(&context)
            .unwrap_or_else(|| panic!("{refused}"));
        names_once(named, &missing);
    }
}

#[test]
fn a_schema_that_is_not_well_formed_is_refused_by_its_name() {
    let variant = Variant::of(tiny()).with(
        "schema/item.xsd",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element></xs:schema>"#,
    );
    names_once(
        &refusal(&variant),
        &format!("{}schema/item.xsd", variant.root()),
    );
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
    names_once(&refused, &format!("{}{MANIFEST}", variant.root()));
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
    names_once(
        &refusal(&variant),
        &format!("{}vocab/broken-table.ttl", variant.root()),
    );
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
