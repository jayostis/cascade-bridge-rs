mod common;

use cascade_bridge::command::{self, Host};
use cascade_bridge::{path_to_file_iri, Error, Resolver, Result};
use common::{tiny, tiny_with_vocabularies, Variant, CRATE};
use oxrdf::Term;
use oxrdfio::{RdfFormat, RdfParser};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

const DOAP_REVISION: &str = "http://usefulinc.com/ns/doap#revision";

/// Everything the command asked of its host, in the order it asked, with the
/// adapter's reads among them.
type Said = Rc<RefCell<Vec<String>>>;

/// A host holding the adapter it was built with, whatever directory the
/// command names, and every file the command writes.
struct Recorder {
    adapter: Option<Box<dyn Resolver>>,
    said: Said,
    written: BTreeMap<String, String>,
}

struct Logged {
    adapter: Box<dyn Resolver>,
    said: Said,
}

impl Resolver for Logged {
    fn root(&self) -> &str {
        self.adapter.root()
    }

    fn vocabularies(&self) -> Option<&str> {
        self.adapter.vocabularies()
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        self.said.borrow_mut().push(format!("adapter {iri}"));
        self.adapter.read(iri)
    }

    fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
        self.adapter.read_vocabulary(iri)
    }
}

impl Recorder {
    fn of(adapter: impl Resolver + 'static) -> Self {
        Self {
            adapter: Some(Box::new(adapter)),
            said: Rc::default(),
            written: BTreeMap::new(),
        }
    }

    fn run(&mut self, argv: &[&str]) -> u8 {
        command::run(argv.iter().map(|a| (*a).to_owned()), self)
    }

    fn said(&self) -> Vec<String> {
        self.said.borrow().clone()
    }

    fn first(&self, starting: &str) -> usize {
        let said = self.said();
        said.iter()
            .position(|s| s.starts_with(starting))
            .unwrap_or_else(|| panic!("nothing starting {starting:?} in {said:#?}"))
    }

    fn printed(&self, stream: &str) -> String {
        self.said()
            .iter()
            .filter_map(|s| s.strip_prefix(&format!("{stream} ")))
            .collect()
    }

    fn tell(&self, what: String) {
        self.said.borrow_mut().push(what);
    }
}

impl Host for Recorder {
    fn resolver(
        &mut self,
        _directory: &str,
        _vocabularies: Option<&str>,
    ) -> Result<Box<dyn Resolver>> {
        let adapter = self.adapter.take().expect("one adapter a run");
        Ok(Box::new(Logged {
            adapter,
            said: self.said.clone(),
        }))
    }

    fn read(&mut self, path: &str) -> Result<Vec<u8>> {
        self.tell(format!("read {path}"));
        std::fs::read(path).map_err(|e| Error::msg(format!("{path}: {e}")))
    }

    fn file_iri(&mut self, path: &str) -> Result<String> {
        self.tell(format!("iri {path}"));
        Ok(path_to_file_iri(Path::new(path)))
    }

    fn create(&mut self, path: &str) -> Result<()> {
        self.tell(format!("create {path}"));
        self.written.entry(path.to_owned()).or_default();
        Ok(())
    }

    fn write(&mut self, path: &str, text: &str) -> Result<()> {
        self.tell(format!("write {path}"));
        self.written.insert(path.to_owned(), text.to_owned());
        Ok(())
    }

    fn out(&mut self, text: &str) -> Result<()> {
        self.tell(format!("out {text}"));
        Ok(())
    }

    fn err(&mut self, text: &str) {
        self.tell(format!("err {text}"));
    }
}

fn two() -> String {
    common::tiny_directory()
        .join("fixtures/in/two.xml")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn test_says_the_adapter_this_bridge_at_its_own_version_and_the_tally() {
    let mut host = Recorder::of(tiny_with_vocabularies());
    let status = host.run(&["test", "adapter", "--vocabularies", "checkout"]);
    let printed = host.printed("out");
    let lines: Vec<&str> = printed.lines().collect();
    assert!(
        lines
            .first()
            .is_some_and(|l| l.starts_with("Adapter  catalog")),
        "{printed}"
    );
    let bridge = format!(
        "Bridge   Cascade Bridge for Rust {}, offers ",
        env!("CARGO_PKG_VERSION")
    );
    assert!(lines.iter().any(|l| l.starts_with(&bridge)), "{printed}");
    assert_eq!(
        lines.last(),
        Some(&"4 passed, 2 failed, 1 cantTell, 1 untested"),
        "{printed}"
    );
    assert_eq!(status, 1, "an entry failed");
}

#[test]
fn test_reports_this_bridge_at_its_own_version() {
    let mut host = Recorder::of(tiny_with_vocabularies());
    host.run(&["test", "adapter", "--earl", "report.ttl"]);
    let report = host
        .written
        .get("report.ttl")
        .expect("the report --earl asked for");
    let revisions: Vec<String> = RdfParser::from_format(RdfFormat::Turtle)
        .for_slice(report.as_bytes())
        .map(|quad| quad.expect("a parsed report"))
        .filter(|quad| quad.predicate.as_str() == DOAP_REVISION)
        .map(|quad| match quad.object {
            Term::Literal(revision) => revision.value().to_owned(),
            other => panic!("a revision that is not a literal: {other}"),
        })
        .collect();
    assert_eq!(revisions, [env!("CARGO_PKG_VERSION")]);
    assert!(
        host.printed("out").ends_with("EARL     report.ttl\n"),
        "{}",
        host.printed("out")
    );
}

#[test]
fn test_writes_no_report_where_none_was_asked_for() {
    let mut host = Recorder::of(tiny_with_vocabularies());
    host.run(&["test", "adapter"]);
    assert!(host.written.is_empty());
}

#[test]
fn refuses_what_it_cannot_parse_with_the_usage_and_status_two() {
    for argv in [
        &["validate"][..],
        &["test"],
        &["test", "adapter", "--earl"],
        &["convert", "adapter", "document.xml", "--format", "rdfxml"],
    ] {
        let mut host = Recorder::of(tiny());
        assert_eq!(host.run(argv), 2, "{argv:?}");
        assert!(
            host.printed("err")
                .starts_with("usage: cascade-bridge test"),
            "{argv:?}: {}",
            host.printed("err")
        );
        assert!(host.printed("out").is_empty(), "{argv:?}");
    }
}

#[test]
fn convert_fills_the_findings_file_it_created_under_that_file_s_iri_before_the_graph() {
    let mut host = Recorder::of(tiny_with_vocabularies());
    let document = two();
    let status = host.run(&[
        "convert",
        "adapter",
        &document,
        "--findings",
        "findings.ttl",
        "--out",
        "graph.ttl",
    ]);
    assert_eq!(status, 0, "{:#?}", host.said());
    assert!(host.first("create findings.ttl") < host.first("iri findings.ttl"));
    assert!(host.first("iri findings.ttl") < host.first("write findings.ttl"));
    assert!(host.first("write findings.ttl") < host.first("write graph.ttl"));
    assert!(host.first("write graph.ttl") < host.first("err Graph    graph.ttl"));
    assert!(host.printed("out").is_empty(), "the graph went to --out");
    let said = host.printed("err");
    assert!(
        said.ends_with("Findings findings.ttl\nGraph    graph.ttl\n"),
        "{said}"
    );
    assert!(
        said.contains(&format!(
            "Document {}  2 record(s)",
            path_to_file_iri(Path::new(&document))
        )),
        "{said}"
    );
}

#[test]
fn convert_writes_the_graph_to_standard_output_where_no_file_is_named() {
    let mut host = Recorder::of(tiny());
    assert_eq!(host.run(&["convert", "adapter", &two()]), 0);
    assert!(host.written.is_empty());
    assert!(!host.printed("out").is_empty());
    assert!(!host.printed("err").contains("Graph"));
}

#[test]
fn stops_with_status_two_and_the_reason_where_a_file_cannot_be_read() {
    let mut host = Recorder::of(tiny());
    let status = host.run(&["convert", "adapter", "no-such-document.xml"]);
    assert_eq!(status, 2);
    let said = host.printed("err");
    assert!(
        said.starts_with("cascade-bridge: no-such-document.xml: "),
        "{said}"
    );
}

#[test]
fn test_says_the_adapter_and_this_bridge_before_it_runs_an_entry() {
    let mut host = Recorder::of(tiny_with_vocabularies());
    host.run(&["test", "adapter"]);
    let said = host.said();
    let header = said
        .iter()
        .position(|s| s.starts_with("out Adapter  catalog"))
        .expect("the adapter line");
    let entry = said
        .iter()
        .position(|s| s.starts_with("adapter ") && s.contains("/fixtures/in/"))
        .expect("an entry's input read");
    assert!(header < entry, "{said:#?}");
}

#[test]
fn convert_reads_no_document_where_the_adapter_does_not_load() {
    let mut host = Recorder::of(Variant::of(tiny()).with(CRATE, "{}"));
    assert_eq!(host.run(&["convert", "adapter", "no-such-document.xml"]), 2);
    let said = host.printed("err");
    assert!(said.contains("names no root entity"), "{said}");
    assert!(
        !host.said().iter().any(|s| s.starts_with("read ")),
        "{:#?}",
        host.said()
    );
}
