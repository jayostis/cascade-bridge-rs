// Every case runs one command on each host. Graphs are compared as graphs: spargebra
// names each aggregate at random per parse, so two runs already differ in their bytes.
mod common;

use cascade_bridge::oxrdf::Quad;
use cascade_bridge::oxrdfio::RdfFormat;
use common::{
    canonical, canonical_lines, copied_to, map_of, names, outcomes, read_at_its_own_iri, results,
    scratch, tiny, vocabularies, BASE, MAX_LENGTH,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn node_host_directory() -> PathBuf {
    workspace().join("hosts/node")
}

fn package_directory() -> PathBuf {
    workspace().join("package/dist")
}

fn require_the_module() {
    let module = package_directory().join("node.js");
    assert!(
        module.is_file(),
        "no package entry at {}: run `sh package/build.sh` first",
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

fn replaced_once(adapter: &Path, path: &str, from: &str, to: &str) {
    let file = adapter.join(path);
    let text = std::fs::read_to_string(&file).expect("the file");
    assert_eq!(text.matches(from).count(), 1, "{path}");
    std::fs::write(&file, text.replace(from, to)).expect("the file");
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

fn each_host_names_the_mapping_once(named: &str) {
    let scratch = scratch();
    let adapter = scratch.path().join("adapter");
    copied_to(&tiny(), &adapter);
    replaced_once(
        &adapter,
        "ro-crate-metadata.json",
        "\"mapping/item-tag.rq\"",
        &format!("\"{named}\""),
    );
    let iri = format!(
        "{}/{named}",
        cascade_bridge_cli::file_iri(&adapter).expect("the adapter's IRI")
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

fn check_imports(allowlist: &Path) -> Output {
    require_the_module();
    Command::new("node")
        .arg(workspace().join("package/check-imports.mjs"))
        .arg(allowlist)
        .arg(package_directory().join("cascade_bridge_bg.wasm"))
        .current_dir(workspace())
        .output()
        .expect("run node")
}

fn allowlist() -> PathBuf {
    workspace().join("package/imports.txt")
}

fn bench(arguments: &[&str]) -> Output {
    require_the_module();
    Command::new("node")
        .arg(node_host_directory().join("bench.mjs"))
        .args(arguments)
        .current_dir(workspace())
        .output()
        .expect("run the bench")
}

fn benched(documents: &[&Path], extra: &[&str]) -> BTreeMap<String, serde_json::Value> {
    let adapter = tiny();
    let mut given: Vec<&str> = vec![adapter.to_str().expect("a path")];
    given.extend(documents.iter().map(|path| path.to_str().expect("a path")));
    given.extend(extra);
    benched_with(&given)
}

fn benched_with(given: &[&str]) -> BTreeMap<String, serde_json::Value> {
    let scratch = scratch();
    let out = scratch.path().join("result.json");
    let vocabularies = vocabularies();
    let mut arguments: Vec<&str> = given.to_vec();
    arguments.extend([
        "--vocabularies",
        vocabularies.to_str().expect("a path"),
        "--allow-fallbacks",
        "--samples",
        "1",
        "--out",
        out.to_str().expect("a path"),
    ]);
    let run = bench(&arguments);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&out).expect("the result file")).expect("JSON");
    let mut file = BTreeMap::new();
    for entry in written["results"].as_array().expect("results") {
        file.insert(
            entry["name"].as_str().expect("a name").to_owned(),
            entry.clone(),
        );
    }
    file.insert("env".to_owned(), written["env"].clone());
    file.insert("commit".to_owned(), written["commit"].clone());
    file
}

const ONE_ITEM: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<catalog>\n  <item id=\"1\"><title>First</title></item>\n</catalog>\n";
const ONE_ITEM_SHA256: &str = "5972608950740eaee2ba162fb8892764b2108e34c91e43fca5ed6c734d9a4181";

type Measures = BTreeMap<String, serde_json::Value>;

fn named_under<'a>(file: &'a Measures, input: &str) -> Vec<&'a String> {
    file.keys()
        .filter(|name| name.starts_with(&format!("{input}/")))
        .collect()
}

fn the_package_manifest() -> serde_json::Value {
    serde_json::from_slice(
        &std::fs::read(package_directory().join("package.json")).expect("the package's manifest"),
    )
    .expect("JSON")
}

fn manifest_of_one_input(sha256: &str) -> (tempfile::TempDir, String) {
    let scratch = scratch();
    let adapter = scratch.path().join("adapter");
    copied_to(&tiny(), &adapter);
    std::fs::create_dir_all(adapter.join("bench")).expect("the bench directory");
    std::fs::write(adapter.join("bench/one.xml"), ONE_ITEM).expect("the input");
    let manifest = adapter.join("bench/manifest.json");
    let listing = serde_json::json!({ "inputs": [{
        "id": "catalog-one",
        "path": "bench/one.xml",
        "sha256": sha256,
        "bytes": ONE_ITEM.len(),
        "measures": ["convert", "triples"],
        "onPullRequests": true,
        "timeoutSeconds": 60,
    }]});
    std::fs::write(&manifest, listing.to_string()).expect("the manifest");
    (scratch, manifest.to_string_lossy().into_owned())
}

/// Running the commands on Node · hosts/node/cascade-bridge.mjs
///
/// The same engine, built as WebAssembly, runs under Node behind a host that offers the native
/// command's commands. Each case runs one command on both hosts and compares them, because a graph
/// must come out the same wherever it is converted.
mod running_the_commands_on_node {
    use super::*;

    mod given_a_document_the_adapter_converts {
        use super::*;

        #[test]
        fn writes_the_graph_and_the_findings_the_native_command_writes() {
            let vocabularies = vocabularies().to_string_lossy().into_owned();
            let facts = tiny()
                .join("fixtures/facts/catalog.ttl")
                .to_string_lossy()
                .into_owned();
            converts_as_the_native_command_does(
                &tiny(),
                "two.xml",
                &["--vocabularies", &vocabularies, "--facts", &facts],
            );
        }

        /// writes the findings the native command draws from the vocabulary's shapes
        #[test]
        fn writes_the_findings_the_native_command_draws_from_the_vocabularys_shapes() {
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
        fn prints_on_standard_error_what_the_native_command_prints() {
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

        /// reports for each entry of `test` the outcome the native command reports
        #[test]
        fn reports_for_each_entry_of_test_the_outcome_the_native_command_reports() {
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
    }

    /// given no `--vocabularies`
    mod given_no_vocabularies {
        use super::*;

        /// refuses to `test`, in the native command's words
        #[test]
        fn refuses_to_test_in_the_native_commands_words() {
            let adapter = tiny().to_string_lossy().into_owned();
            refuses_as_the_native_command_does_without_the_vocabularies(&["test", &adapter]);
        }

        /// refuses `--findings`, in the native command's words, writing no file
        #[test]
        fn refuses_findings_in_the_native_commands_words_writing_no_file() {
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

        /// converts to the native command's graph, saying the same line of the graph going unvalidated
        #[test]
        fn converts_to_the_native_commands_graph_saying_the_same_line_about_it_going_unvalidated() {
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
    }

    mod given_an_adapter_whose_files_are_unusually_named_or_placed {
        use super::*;

        /// converts an adapter under a folder named with a space, a non-ASCII letter and sub-delimiters
        #[test]
        fn converts_an_adapter_under_a_folder_named_with_a_space_a_non_ascii_letter_and_sub_delimiters(
        ) {
            let scratch = scratch();
            let adapter = scratch
                .path()
                .join("une données (old)+@,;=&!")
                .join("tiny-adapter");
            copied_to(&tiny(), &adapter);
            let vocabularies = vocabularies().to_string_lossy().into_owned();
            converts_as_the_native_command_does(
                &adapter,
                "two.xml",
                &["--vocabularies", &vocabularies],
            );
        }

        #[test]
        fn reads_a_directory_inside_the_adapter_whose_name_begins_with_two_dots() {
            let scratch = scratch();
            let adapter = scratch.path().join("dotted-adapter");
            copied_to(&tiny(), &adapter);
            std::fs::rename(adapter.join("mapping"), adapter.join("..mapping"))
                .expect("the renamed mapping");
            let metadata = adapter.join("ro-crate-metadata.json");
            let text = std::fs::read_to_string(&metadata).expect("the metadata");
            std::fs::write(&metadata, text.replace("\"mapping/", "\"..mapping/"))
                .expect("the metadata");
            let vocabularies = vocabularies().to_string_lossy().into_owned();
            converts_as_the_native_command_does(
                &adapter,
                "two.xml",
                &["--vocabularies", &vocabularies],
            );
        }

        /// refuses, as the native command does, a file outside the adapter named with an encoded slash
        #[test]
        fn refuses_a_file_outside_the_adapter_named_with_an_encoded_slash_as_the_native_command_does(
        ) {
            let scratch = scratch();
            let adapter = scratch.path().join("adapter");
            copied_to(&tiny(), &adapter);
            std::fs::rename(
                adapter.join("mapping/detect.rq"),
                scratch.path().join("secret.rq"),
            )
            .expect("the query moved beside the adapter");
            replaced_once(
                &adapter,
                "ro-crate-metadata.json",
                "\"mapping/detect.rq\"",
                "\"..%2Fsecret.rq\"",
            );
            let document = adapter.join("fixtures/in/two.xml");
            let document = document.to_string_lossy().into_owned();
            let adapter = adapter.to_string_lossy().into_owned();
            for (host, run) in [
                ("native", native(&["convert", &adapter, &document])),
                ("node", node(&["convert", &adapter, &document])),
            ] {
                let said = String::from_utf8_lossy(&run.stderr);
                assert_eq!(run.status.code(), Some(2), "{host}: {said}");
                assert!(said.contains("not inside"), "{host}: {said}");
            }
        }
    }

    /// given an adapter whose schema imports the XML namespace
    mod given_an_adapter_whose_schema_imports_the_xml_namespace {
        use super::*;

        /// answers the import itself where the adapter has no `xml.xsd`
        #[test]
        fn answers_the_import_itself_where_the_adapter_has_no_xml_xsd() {
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
            converts_as_the_native_command_does(
                &adapter,
                "two.xml",
                &["--vocabularies", &vocabularies],
            );
        }

        /// validates with the adapter's own `xml.xsd` where it has one
        #[test]
        fn validates_with_the_adapters_own_xml_xsd_where_it_has_one() {
            let scratch = scratch();
            let adapter = scratch.path().join("adapter");
            copied_to(&tiny(), &adapter);
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
            std::fs::write(
                adapter.join("schema/xml.xsd"),
                "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" targetNamespace=\"http://www.w3.org/XML/1998/namespace\">\n  <xs:attribute name=\"lang\">\n    <xs:simpleType>\n      <xs:restriction base=\"xs:language\">\n        <xs:enumeration value=\"fr\"/>\n      </xs:restriction>\n    </xs:simpleType>\n  </xs:attribute>\n</xs:schema>\n",
            )
            .expect("the adapter's own schema of the XML namespace");
            let vocabularies = vocabularies().to_string_lossy().into_owned();
            let found = converts_as_the_native_command_does(
                &adapter,
                "two.xml",
                &["--vocabularies", &vocabularies],
            );
            assert!(
                names(&found, "http://www.w3.org/ns/shacl#Violation"),
                "the item whose xml:lang only the adapter's schema refuses: {found:?}"
            );
        }
    }

    mod given_an_adapter_neither_host_can_load {
        use super::*;

        /// refuses a mapping holding a `SERVICE` pattern, naming the query and the clause
        #[test]
        fn refuses_a_mapping_holding_a_service_pattern_naming_the_query_and_the_clause() {
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

        /// names a mapping that is missing once, by its IRI
        #[test]
        fn names_a_mapping_that_is_missing_once_by_its_iri() {
            each_host_names_the_mapping_once("mapping/missing.rq");
        }

        /// names a mapping that is a directory once, by its IRI
        #[test]
        fn names_a_mapping_that_is_a_directory_once_by_its_iri() {
            each_host_names_the_mapping_once("mapping");
        }
    }

    mod when_something_outside_the_adapter_fails {
        use super::*;

        /// exits 2 saying `standard output` when standard output closes
        #[test]
        fn exits_2_saying_standard_output_when_standard_output_closes() {
            require_the_module();
            let native_run =
                convert_into_a_closed_pipe(Command::new(env!("CARGO_BIN_EXE_cascade-bridge")));
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

        /// exits 2 saying a fault in the Bridge when the engine traps
        #[test]
        fn exits_2_saying_a_fault_in_the_bridge_when_the_engine_traps() {
            let scratch = scratch();
            let host = scratch.path().join("hosts/node");
            copied_to(&node_host_directory(), &host);
            let package = scratch.path().join("package/dist");
            std::fs::create_dir_all(&package).expect("the package's directory");
            std::fs::write(package.join("package.json"), "{ \"type\": \"module\" }\n")
                .expect("the package's manifest");
            std::fs::write(
                package.join("node.js"),
                "const trap = () => { throw new WebAssembly.RuntimeError(\"unreachable\"); };\n\
                 export const describe = trap;\n\
                 export const test = trap;\n\
                 export class Adapter { static load() { trap(); } }\n\
                 export class BridgeError extends Error {}\n",
            )
            .expect("a package whose engine traps");
            let adapter = tiny().to_string_lossy().into_owned();
            let vocabularies = vocabularies().to_string_lossy().into_owned();
            let run = Command::new("node")
                .arg(host.join("cascade-bridge.mjs"))
                .args(["test", &adapter, "--vocabularies", &vocabularies])
                .current_dir(scratch.path())
                .output()
                .expect("run node");
            let said = String::from_utf8_lossy(&run.stderr);
            assert_eq!(run.status.code(), Some(2), "{said}");
            assert!(
                said.starts_with(
                    "cascade-bridge: a fault in the bridge, whose module trapped: unreachable"
                ),
                "the trap is said to be a fault in the Bridge: {said}"
            );
        }
    }
}

/// Answering a calls file for conformance · cascade-bridge library
mod answering_a_calls_file {
    use super::*;

    mod given_the_node_host {
        use super::*;

        #[test]
        fn writes_the_results_the_native_command_writes_for_one_calls_file() {
            let scratch = scratch();
            let adapter_iri = "https://example.org/adapters/catalog/";
            let vocabulary = map_of(&vocabularies(), "https://example.org/vocabularies/", &[]);
            let to_load = map_of(&tiny(), adapter_iri, &["fixtures/"]);
            let lacking = map_of(&tiny(), adapter_iri, &["fixtures/", "mapping/item.rq"]);
            let whole = map_of(&tiny(), adapter_iri, &[]);
            let not_xml = scratch.path().join("not-xml.xml");
            std::fs::write(&not_xml, "<catalog><item").expect("a document that does not parse");
            let document = |path: &Path| {
                serde_json::json!({
                    "iri": "https://example.org/documents/catalog.xml",
                    "path": path.to_string_lossy(),
                })
            };
            let two = document(&tiny().join("fixtures/in/two.xml"));
            let mut with_facts = two.clone();
            with_facts["facts"] = serde_json::json!({
                "iri": "https://example.org/facts/catalog.ttl",
                "path": tiny().join("fixtures/facts/catalog.ttl").to_string_lossy(),
            });
            let calls = serde_json::json!({ "cases": [
                { "name": "describe", "calls": [
                    { "describe": {
                        "adapter": adapter_iri,
                        "metadata": tiny().join("ro-crate-metadata.json").to_string_lossy(),
                        "format": "turtle" } } ] },
                { "name": "convert", "calls": [
                    { "convert": { "document": two, "format": "turtle" } },
                    { "load": { "adapter": to_load, "vocabulary": vocabulary } },
                    { "ask": { "document": two } },
                    { "convert": { "document": document(&not_xml), "format": "turtle" } },
                    { "convert": { "document": with_facts, "format": "ntriples" } } ] },
                { "name": "missing", "calls": [
                    { "load": { "adapter": lacking, "vocabulary": vocabulary } },
                    { "load": { "adapter": to_load } },
                    { "convert": { "document": two, "format": "turtle" } } ] },
                { "name": "test", "calls": [
                    { "test": { "adapter": whole, "vocabulary": vocabulary } },
                    { "test": { "adapter": whole } } ] }
            ] });
            let file = scratch.path().join("calls.json");
            std::fs::write(&file, calls.to_string()).expect("the calls file");
            let mut held = BTreeMap::new();
            for host in ["native", "node"] {
                let directory = scratch.path().join(host);
                std::fs::create_dir(&directory).expect("an empty results directory");
                let arguments = [
                    "library",
                    &file.to_string_lossy(),
                    &directory.to_string_lossy(),
                ];
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
                held.insert(host, results(&directory));
            }
            assert!(
                !held["native"].is_empty(),
                "the native command wrote results"
            );
            assert_eq!(held["node"], held["native"]);
        }
    }

    mod when_a_result_cannot_be_written {
        use super::*;

        /// exits 2 on each host, naming the file, with no answer written
        #[test]
        fn exits_2_on_each_host_naming_the_file_with_no_answer_written() {
            let scratch = scratch();
            let calls = serde_json::json!({ "cases": [
                { "name": "describe", "calls": [
                    { "describe": {
                        "adapter": "https://example.org/adapters/catalog/",
                        "metadata": tiny().join("ro-crate-metadata.json").to_string_lossy(),
                        "format": "turtle" } } ] }
            ] });
            let file = scratch.path().join("calls.json");
            std::fs::write(&file, calls.to_string()).expect("the calls file");
            for host in ["native", "node"] {
                let directory = scratch.path().join(host);
                let blocked = directory.join("describe/1.graph");
                std::fs::create_dir_all(&blocked).expect("a directory where the graph would go");
                let arguments = [
                    "library",
                    &file.to_string_lossy(),
                    &directory.to_string_lossy(),
                ];
                let run = match host {
                    "native" => native(&arguments),
                    _ => node(&arguments),
                };
                let said = String::from_utf8_lossy(&run.stderr);
                assert_eq!(run.status.code(), Some(2), "{host}: {said}");
                assert!(said.contains("1.graph"), "{host}: {said}");
                assert!(
                    !directory.join("describe/1.json").exists(),
                    "{host}: an answer was written"
                );
            }
        }
    }
}

/// Measuring conversions for the benchmark · hosts/node/bench.mjs
///
/// `bench.mjs` converts documents on Node and writes a result file of measures in the form the
/// specification's `bench/result.schema.json` gives. An engine's speed over time is drawn from these
/// files, so a measure keeps its name and its kind.
mod measuring_for_the_benchmark {
    use super::*;

    /// writes each measure under its input's name, of the kind the schema gives it
    #[test]
    fn writes_each_measure_under_its_inputs_name_of_the_kind_the_schema_gives_it() {
        let two = tiny().join("fixtures/in/two.xml");
        let file = benched(&[&two], &[]);
        let kind = |name: &str| {
            file.get(name)
                .and_then(|entry| entry["kind"].as_str())
                .unwrap_or_else(|| panic!("no {name} in {:?}", file.keys()))
                .to_owned()
        };
        for (name, expected) in [
            ("two/convert/first", "clock"),
            ("two/convert/second", "clock"),
            ("two/memory.grow/first", "cost"),
            ("two/memory.grow/second", "cost"),
            ("two/wasm-memory/first", "cost"),
            ("two/wasm-memory/second", "cost"),
            ("two/rss", "clock"),
            ("two/records", "output"),
            ("two/triples", "output"),
            ("two/findings", "output"),
            ("two/index-built", "output"),
            ("two/fallbacks", "output"),
            ("two/output-bytes/graph", "output"),
            ("two/output-bytes/findings", "output"),
            ("two/query-time/item.rq", "clock"),
            ("two/query-rows/item.rq", "output"),
            ("two/query-time/detect.rq", "clock"),
            ("catalog/load", "clock"),
            ("catalog/load/files-read", "cost"),
            ("catalog/load/missing-files", "cost"),
        ] {
            assert_eq!(kind(name), expected, "{name}");
        }
    }

    /// counts the triples `convert` reports for the same document
    #[test]
    fn counts_the_triples_convert_reports_for_the_same_document() {
        let two = tiny().join("fixtures/in/two.xml");
        let file = benched(&[&two], &[]);
        let vocabularies = vocabularies().to_string_lossy().into_owned();
        let (adapter, document) = (
            tiny().to_string_lossy().into_owned(),
            two.to_string_lossy().into_owned(),
        );
        let converted = node(&[
            "convert",
            &adapter,
            &document,
            "--vocabularies",
            &vocabularies,
        ]);
        let said = String::from_utf8_lossy(&converted.stderr).into_owned();
        let triples = file["two/triples"]["value"].as_u64().expect("a count");
        assert!(said.contains(&format!(" {triples} triples")), "{said}");
    }

    #[test]
    fn records_the_build_profile_and_the_commit_of_the_package_it_measured() {
        let two = tiny().join("fixtures/in/two.xml");
        let file = benched(&[&two], &[]);
        let built = the_package_manifest();
        assert_eq!(file["env"]["profile"], built["cascadeBridge"]["profile"]);
        let commit = file["commit"]["cascade-bridge-rs"]
            .as_str()
            .expect("the engine's commit");
        let built_commit = built["cascadeBridge"]["commit"].as_str().expect("a commit");
        assert!(commit.starts_with(built_commit), "{commit}");
    }

    /// exits 1 where a record's query falls back to the general evaluator, naming the measure
    #[test]
    fn exits_1_where_a_records_query_falls_back_to_the_general_evaluator_naming_the_measure() {
        let scratch = scratch();
        let adapter = scratch.path().join("adapter");
        copied_to(&tiny(), &adapter);
        replaced_once(
            &adapter,
            "mapping/item.rq",
            "rdf:_1 ?title",
            "rdf:_1/rdf:_1* ?title",
        );
        let two = tiny().join("fixtures/in/two.xml");
        let vocabularies = vocabularies();
        let run = bench(&[
            adapter.to_str().expect("a path"),
            two.to_str().expect("a path"),
            "--vocabularies",
            vocabularies.to_str().expect("a path"),
            "--samples",
            "1",
        ]);
        let said = String::from_utf8_lossy(&run.stderr);
        assert_eq!(run.status.code(), Some(1), "{said}");
        assert!(
            said.contains("two/fallbacks: ") && said.contains("fell back to the general evaluator"),
            "{said}"
        );
    }

    /// with `--executor <name>`
    mod with_executor {
        use super::*;

        /// converts to as many triples with each executor, building an index only where a query is compiled
        #[test]
        fn converts_to_as_many_triples_with_each_executor_building_an_index_only_where_compiled() {
            let two = tiny().join("fixtures/in/two.xml");
            let by_default = benched(&[&two], &[]);
            assert_eq!(by_default["two/index-built"]["value"], 1);
            for (executor, index_built) in [("compiled", 1), ("spareval", 0), ("both", 1)] {
                let file = benched(&[&two], &["--executor", executor]);
                assert_eq!(file["two/index-built"]["value"], index_built, "{executor}");
                assert_eq!(
                    file["two/triples"]["value"], by_default["two/triples"]["value"],
                    "{executor}"
                );
            }
        }

        /// refuses a name it does not know, with the usage
        #[test]
        fn refuses_a_name_it_does_not_know_with_the_usage() {
            let two = tiny().join("fixtures/in/two.xml");
            let adapter = tiny();
            let refused = bench(&[
                adapter.to_str().expect("a path"),
                two.to_str().expect("a path"),
                "--executor",
                "fast",
            ]);
            assert_eq!(refused.status.code(), Some(2));
            assert!(String::from_utf8_lossy(&refused.stderr).contains("usage:"));
        }
    }

    /// with `--inputs <manifest>`
    mod with_inputs {
        use super::*;

        /// measures each input the manifest lists, and only the measures it asks for
        #[test]
        fn measures_each_input_the_manifest_lists_and_only_the_measures_it_asks_for() {
            let (_scratch, manifest) = manifest_of_one_input(ONE_ITEM_SHA256);
            let file = benched_with(&["--inputs", manifest.as_str()]);
            let named: Vec<&String> = file
                .keys()
                .filter(|name| name.starts_with("catalog-one/"))
                .collect();
            assert_eq!(
                named,
                [
                    "catalog-one/convert/first",
                    "catalog-one/convert/second",
                    "catalog-one/triples"
                ]
            );
        }

        /// refuses an input whose bytes are not the SHA-256 the manifest gives, naming the one it found
        #[test]
        fn refuses_an_input_whose_bytes_are_not_the_manifests_sha_256_naming_the_one_it_found() {
            let (_scratch, manifest) = manifest_of_one_input(&"0".repeat(64));
            let refused = bench(&["--inputs", manifest.as_str(), "--samples", "1"]);
            let said = String::from_utf8_lossy(&refused.stderr);
            assert_eq!(refused.status.code(), Some(2), "{said}");
            assert!(said.contains(ONE_ITEM_SHA256), "{said}");
        }
    }

    mod given_an_input_it_cannot_measure {
        use super::*;

        /// records only `refused/documentFailure` for a document the adapter refuses
        #[test]
        fn records_only_refused_document_failure_for_a_document_the_adapter_refuses() {
            let scratch = scratch();
            let broken = scratch.path().join("broken.xml");
            std::fs::write(&broken, "<catalog><item").expect("a document that does not parse");
            let two = tiny().join("fixtures/in/two.xml");
            let file = benched(&[&broken, &two], &[]);
            let entry = "broken/refused/documentFailure";
            assert_eq!(named_under(&file, "broken"), [entry]);
            assert_eq!(file[entry]["kind"], "output");
            assert_eq!(file[entry]["value"].as_f64(), Some(1.0));
            assert!(file.contains_key("two/convert/first"), "{:?}", file.keys());
        }

        /// records only `timeout/load` for a load that takes longer than `--timeout`
        #[test]
        fn records_only_timeout_load_for_a_load_that_takes_longer_than_the_timeout() {
            let two = tiny().join("fixtures/in/two.xml");
            let file = benched(&[&two], &["--timeout", "0.001"]);
            let entry = "two/timeout/load";
            assert_eq!(named_under(&file, "two"), [entry]);
            assert_eq!(file[entry]["kind"], "output");
            assert_eq!(file[entry]["value"].as_f64(), Some(0.001));
        }
    }
}

/// The package's imports, held to the allowlist · package/check-imports.mjs
mod the_packages_imports {
    use super::*;

    /// passes on the package's module
    #[test]
    fn passes_on_the_packages_module() {
        let run = check_imports(&allowlist());
        assert!(
            run.status.success(),
            "{}{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
    }

    #[test]
    fn names_the_import_an_allowlist_one_entry_short_leaves_out() {
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
}
