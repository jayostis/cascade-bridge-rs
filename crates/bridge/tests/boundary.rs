mod common;

use cascade_bridge::{Prepared, Resolver};
use common::{tiny, tiny_with_vocabularies, with_accounting, ACCOUNTING, CRATE};
use std::path::PathBuf;

const ALLOWED: &str = "resolver.rs";

fn source_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

#[test]
fn lets_only_the_resolver_name_the_filesystem() {
    let mut offenders = Vec::new();
    let mut stack = vec![source_dir()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).expect("read src") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            if path.file_name().is_some_and(|n| n == ALLOWED) {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read module");
            for forbidden in ["std::fs", "std::path"] {
                if text.contains(forbidden) {
                    offenders.push(format!("{}: {forbidden}", path.display()));
                }
            }
        }
    }
    assert_eq!(offenders, Vec::<String>::new());
}

fn refusal(read: cascade_bridge::Result<Vec<u8>>) -> String {
    read.err()
        .map(|error| error.to_string())
        .unwrap_or_default()
}

#[test]
fn refuses_a_path_that_leaves_the_adapter_however_it_is_spelled() {
    let resolver = tiny();
    let root = resolver.root().to_owned();

    resolver
        .read(&format!("{root}ro-crate-metadata.json"))
        .expect("the crate, inside the adapter");

    for escape in ["../harness.rs", "%2e%2e/harness.rs", "%2E%2E/boundary.rs"] {
        let iri = format!("{root}{escape}");
        let refused = refusal(resolver.read(&iri));
        assert!(
            refused.contains("not inside the adapter"),
            "{iri} was readable from outside the adapter: {refused:?}"
        );
    }

    let elsewhere = "file://elsewhere/share/adapter/ro-crate-metadata.json";
    let refused = refusal(resolver.read(elsewhere));
    assert!(refused.contains("not inside the adapter"), "{refused:?}");
}

#[test]
fn reads_each_directory_for_what_it_holds_and_refuses_what_is_in_neither() {
    let resolver = tiny_with_vocabularies();
    let vocabularies = resolver
        .vocabularies()
        .expect("the checkout the command named")
        .to_owned();
    let shapes = format!("{vocabularies}ontologies/catalog/v1/catalog.shapes.ttl");
    let crate_iri = format!("{}ro-crate-metadata.json", resolver.root());

    resolver
        .read(&crate_iri)
        .expect("the crate, inside the adapter");
    resolver
        .read_vocabulary(&shapes)
        .expect("the shapes, inside the vocabularies");
    let refused = refusal(resolver.read(&shapes));
    assert!(
        refused.contains("not inside the adapter"),
        "{shapes} was read as a file of the crate: {refused:?}"
    );
    let refused = refusal(resolver.read_vocabulary(&crate_iri));
    assert!(
        refused.contains("not inside the vocabularies"),
        "{crate_iri} was read as a file of the vocabulary: {refused:?}"
    );

    for escape in ["../boundary.rs", "%2e%2e/boundary.rs"] {
        let iri = format!("{vocabularies}{escape}");
        let refused = refusal(resolver.read_vocabulary(&iri));
        assert!(
            refused.contains("not inside the vocabularies"),
            "{iri} was readable from outside the vocabularies directory: {refused:?}"
        );
    }
    let refused = refusal(resolver.read(&format!("{}../harness.rs", resolver.root())));
    assert!(refused.contains("not inside the adapter"), "{refused:?}");
}

#[test]
fn reads_no_vocabulary_where_the_command_named_no_checkout() {
    let resolver = tiny();
    let refused =
        refusal(resolver.read_vocabulary(&format!("{}ro-crate-metadata.json", resolver.root())));
    assert!(refused.contains("no vocabularies"), "{refused:?}");
}

#[test]
fn moves_a_prepared_adapter_to_the_thread_that_converts_with_it() {
    fn sendable<T: Send>() {}
    sendable::<Prepared>();
}

#[test]
fn serves_a_replaced_file_only_where_the_adapter_would_read_it() {
    let replaced = with_accounting("# replaced\n");
    let inside = format!("{}{ACCOUNTING}", replaced.root());
    assert_eq!(replaced.read(&inside).expect("inside"), b"# replaced\n");

    let outside = format!("{}../{ACCOUNTING}", replaced.root());
    let refused = replaced.read(&outside).expect_err("outside the adapter");
    assert!(
        refused.to_string().contains("not inside the adapter"),
        "{refused}"
    );

    let refused = replaced
        .read_vocabulary(&inside)
        .expect_err("no vocabularies were named");
    assert!(
        refused.to_string().contains("named no vocabularies"),
        "{refused}"
    );
}

#[test]
fn reads_the_file_the_boundary_was_judged_on() {
    let resolver = tiny();
    let root = resolver.root().to_owned();
    let judged = resolver.read(&format!("{root}{CRATE}")).expect("the crate");
    let read = resolver
        .read(&format!("{root}missing/../{CRATE}"))
        .expect("judged to be the crate, inside the adapter");
    assert_eq!(read, judged);
}
