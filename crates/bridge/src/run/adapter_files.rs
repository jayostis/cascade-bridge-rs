use crate::fixtures::{
    accounting, entry, tiny, with_accounting, Variant, ACCOUNTING, CRATE, GAP_SCHEME,
};
use crate::harness::{run_manifest, RunOptions};
use crate::load::load_adapter;
use crate::resolver::Resolver;
use crate::run::prepare;

const MANIFEST: &str = "fixtures/manifest.ttl";

fn refusal(variant: &Variant) -> String {
    let adapter = load_adapter(variant).expect("the crate loads");
    prepare(&adapter, variant)
        .err()
        .expect("the adapter is refused")
        .to_string()
}

fn names_once(refused: &str, iri: &str) {
    assert!(refused.starts_with(&format!("{iri}: ")), "{refused}");
    assert_eq!(refused.matches(iri).count(), 1, "{refused}");
}

mod tests {
    use super::*;

    /// Loading an adapter from maps of files · load(), Named, Files, Loaded
    mod loading_an_adapter_from_maps_of_files {
        use super::*;

        mod given_a_file_the_adapter_names_and_its_map_lacks {
            use super::*;

            /// refuses by its name a mapping the adapter's metadata names and the adapter lacks
            #[test]
            fn refuses_by_its_name_a_mapping_the_adapter_s_metadata_names_and_the_adapter_lacks() {
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
            fn names_once_a_file_the_metadata_or_its_accounting_names_and_the_adapter_lacks() {
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
        }

        mod given_a_file_of_the_adapter_that_does_not_parse {
            use super::*;

            #[test]
            fn refuses_by_its_name_a_schema_that_is_not_well_formed() {
                let variant = Variant::of(tiny()).with(
                    "schema/item.xsd",
                    r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element></xs:schema>"#,
                );
                names_once(
                    &refusal(&variant),
                    &format!("{}schema/item.xsd", variant.root()),
                );
            }

            /// refuses by its name a manifest that is not Turtle
            #[test]
            fn refuses_by_its_name_a_manifest_that_is_not_turtle() {
                let variant = Variant::of(tiny()).replacing_exactly(
                    MANIFEST,
                    "<> a mf:Manifest ;",
                    "<> a mf:Manifest ; {",
                    1,
                );
                let adapter = load_adapter(&variant).expect("the crate, which names the manifest");
                let refused = run_manifest(&adapter, &variant, RunOptions::default())
                    .err()
                    .expect("a manifest that is not Turtle is refused")
                    .to_string();
                names_once(&refused, &format!("{}{MANIFEST}", variant.root()));
            }

            /// refuses by its name a table that is not Turtle
            #[test]
            fn refuses_by_its_name_a_table_that_is_not_turtle() {
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
        }
    }

    /// Running an adapter's test manifest · test(), TestOptions, TestReport
    mod running_an_adapter_s_test_manifest {
        use super::*;

        mod given_a_manifest_that_cannot_be_read {
            use super::*;

            /// reports by its name an expected graph that is not Turtle
            #[test]
            fn reports_by_its_name_an_expected_graph_that_is_not_turtle() {
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
        }
    }
}
