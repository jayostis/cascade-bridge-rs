use std::fs;
use std::path::Path;

pub(crate) fn fixture(path: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

pub(crate) fn fixture_names(directory: &str) -> Vec<String> {
    fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(directory))
        .expect("the fixtures")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}
