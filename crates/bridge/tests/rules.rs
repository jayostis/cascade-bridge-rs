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

fn quoted<'a>(table: &'a str, key: &str) -> Option<&'a str> {
    let (_, rest) = table.split_once(&format!("{key} = \""))?;
    rest.split_once('"').map(|(value, _)| value)
}

fn exact_version(requirement: &str) -> bool {
    let Some(version) = requirement.strip_prefix('=') else {
        return false;
    };
    let core = version.split(['-', '+']).next().unwrap_or_default();
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn full_rev(rev: &str) -> bool {
    rev.len() == 40 && rev.bytes().all(|b| b.is_ascii_hexdigit())
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
                quoted(requirement, "rev").is_some_and(full_rev)
            } else {
                quoted(requirement, "version").is_some_and(exact_version)
                    || requirement.contains("workspace = true")
            }
        } else {
            requirement
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .is_some_and(exact_version)
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
        for entry in std::fs::read_dir(&directory).expect("read directory") {
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

fn files_naming(name: &str) -> Vec<String> {
    let workspace = workspace();
    let mut naming: Vec<String> = files_under(&workspace.join("crates"))
        .into_iter()
        .filter(|file| {
            String::from_utf8_lossy(&std::fs::read(file).expect("read file")).contains(name)
        })
        .map(|file| {
            file.strip_prefix(&workspace)
                .expect("under the workspace")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    naming.sort();
    naming
}

#[test]
fn meets_a_real_adapter_only_through_the_compatibility_run() {
    let adapters = must_pass_with();
    assert!(!adapters.is_empty(), "compatibility.json names no adapter");
    let offenders: Vec<(String, String)> = adapters
        .iter()
        .flat_map(|adapter| {
            files_naming(adapter)
                .into_iter()
                .map(move |file| (file, adapter.clone()))
        })
        .collect();
    assert_eq!(offenders, Vec::<(String, String)>::new());
}

#[test]
fn meets_the_specification_s_synthetic_adapter_only_through_its_vector() {
    let offenders: Vec<String> = files_naming("fixtures/synthetic-adapter")
        .into_iter()
        .filter(|file| file != "crates/bridge-cli/tests/specification_vector.rs" && file != file!())
        .collect();
    assert_eq!(offenders, Vec::<String>::new());
}
