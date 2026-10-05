use crate::error::{ErrorKind, Map};
use crate::fixtures::{tiny, tiny_with_vocabularies, with_accounting, ACCOUNTING, CRATE};
use crate::library::Loaded;
use crate::resolver::Resolver;

fn refusal(read: crate::Result<Vec<u8>>) -> String {
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
        let refused = resolver.read(&iri).expect_err("outside the adapter");
        assert_eq!(refused.kind(), &ErrorKind::Adapter, "{iri}");
        assert!(
            refused.to_string().contains("not inside the adapter"),
            "{iri} was readable from outside the adapter: {refused}"
        );
    }

    let elsewhere = "file://elsewhere/share/adapter/ro-crate-metadata.json";
    let refused = refusal(resolver.read(elsewhere));
    assert!(refused.contains("not inside the adapter"), "{refused:?}");
}

#[test]
fn reads_each_map_for_what_it_holds_and_refuses_what_is_in_neither() {
    let resolver = tiny_with_vocabularies();
    let vocabularies = resolver
        .vocabularies()
        .expect("the vocabulary given")
        .to_owned();
    let shapes = format!("{vocabularies}ontologies/catalog/v1/catalog.shapes.ttl");
    let crate_iri = format!("{}ro-crate-metadata.json", resolver.root());

    resolver
        .read(&crate_iri)
        .expect("the crate, inside the adapter");
    resolver
        .read_vocabulary(&shapes)
        .expect("the shapes, inside the vocabulary");
    let refused = refusal(resolver.read(&shapes));
    assert!(
        refused.contains("not inside the adapter"),
        "{shapes} was read as a file of the crate: {refused:?}"
    );
    let refused = resolver
        .read_vocabulary(&crate_iri)
        .expect_err("the crate is no file of the vocabulary");
    assert_eq!(refused.kind(), &ErrorKind::Vocabulary);
    assert!(
        refused.to_string().contains("not inside the vocabulary"),
        "{crate_iri} was read as a file of the vocabulary: {refused}"
    );

    for escape in ["../boundary.rs", "%2e%2e/boundary.rs"] {
        let iri = format!("{vocabularies}{escape}");
        let refused = refusal(resolver.read_vocabulary(&iri));
        assert!(
            refused.contains("not inside the vocabulary"),
            "{iri} was readable from outside the vocabulary: {refused:?}"
        );
    }
    let refused = refusal(resolver.read(&format!("{}../harness.rs", resolver.root())));
    assert!(refused.contains("not inside the adapter"), "{refused:?}");
}

#[test]
fn names_the_map_and_the_path_of_a_file_inside_it_that_it_lacks() {
    let resolver = tiny_with_vocabularies();
    let missing = resolver
        .read(&format!("{}mapping/absent.rq#fragment", resolver.root()))
        .expect_err("no such file");
    assert_eq!(
        missing.kind(),
        &ErrorKind::Missing {
            map: Map::Adapter,
            path: "mapping/absent.rq".to_owned()
        }
    );
    let vocabularies = resolver.vocabularies().expect("the vocabulary given");
    let missing = resolver
        .read_vocabulary(&format!("{vocabularies}ontologies/absent.ttl"))
        .expect_err("no such file");
    assert_eq!(
        missing.kind(),
        &ErrorKind::Missing {
            map: Map::Vocabulary,
            path: "ontologies/absent.ttl".to_owned()
        }
    );
}

#[test]
fn reads_no_vocabulary_where_none_was_given() {
    let resolver = tiny();
    let refused =
        refusal(resolver.read_vocabulary(&format!("{}ro-crate-metadata.json", resolver.root())));
    assert!(refused.contains("no vocabulary was given"), "{refused:?}");
}

#[test]
fn moves_a_loaded_adapter_to_the_thread_that_converts_with_it() {
    fn sendable<T: Send>() {}
    sendable::<Loaded>();
}

#[test]
fn serves_a_replaced_file_only_inside_the_adapter() {
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
        .expect_err("no vocabulary was given");
    assert!(
        refused.to_string().contains("no vocabulary was given"),
        "{refused}"
    );
}

#[test]
fn reads_a_path_with_its_dot_segments_removed() {
    let resolver = tiny();
    let root = resolver.root().to_owned();
    let judged = resolver.read(&format!("{root}{CRATE}")).expect("the crate");
    for dotted in [
        format!("{root}missing/../{CRATE}"),
        format!("{root}./{CRATE}"),
        format!("{root}missing/%2e%2E/{CRATE}"),
    ] {
        let read = resolver
            .read(&dotted)
            .unwrap_or_else(|e| panic!("{dotted} is the crate, inside the adapter: {e}"));
        assert_eq!(read, judged, "{dotted}");
    }
}
