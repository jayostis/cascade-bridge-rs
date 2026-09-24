// The node host runs the library built for wasm32-unknown-unknown and takes
// the native command's arguments, so every case here runs one command twice,
// once on each host, and holds the node host to what the native one wrote.
//
// Graphs are compared as graphs: spargebra names each aggregate at random per
// parse, so two runs of one host already differ in their bytes.
mod common;

use common::{
    canonical, copied_to, names, read_at_its_own_iri, scratch, tiny, vocabularies, BASE, MAX_LENGTH,
};
use oxrdf::{NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfParser};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const EARL: &str = "http://www.w3.org/ns/earl#";
const DCT_TITLE: &str = "http://purl.org/dc/terms/title";

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn node_host_directory() -> PathBuf {
    workspace().join("hosts/node")
}

/// The module the node host loads is built by a step of its own, as the
/// compatibility tooling builds it, and never by `cargo test`.
fn require_the_module() {
    let module = node_host_directory().join("pkg/cascade_bridge_wasm.js");
    assert!(
        module.is_file(),
        "no node host module at {}: run `sh hosts/node/setup.sh` first",
        module.display()
    );
}

fn native(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cascade-bridge"))
        .args(arguments)
        .current_dir(workspace())
        .output()
        .expect("run the native command")
}

fn node(arguments: &[&str]) -> Output {
    require_the_module();
    Command::new("node")
        .arg(node_host_directory().join("cascade-bridge.mjs"))
        .args(arguments)
        .current_dir(workspace())
        .output()
        .expect("run node")
}

fn graph(path: &Path) -> BTreeSet<String> {
    canonical(
        &std::fs::read(path).expect("the written graph"),
        RdfFormat::Turtle,
        BASE,
    )
}

/// The node host's findings, once they have been shown to be the native
/// command's. A findings file names its document relative to itself, so each
/// is read against its own IRI.
fn converts_as_the_native_command_does(
    adapter: &Path,
    document: &str,
    extra: &[&str],
) -> Vec<Quad> {
    let scratch = scratch();
    let document = adapter.join("fixtures/in").join(document);
    let adapter = adapter.to_string_lossy().into_owned();
    let document = document.to_string_lossy().into_owned();
    let mut written = BTreeMap::new();
    for host in ["native", "node"] {
        let out = scratch.path().join(format!("{host}-graph.ttl"));
        let found = scratch.path().join(format!("{host}-findings.ttl"));
        let mut arguments = vec![
            "convert".to_owned(),
            adapter.clone(),
            document.clone(),
            "--out".to_owned(),
            out.to_string_lossy().into_owned(),
            "--findings".to_owned(),
            found.to_string_lossy().into_owned(),
        ];
        arguments.extend(extra.iter().map(|a| (*a).to_owned()));
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let run = match host {
            "native" => native(&arguments),
            _ => node(&arguments),
        };
        assert_eq!(
            run.status.code(),
            Some(0),
            "{host}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        written.insert(
            host,
            (graph(&out), read_at_its_own_iri(&found, RdfFormat::Turtle)),
        );
    }
    let (native_graph, native_findings) = &written["native"];
    let (node_graph, node_findings) = &written["node"];
    assert!(!native_graph.is_empty(), "the native command wrote a graph");
    assert_eq!(node_graph, native_graph, "the graph --out names");
    assert_eq!(
        cascade_bridge::canonical_lines(node_findings.clone()).expect("one canonical graph"),
        cascade_bridge::canonical_lines(native_findings.clone()).expect("one canonical graph"),
        "the findings --findings names"
    );
    node_findings.clone()
}

#[test]
fn the_node_host_converts_a_document_to_the_graph_and_findings_the_native_command_writes() {
    converts_as_the_native_command_does(&tiny(), "two.xml", &[]);
}

#[test]
fn the_node_host_writes_the_findings_the_native_command_draws_from_the_vocabularies_shapes() {
    let vocabularies = vocabularies().to_string_lossy().into_owned();
    let found = converts_as_the_native_command_does(
        &tiny(),
        "output-fails-a-shape.xml",
        &["--vocabularies", &vocabularies],
    );
    assert!(
        names(&found, MAX_LENGTH),
        "the finding the checkout's shapes draw is among what both hosts wrote: {found:?}"
    );
}

#[test]
fn the_node_host_reads_a_directory_inside_the_adapter_whose_name_begins_with_two_dots() {
    let scratch = scratch();
    let adapter = scratch.path().join("dotted-adapter");
    copied_to(&tiny(), &adapter);
    std::fs::rename(adapter.join("mapping"), adapter.join("..mapping"))
        .expect("the renamed mapping");
    let metadata = adapter.join("ro-crate-metadata.json");
    let text = std::fs::read_to_string(&metadata).expect("the metadata");
    std::fs::write(&metadata, text.replace("\"mapping/", "\"..mapping/")).expect("the metadata");
    converts_as_the_native_command_does(&adapter, "two.xml", &[]);
}

/// A run whose standard output is closed before the graph is written to it.
fn convert_into_a_closed_pipe(mut command: Command) -> Output {
    let adapter = tiny().to_string_lossy().into_owned();
    let document = tiny().join("fixtures/in/two.xml");
    let mut child = command
        .args(["convert", &adapter, &document.to_string_lossy()])
        .current_dir(workspace())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the command");
    drop(child.stdout.take());
    child.wait_with_output().expect("the command's end")
}

#[test]
fn the_node_host_exits_as_the_native_command_does_when_standard_output_is_closed() {
    require_the_module();
    let native_run = convert_into_a_closed_pipe(Command::new(env!("CARGO_BIN_EXE_cascade-bridge")));
    let mut node = Command::new("node");
    node.arg(node_host_directory().join("cascade-bridge.mjs"));
    let node_run = convert_into_a_closed_pipe(node);
    let said = String::from_utf8_lossy(&node_run.stderr);
    assert_eq!(
        native_run.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&native_run.stderr)
    );
    assert_eq!(node_run.status.code(), native_run.status.code(), "{said}");
    assert!(said.contains("standard output"), "{said}");
}

/// Each entry's outcome, by the test's IRI or, for an entry with none, by the
/// title the report gives it.
fn outcomes(report: &Path) -> BTreeMap<String, String> {
    let quads: Vec<_> = RdfParser::from_format(RdfFormat::Turtle)
        .for_slice(&std::fs::read(report).expect("the written report"))
        .map(|quad| quad.expect("a parsed report"))
        .collect();
    let object = |subject: &NamedOrBlankNode, predicate: &str| -> Option<Term> {
        quads
            .iter()
            .find(|q| &q.subject == subject && q.predicate.as_str() == predicate)
            .map(|q| q.object.clone())
    };
    let as_subject = |term: Term| match term {
        Term::NamedNode(n) => Some(NamedOrBlankNode::from(n)),
        Term::BlankNode(b) => Some(NamedOrBlankNode::from(b)),
        _ => None,
    };
    quads
        .iter()
        .filter(|q| q.predicate.as_str() == format!("{EARL}test"))
        .map(|q| {
            let test = match &q.object {
                Term::NamedNode(n) => n.as_str().to_owned(),
                other => match object(&as_subject(other.clone()).expect("a node"), DCT_TITLE) {
                    Some(Term::Literal(title)) => title.value().to_owned(),
                    _ => panic!("an anonymous test with no title"),
                },
            };
            let result = object(&q.subject, &format!("{EARL}result"))
                .and_then(as_subject)
                .expect("a result");
            let outcome = match object(&result, &format!("{EARL}outcome")) {
                Some(Term::NamedNode(n)) => n.as_str().to_owned(),
                _ => panic!("a result with no outcome"),
            };
            (test, outcome)
        })
        .collect()
}

#[test]
fn the_node_host_reports_the_outcome_the_native_command_reports_for_each_entry() {
    let scratch = scratch();
    let adapter = tiny().to_string_lossy().into_owned();
    let vocabularies = vocabularies().to_string_lossy().into_owned();
    let native_report = scratch.path().join("native.earl.ttl");
    let node_report = scratch.path().join("node.earl.ttl");
    let native_run = native(&[
        "test",
        &adapter,
        "--vocabularies",
        &vocabularies,
        "--earl",
        &native_report.to_string_lossy(),
    ]);
    let node_run = node(&[
        "test",
        &adapter,
        "--vocabularies",
        &vocabularies,
        "--earl",
        &node_report.to_string_lossy(),
    ]);
    assert_eq!(
        node_run.status.code(),
        native_run.status.code(),
        "{}",
        String::from_utf8_lossy(&node_run.stderr)
    );
    let expected = outcomes(&native_report);
    assert!(!expected.is_empty(), "the native command reported entries");
    assert_eq!(outcomes(&node_report), expected);
}

fn check_imports(allowlist: &Path) -> Output {
    require_the_module();
    Command::new("node")
        .arg(node_host_directory().join("check-imports.mjs"))
        .arg(allowlist)
        .current_dir(workspace())
        .output()
        .expect("run node")
}

fn allowlist() -> PathBuf {
    node_host_directory().join("imports.txt")
}

#[test]
fn the_import_check_passes_on_the_module_the_node_host_loads() {
    let run = check_imports(&allowlist());
    assert!(
        run.status.success(),
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn the_import_check_names_the_import_an_allowlist_one_entry_short_leaves_out() {
    let text = std::fs::read_to_string(allowlist()).expect("the allowlist");
    let mut entries: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let left_out = entries.pop();
    assert!(
        left_out.is_some(),
        "the allowlist names the module's imports"
    );
    let left_out = left_out.unwrap_or_default();
    let scratch = scratch();
    let short = scratch.path().join("imports-one-short.txt");
    std::fs::write(&short, entries.join("\n")).expect("the short allowlist");
    let run = check_imports(&short);
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!run.status.success(), "{said}");
    assert!(said.contains(left_out), "{left_out} is not named: {said}");
}
