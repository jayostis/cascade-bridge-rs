//! `test` and `convert` for a host that is not a process.
//!
//! What these functions take and return is the node host's business and
//! promises nothing to anyone else.
use cascade_bridge::{
    earl_report, load_adapter, prepare, require_vocabularies, run_manifest, serialise,
    serialise_at, unread, unvalidated_output, Conversion, EntryResult, Error, GraphFormat, Outcome,
    ReportSubject, Resolver, RunOptions, Source, OFFERED_PROFILES,
};
use std::fmt::Write;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// Each method throws a string saying why a file could not be read.
    pub type Files;

    #[wasm_bindgen(method, catch)]
    fn read(this: &Files, iri: &str) -> Result<Vec<u8>, JsValue>;

    #[wasm_bindgen(method, catch, js_name = readVocabulary)]
    fn read_vocabulary(this: &Files, iri: &str) -> Result<Vec<u8>, JsValue>;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(message: &str);
}

/// A panic reaches the host as a trap saying only "unreachable", so its message goes first.
#[wasm_bindgen(start)]
fn start() {
    std::panic::set_hook(Box::new(|panic| error(&panic.to_string())));
}

struct Host {
    root: String,
    vocabularies: Option<String>,
    files: Files,
}

fn refused(iri: &str, thrown: JsValue) -> Error {
    match thrown.as_string() {
        Some(reason) if reason == "ENOENT" => Error::missing(format!("{iri}: {reason}")),
        Some(reason) => unread(iri, &reason),
        None => unread(iri, &format!("{thrown:?}")),
    }
}

impl Resolver for Host {
    fn root(&self) -> &str {
        &self.root
    }

    fn vocabularies(&self) -> Option<&str> {
        self.vocabularies.as_deref()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        self.files.read(iri).map_err(|e| refused(iri, e))
    }

    fn read_vocabulary(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        match &self.vocabularies {
            Some(_) => self.files.read_vocabulary(iri).map_err(|e| refused(iri, e)),
            None => Err(Error::msg(format!(
                "the command named no vocabularies: {iri}"
            ))),
        }
    }
}

fn thrown(error: Error) -> JsError {
    JsError::new(&error.to_string())
}

/// `root` and `vocabularies` name directories, each ending in "/".
fn host(root: String, vocabularies: Option<String>, files: Files) -> Host {
    Host {
        root,
        vocabularies,
        files,
    }
}

fn subject() -> ReportSubject {
    ReportSubject {
        iri: "https://github.com/jayostis/cascade-bridge-rs".to_owned(),
        name: "Cascade Bridge for Rust".to_owned(),
        version: "0.0.0".to_owned(),
    }
}

const FAILING: [Outcome; 2] = [Outcome::Failed, Outcome::Inapplicable];

#[wasm_bindgen(getter_with_clone)]
pub struct TestRun {
    pub summary: String,
    pub earl: String,
    pub holds: bool,
}

#[wasm_bindgen]
pub fn test(
    root: String,
    vocabularies: Option<String>,
    files: Files,
    datasets: bool,
) -> Result<TestRun, JsError> {
    let host = host(root, vocabularies, files);
    let subject = subject();
    let adapter = load_adapter(&host).map_err(thrown)?;
    require_vocabularies(&adapter, &host).map_err(thrown)?;
    let results = run_manifest(&adapter, &host, RunOptions { datasets }).map_err(thrown)?;
    let mut summary = String::new();
    let _ = writeln!(
        summary,
        "Adapter  {}  ({})",
        adapter.identifier.as_deref().unwrap_or(&adapter.root),
        adapter.root
    );
    let _ = writeln!(
        summary,
        "Bridge   {} {}, offers {}\n",
        subject.name,
        subject.version,
        OFFERED_PROFILES
            .iter()
            .map(|p| p.split('#').nth(1).unwrap_or(p))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let width = results
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    for result in &results {
        let _ = writeln!(
            summary,
            "  {:<12} {:<width$}  {:>6} s  {}",
            result.outcome.as_str(),
            result.name,
            format!("{:.2}", result.elapsed.as_secs_f64()),
            result.description,
        );
    }
    let _ = write!(summary, "\n{}", tally(&results));
    Ok(TestRun {
        summary,
        earl: earl_report(&results, &subject).map_err(thrown)?,
        holds: !results.iter().any(|r| FAILING.contains(&r.outcome)),
    })
}

fn tally(results: &[EntryResult]) -> String {
    let mut counts: Vec<(Outcome, usize)> = Vec::new();
    for result in results {
        match counts.iter_mut().find(|(o, _)| *o == result.outcome) {
            Some((_, n)) => *n += 1,
            None => counts.push((result.outcome, 1)),
        }
    }
    counts
        .iter()
        .map(|(outcome, n)| format!("{n} {}", outcome.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

#[wasm_bindgen]
pub struct Converted {
    conversion: Conversion,
    format: GraphFormat,
    prefixes: Vec<(String, String)>,
    findings_prefixes: Vec<(String, String)>,
    summary: String,
}

#[wasm_bindgen]
impl Converted {
    #[wasm_bindgen(getter)]
    pub fn summary(&self) -> String {
        self.summary.clone()
    }

    pub fn graph(&self) -> Result<String, JsError> {
        serialise(&self.conversion.quads, self.format, &self.prefixes).map_err(thrown)
    }

    pub fn findings(&self, at: &str) -> Result<String, JsError> {
        serialise_at(
            &self.conversion.findings,
            self.format,
            &self.findings_prefixes,
            Some(at),
        )
        .map_err(thrown)
    }
}

#[wasm_bindgen]
pub fn convert(
    root: String,
    vocabularies: Option<String>,
    files: Files,
    document_iri: &str,
    document: &[u8],
    format: &str,
    findings: bool,
) -> Result<Converted, JsError> {
    let format =
        GraphFormat::named(format).ok_or_else(|| JsError::new(&format!("no format {format}")))?;
    let host = host(root, vocabularies, files);
    let adapter = load_adapter(&host).map_err(thrown)?;
    let unvalidated = if findings {
        require_vocabularies(&adapter, &host).map_err(thrown)?;
        None
    } else {
        unvalidated_output(&adapter, &host)
    };
    let prepared = prepare(&adapter, &host).map_err(thrown)?;
    let conversion = cascade_bridge::convert(
        &prepared,
        Source {
            iri: document_iri,
            envelope: None,
            xml: document,
        },
    )
    .map_err(thrown)?;
    let mut summary = String::new();
    if let Some(unvalidated) = unvalidated {
        let _ = writeln!(summary, "cascade-bridge: {unvalidated}");
    }
    let _ = writeln!(
        summary,
        "Adapter  {}  ({})",
        adapter.identifier.as_deref().unwrap_or(&adapter.root),
        adapter.root
    );
    let _ = writeln!(
        summary,
        "Document {document_iri}  {} record(s), {} triples, {} finding(s)",
        conversion.units,
        conversion.triples(),
        conversion.annotations()
    );
    let _ = write!(
        summary,
        "Detect   {}",
        match conversion.detected {
            Some(true) => "true",
            Some(false) => "false: this adapter does not claim this document, converted anyway",
            None => "the adapter names no bridge:detectQuery",
        }
    );
    Ok(Converted {
        conversion,
        format,
        prefixes: prepared.prefixes,
        findings_prefixes: prepared.findings_prefixes,
        summary,
    })
}
