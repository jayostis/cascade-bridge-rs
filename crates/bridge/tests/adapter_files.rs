mod common;

use cascade_bridge::{load_adapter, prepare};
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
