mod common;

use cascade_bridge_cli::{file_iri, file_iri_to_path, path_to_file_iri};
use common::scratch;
use std::path::{Path, PathBuf};

/// Naming a file by its IRI · file_iri(), path_to_file_iri(), file_iri_to_path()
///
/// The command names an adapter, a document and its facts by `file:` IRIs, and a finding names its
/// document by one. Both hosts must write a path's IRI alike, as Node writes it for a real path, or
/// a findings file written on one host names a document the other cannot find.
mod naming_a_file_by_its_iri {
    use super::*;

    /// `path_to_file_iri(path)`
    mod path_to_file_iri {
        use super::*;

        /// writes a drive path under the empty authority, with Windows' `\\?\` prefix or without
        #[test]
        fn writes_a_drive_path_under_the_empty_authority_with_the_prefix_or_without() {
            assert_eq!(
                path_to_file_iri(Path::new(r"\\?\C:\dev\adapter")),
                "file:///C:/dev/adapter"
            );
            assert_eq!(
                path_to_file_iri(Path::new(r"C:\dev\adapter")),
                "file:///C:/dev/adapter"
            );
        }

        /// writes a network path with its server as the authority, with the `\\?\UNC\` prefix or without
        #[test]
        fn writes_a_network_path_with_its_server_as_the_authority_with_the_prefix_or_without() {
            assert_eq!(
                path_to_file_iri(Path::new(r"\\?\UNC\server\share\adapter")),
                "file://server/share/adapter"
            );
            assert_eq!(
                path_to_file_iri(Path::new(r"\\server\share\adapter")),
                "file://server/share/adapter"
            );
        }

        /// writes a space and a non-ASCII letter percent-encoded
        #[test]
        fn writes_a_space_and_a_non_ascii_letter_percent_encoded() {
            assert_eq!(
                path_to_file_iri(Path::new("/tmp/une données/adapter")),
                "file:///tmp/une%20donn%C3%A9es/adapter"
            );
        }
    }

    /// `file_iri_to_path(iri)`
    mod file_iri_to_path {
        use super::*;

        /// returns the drive path of an IRI under the empty authority
        #[test]
        fn returns_the_drive_path_of_an_iri_under_the_empty_authority() {
            assert_eq!(
                file_iri_to_path("file:///C:/dev/adapter/x.json"),
                Some(PathBuf::from("C:/dev/adapter/x.json"))
            );
        }

        /// returns the network path of an IRI that names a server
        #[test]
        fn returns_the_network_path_of_an_iri_that_names_a_server() {
            assert_eq!(
                file_iri_to_path("file://server/share/adapter/ro-crate-metadata.json"),
                Some(PathBuf::from(
                    "//server/share/adapter/ro-crate-metadata.json"
                ))
            );
        }

        /// returns a path that `path_to_file_iri` writes back to the same IRI
        #[test]
        fn returns_a_path_that_path_to_file_iri_writes_back_to_the_same_iri() {
            for iri in [
                "file://server/share/adapter",
                "file:///tmp/une%20donn%C3%A9es/adapter",
            ] {
                let path = file_iri_to_path(iri).expect("a file IRI");
                assert_eq!(path_to_file_iri(&path), iri);
            }
        }

        /// returns `None` for an IRI that is no `file:` IRI
        #[test]
        fn returns_none_for_an_iri_that_is_no_file_iri() {
            assert_eq!(file_iri_to_path("https://example.org/a"), None);
        }
    }

    /// `file_iri(path)`
    mod file_iri {
        use super::*;

        #[test]
        fn returns_an_error_for_a_path_that_reaches_nothing() {
            let scratch = scratch();
            assert!(file_iri(scratch.path().join("no-such-file")).is_err());
        }
    }
}
