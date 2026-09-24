mod common;

use cascade_bridge::{load_adapter, prepare, DirectoryResolver, Resolver, Result};
use common::tiny;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn members() -> Vec<PathBuf> {
    let mut members: Vec<PathBuf> = std::fs::read_dir(workspace().join("crates"))
        .expect("read crates")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();
    members.sort();
    members
}

fn unpinned(manifest: &str) -> Vec<String> {
    let mut offenders = Vec::new();
    let mut listing = false;
    for line in manifest.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            listing = line.ends_with("dependencies]");
            if line.contains("dependencies.") {
                offenders.push(line.to_owned());
            }
            continue;
        }
        if !listing {
            continue;
        }
        let Some((_, requirement)) = line.split_once('=') else {
            offenders.push(line.to_owned());
            continue;
        };
        let requirement = requirement.trim();
        let pinned = if requirement.starts_with('{') {
            if requirement.contains("git =") {
                requirement.contains("rev = \"")
            } else {
                requirement.contains("version = \"=") || requirement.contains("workspace = true")
            }
        } else {
            requirement.starts_with("\"=")
        };
        if !pinned {
            offenders.push(line.to_owned());
        }
    }
    offenders
}

#[test]
fn pins_every_dependency_to_an_exact_version_or_a_git_rev() {
    let manifests = std::iter::once(workspace())
        .chain(members())
        .map(|directory| directory.join("Cargo.toml"));
    let mut offenders = Vec::new();
    for manifest in manifests {
        let text = std::fs::read_to_string(&manifest).expect("read Cargo.toml");
        for line in unpinned(&text) {
            offenders.push(format!("{}: {line}", manifest.display()));
        }
    }
    assert_eq!(offenders, Vec::<String>::new());
}

struct Counting {
    directory: DirectoryResolver,
    reads: RefCell<BTreeMap<String, usize>>,
}

impl Counting {
    fn count(&self, iri: &str) {
        *self.reads.borrow_mut().entry(iri.to_owned()).or_default() += 1;
    }
}

impl Resolver for Counting {
    fn root(&self) -> &str {
        self.directory.root()
    }

    fn vocabularies(&self) -> Option<&str> {
        self.directory.vocabularies()
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        self.count(iri);
        self.directory.read(iri)
    }

    fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
        self.count(iri);
        self.directory.read_vocabulary(iri)
    }
}

#[test]
fn reads_each_query_of_the_adapter_once_per_prepare() {
    let adapter = load_adapter(&tiny()).expect("adapter");
    let counting = Counting {
        directory: tiny(),
        reads: RefCell::default(),
    };
    prepare(&adapter, &counting).expect("prepared");
    let reads = counting.reads.into_inner();
    let queries: Vec<&String> = adapter
        .mappings
        .iter()
        .chain(&adapter.findings_queries)
        .chain(&adapter.detect_query)
        .collect();
    assert!(!adapter.findings_queries.is_empty() && adapter.detect_query.is_some());
    let misread: Vec<(&String, usize)> = queries
        .into_iter()
        .map(|query| (query, reads.get(query).copied().unwrap_or_default()))
        .filter(|(_, times)| *times != 1)
        .collect();
    assert_eq!(misread, Vec::<(&String, usize)>::new());
}

fn must_pass_with() -> Vec<String> {
    let text =
        std::fs::read_to_string(workspace().join("compatibility.json")).expect("compatibility");
    let listed = text
        .split_once("\"mustPassWith\"")
        .and_then(|(_, rest)| rest.split_once('['))
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(listed, _)| listed)
        .expect("compatibility.json lists mustPassWith");
    listed
        .split('"')
        .skip(1)
        .step_by(2)
        .map(|iri| iri.trim_end_matches('/').rsplit('/').next().unwrap_or(iri))
        .map(str::to_owned)
        .collect()
}

fn files_under(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![directory.to_owned()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).expect("read tests") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn meets_a_real_adapter_only_through_the_compatibility_run() {
    let adapters = must_pass_with();
    assert!(!adapters.is_empty(), "compatibility.json names no adapter");
    let mut offenders = Vec::new();
    for tests in members().iter().map(|member| member.join("tests")) {
        if !tests.is_dir() {
            continue;
        }
        for file in files_under(&tests) {
            let text =
                String::from_utf8_lossy(&std::fs::read(&file).expect("read file")).into_owned();
            for adapter in &adapters {
                if text.contains(adapter.as_str()) {
                    offenders.push(format!("{}: {adapter}", file.display()));
                }
            }
        }
    }
    assert_eq!(offenders, Vec::<String>::new());
}
