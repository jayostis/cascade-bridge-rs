use cascade_bridge_cli::{file_iri_to_path, path_to_file_iri};
use std::path::{Path, PathBuf};

#[test]
fn writes_a_drive_path_under_the_empty_authority() {
    assert_eq!(
        path_to_file_iri(Path::new(r"\\?\C:\dev\adapter")),
        "file:///C:/dev/adapter"
    );
    assert_eq!(
        file_iri_to_path("file:///C:/dev/adapter/x.json"),
        Some(PathBuf::from("C:/dev/adapter/x.json"))
    );
}

#[test]
fn writes_a_network_path_with_its_server_as_the_authority() {
    assert_eq!(
        path_to_file_iri(Path::new(r"\\?\UNC\server\share\adapter")),
        "file://server/share/adapter"
    );
}

#[test]
fn writes_a_drive_path_as_node_s_realpath_spells_it_under_the_empty_authority() {
    assert_eq!(
        path_to_file_iri(Path::new(r"C:\dev\adapter")),
        "file:///C:/dev/adapter"
    );
}

#[test]
fn writes_a_network_path_as_node_s_realpath_spells_it_with_its_server_as_the_authority() {
    let iri = path_to_file_iri(Path::new(r"\\server\share\adapter"));
    assert_eq!(iri, "file://server/share/adapter");
    let path = file_iri_to_path(&iri).expect("a file IRI");
    assert_eq!(path, PathBuf::from("//server/share/adapter"));
    assert_eq!(path_to_file_iri(&path), iri);
}

#[test]
fn reads_a_network_iri_back_to_the_network_path() {
    assert_eq!(
        file_iri_to_path("file://server/share/adapter/ro-crate-metadata.json"),
        Some(PathBuf::from("//server/share/adapter/ro-crate-metadata.json"))
    );
}
