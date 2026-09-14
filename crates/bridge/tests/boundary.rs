// Two boundaries, each of which is invisible until something crosses it.
//
// The filesystem is one module's business. The library must run wherever a
// host can hand it bytes, and std::fs named anywhere but the resolver makes
// that false for every host without a filesystem, which only the first one to
// try would discover.
//
// The adapter's directory is the other. A mapping's IRI comes out of a
// stranger's crate, so the boundary is decided on the path a filesystem would
// actually reach, never on how the IRI happens to be spelled.
use cascade_bridge::{DirectoryResolver, Resolver};
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

fn tiny() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tiny-adapter")
}

#[test]
fn refuses_a_path_that_leaves_the_adapter_however_it_is_spelled() {
    let resolver = DirectoryResolver::new(tiny()).expect("resolver");
    let root = resolver.root().to_owned();

    // Inside: the crate itself is readable.
    assert!(resolver
        .read(&format!("{root}ro-crate-metadata.json"))
        .is_ok());

    // Outside, spelled plainly and spelled in percent-encoded dot segments.
    // The second is what a prefix test on the IRI string lets through.
    for escape in ["../harness.rs", "%2e%2e/harness.rs", "%2E%2E/boundary.rs"] {
        let iri = format!("{root}{escape}");
        let refused = resolver.read(&iri);
        assert!(
            refused.is_err(),
            "{iri} was readable from outside the adapter"
        );
        assert!(refused
            .unwrap_err()
            .to_string()
            .contains("not inside the adapter"));
    }
}
