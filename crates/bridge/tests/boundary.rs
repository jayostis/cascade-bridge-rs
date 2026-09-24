// Three boundaries, each of which is invisible until something crosses it.
//
// The filesystem is one module's business. The library must run wherever a
// host can hand it bytes, and std::fs named anywhere but the resolver makes
// that false for every host without a filesystem, which only the first one to
// try would discover.
//
// The adapter's directory is the second. A mapping's IRI comes out of a
// stranger's crate, so the boundary is decided on the path a filesystem would
// actually reach, never on how the IRI happens to be spelled.
//
// The thread is the third. An adapter is prepared once and converted with many
// times, so a host prepares on one thread and hands the result to the worker
// that converts. An auto trait is granted by every field at once and withdrawn
// by any one of them, with no line to read it off and no caller in this
// repository to miss it.
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

/// What refusing this read said, where it was refused.
fn refusal(read: cascade_bridge::Result<Vec<u8>>) -> String {
    read.err()
        .map(|error| error.to_string())
        .unwrap_or_default()
}

#[test]
fn refuses_a_path_that_leaves_the_adapter_however_it_is_spelled() {
    let resolver = tiny();
    let root = resolver.root().to_owned();

    // Inside: the crate itself is readable.
    resolver
        .read(&format!("{root}ro-crate-metadata.json"))
        .expect("the crate, inside the adapter");

    // Outside, spelled plainly and spelled in percent-encoded dot segments.
    // The second is what a prefix test on the IRI string lets through.
    for escape in ["../harness.rs", "%2e%2e/harness.rs", "%2E%2E/boundary.rs"] {
        let iri = format!("{root}{escape}");
        let refused = refusal(resolver.read(&iri));
        assert!(
            refused.contains("not inside the adapter"),
            "{iri} was readable from outside the adapter: {refused:?}"
        );
    }

    // Another server is outside however its path is spelled.
    let elsewhere = "file://elsewhere/share/adapter/ro-crate-metadata.json";
    let refused = refusal(resolver.read(elsewhere));
    assert!(refused.contains("not inside the adapter"), "{refused:?}");
}

/// The checkout of `the-cascade-protocol/spec` the engine command was given is
/// the second directory a run may read, and the last. An adapter's crate names
/// a file in it by a path of a stranger's writing, so the same boundary is
/// decided the same way there. Neither directory is the other's: an ontology
/// the adapter wrote itself is no ontology at the vocabulary pin, and a file of
/// the checkout is no file of the crate.
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

/// A run given no checkout reads no vocabulary, and says so rather than
/// reaching for a file of the adapter.
#[test]
fn reads_no_vocabulary_where_the_command_named_no_checkout() {
    let resolver = tiny();
    let refused =
        refusal(resolver.read_vocabulary(&format!("{}ro-crate-metadata.json", resolver.root())));
    assert!(refused.contains("no vocabularies"), "{refused:?}");
}

/// Send and not Sync: a schema has carried a RefCell since long before this
/// test, so a prepared adapter has never been shared between threads, only
/// moved to one.
#[test]
fn moves_a_prepared_adapter_to_the_thread_that_converts_with_it() {
    fn sendable<T: Send>() {}
    sendable::<Prepared>();
}

/// A test's replaced file stands where the adapter's file would, and nowhere
/// else: a double serving it at any path ending in its name would let a test
/// pass while the Bridge read that file from outside the adapter.
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
