// Every case runs one command on each host. Graphs are compared as graphs: spargebra
// names each aggregate at random per parse, so two runs already differ in their bytes.
mod common;

use cascade_bridge::oxrdf::{NamedOrBlankNode, Quad, Term};
use cascade_bridge::oxrdfio::{RdfFormat, RdfParser};
use common::{
    canonical, canonical_lines, copied_to, names, read_at_its_own_iri, scratch, tiny, vocabularies,
    BASE, MAX_LENGTH,
};
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
        canonical_lines(node_findings.clone()),
        canonical_lines(native_findings.clone()),
        "the findings --findings names"
    );
    node_findings.clone()
}

#[test]
fn the_node_host_converts_a_document_to_the_graph_and_findings_the_native_command_writes() {
    let vocabularies = vocabularies().to_string_lossy().into_owned();
    converts_as_the_native_command_does(&tiny(), "two.xml", &["--vocabularies", &vocabularies]);
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
    let vocabularies = vocabularies().to_string_lossy().into_owned();
    converts_as_the_native_command_does(&adapter, "two.xml", &["--vocabularies", &vocabularies]);
}

fn replaced_once(adapter: &Path, path: &str, from: &str, to: &str) {
    let file = adapter.join(path);
    let text = std::fs::read_to_string(&file).expect("the file");
    assert_eq!(text.matches(from).count(), 1, "{path}");
    std::fs::write(&file, text.replace(from, to)).expect("the file");
}

#[test]
fn the_node_host_answers_an_import_of_the_xml_namespace_from_a_file_the_adapter_lacks_as_the_native_command_does(
) {
    let scratch = scratch();
    let adapter = scratch.path().join("adapter");
    copied_to(&tiny(), &adapter);
    assert!(!adapter.join("schema/xml.xsd").exists());
    replaced_once(
        &adapter,
        "schema/item.xsd",
        "<xs:element name=\"item\"",
        "<xs:import namespace=\"http://www.w3.org/XML/1998/namespace\" schemaLocation=\"xml.xsd\"/>\n  <xs:element name=\"item\"",
    );
    replaced_once(
        &adapter,
        "schema/item.xsd",
        "<xs:attribute name=\"internal\" type=\"xs:string\"/>",
        "<xs:attribute name=\"internal\" type=\"xs:string\"/>\n    <xs:attribute ref=\"xml:lang\"/>",
    );
    replaced_once(
        &adapter,
        "fixtures/in/two.xml",
        "<item id=\"1\">",
        "<item id=\"1\" xml:lang=\"en\">",
    );
    let vocabularies = vocabularies().to_string_lossy().into_owned();
    converts_as_the_native_command_does(&adapter, "two.xml", &["--vocabularies", &vocabularies]);
}

#[test]
fn the_node_host_refuses_a_mapping_holding_a_service_pattern_as_the_native_command_does() {
    let scratch = scratch();
    let adapter = scratch.path().join("service-adapter");
    copied_to(&tiny(), &adapter);
    let mapping = adapter.join("mapping/item.rq");
    let text = std::fs::read_to_string(&mapping).expect("the mapping");
    std::fs::write(
        &mapping,
        text.replacen(
            "WHERE {\n",
            "WHERE {\n  SERVICE <https://example.invalid/sparql> { ?there ?p ?o }\n",
            1,
        ),
    )
    .expect("the mapping");
    let adapter = adapter.to_string_lossy().into_owned();
    let document = tiny()
        .join("fixtures/in/two.xml")
        .to_string_lossy()
        .into_owned();
    let vocabularies = vocabularies().to_string_lossy().into_owned();
    let arguments = [
        "convert",
        &adapter,
        &document,
        "--vocabularies",
        &vocabularies,
    ];
    let native_run = native(&arguments);
    let node_run = node(&arguments);
    let said = String::from_utf8_lossy(&native_run.stderr);
    assert_eq!(native_run.status.code(), Some(2), "{said}");
    assert!(native_run.stdout.is_empty());
    assert!(
        said.contains("item.rq") && said.contains("SERVICE"),
        "the refusal names the query and the clause: {said}"
    );
    assert_eq!(
        node_run.status.code(),
        native_run.status.code(),
        "{}",
        String::from_utf8_lossy(&node_run.stderr)
    );
    assert!(node_run.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&node_run.stderr), said);
}

fn refuses_as_the_native_command_does_without_the_vocabularies(arguments: &[&str]) {
    let native_run = native(arguments);
    let node_run = node(arguments);
    let said = String::from_utf8_lossy(&native_run.stderr);
    assert_eq!(native_run.status.code(), Some(2), "{said}");
    assert!(native_run.stdout.is_empty());
    assert!(
        said.contains("bridge:vocabularyFile") && said.contains("--vocabularies"),
        "the refusal names what the crate names and the flag that reads it: {said}"
    );
    assert_eq!(
        node_run.status.code(),
        native_run.status.code(),
        "{}",
        String::from_utf8_lossy(&node_run.stderr)
    );
    assert!(node_run.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&node_run.stderr), said);
}

#[test]
fn the_node_host_refuses_to_test_without_the_vocabularies_as_the_native_command_does() {
    let adapter = tiny().to_string_lossy().into_owned();
    refuses_as_the_native_command_does_without_the_vocabularies(&["test", &adapter]);
}

#[test]
fn the_node_host_refuses_to_write_findings_without_the_vocabularies_as_the_native_command_does() {
    let scratch = scratch();
    let findings = scratch.path().join("findings.ttl");
    let adapter = tiny().to_string_lossy().into_owned();
    let document = tiny()
        .join("fixtures/in/two.xml")
        .to_string_lossy()
        .into_owned();
    refuses_as_the_native_command_does_without_the_vocabularies(&[
        "convert",
        &adapter,
        &document,
        "--findings",
        &findings.to_string_lossy(),
    ]);
    assert!(!findings.exists(), "a findings file was written");
}

#[test]
fn the_node_host_converts_without_the_vocabularies_and_says_so_as_the_native_command_does() {
    let adapter = tiny().to_string_lossy().into_owned();
    let document = tiny()
        .join("fixtures/in/two.xml")
        .to_string_lossy()
        .into_owned();
    let arguments = ["convert", &adapter, &document];
    let native_run = native(&arguments);
    let node_run = node(&arguments);
    let unvalidated = |run: &Output| -> Vec<String> {
        assert_eq!(
            run.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        String::from_utf8_lossy(&run.stderr)
            .lines()
            .filter(|line| line.contains("bridge:vocabularyFile"))
            .map(str::to_owned)
            .collect()
    };
    let said = unvalidated(&native_run);
    assert!(
        matches!(said.as_slice(), [line] if line.contains("--vocabularies")),
        "{said:?}"
    );
    assert_eq!(unvalidated(&node_run), said);
    let native_graph = canonical(&native_run.stdout, RdfFormat::Turtle, BASE);
    assert!(!native_graph.is_empty(), "the native command wrote a graph");
    assert_eq!(
        canonical(&node_run.stdout, RdfFormat::Turtle, BASE),
        native_graph
    );
}

#[test]
fn the_node_host_says_of_a_conversion_what_the_native_command_says() {
    let adapter = tiny().to_string_lossy().into_owned();
    let document = tiny()
        .join("fixtures/in/two.xml")
        .to_string_lossy()
        .into_owned();
    let arguments = ["convert", adapter.as_str(), document.as_str()];
    let native_run = native(&arguments);
    let node_run = node(&arguments);
    let said = |run: &Output| String::from_utf8_lossy(&run.stderr).into_owned();
    assert_eq!(native_run.status.code(), Some(0), "{}", said(&native_run));
    assert!(
        said(&native_run).contains("Document "),
        "{}",
        said(&native_run)
    );
    assert_eq!(said(&node_run), said(&native_run));
}

fn convert_into_a_closed_pipe(mut command: Command) -> Output {
    let adapter = tiny().to_string_lossy().into_owned();
    let vocabularies = vocabularies().to_string_lossy().into_owned();
    let document = tiny().join("fixtures/in/two.xml");
    let mut child = command
        .args([
            "convert",
            &adapter,
            &document.to_string_lossy(),
            "--vocabularies",
            &vocabularies,
        ])
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

#[test]
fn a_mapping_neither_host_can_read_is_named_once_by_each_and_both_exit_alike() {
    let scratch = scratch();
    for (case, named) in ["mapping/missing.rq", "mapping"].into_iter().enumerate() {
        let adapter = scratch.path().join(format!("adapter-{case}"));
        copied_to(&tiny(), &adapter);
        let metadata = adapter.join("ro-crate-metadata.json");
        let text = std::fs::read_to_string(&metadata).expect("the metadata");
        assert_eq!(text.matches("\"mapping/item-tag.rq\"").count(), 1);
        std::fs::write(
            &metadata,
            text.replace("\"mapping/item-tag.rq\"", &format!("\"{named}\"")),
        )
        .expect("the metadata");
        let iri = format!(
            "{}/{named}",
            cascade_bridge::file_iri(&adapter).expect("the adapter's IRI")
        );
        let document = adapter.join("fixtures/in/two.xml");
        let document = document.to_string_lossy().into_owned();
        let adapter = adapter.to_string_lossy().into_owned();
        let native_run = native(&["convert", &adapter, &document]);
        let node_run = node(&["convert", &adapter, &document]);
        assert_eq!(native_run.status.code(), Some(2));
        assert_eq!(node_run.status.code(), native_run.status.code());
        for run in [native_run, node_run] {
            let said = String::from_utf8_lossy(&run.stderr);
            assert!(said.contains(&format!("{iri}: ")), "{said}");
            assert_eq!(said.matches(&iri).count(), 1, "{said}");
        }
    }
}
