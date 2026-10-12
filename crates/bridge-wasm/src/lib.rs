#[cfg(target_arch = "wasm32")]
mod allocator;

use cascade_bridge::{
    Budgets, ErrorKind, Executor, Files, Format, Loaded, Map, Named, QueryProfile,
};
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"
export type Kind = "document" | "facts" | "adapter" | "vocabulary" | "missing" | "bridge";

/**
 * What every export throws when it fails. A `WebAssembly.RuntimeError` thrown by
 * an export is a fault in the Bridge too, as kind `"bridge"` is: after either, the
 * module instance must be discarded.
 */
export interface BridgeError extends Error {
  name: "BridgeError";
  kind: Kind;
  map?: "adapter" | "vocabulary";
  path?: string;
}

export type Format = "turtle" | "ntriples";

/** Files keyed by their paths, as the crate or a `bridge:vocabularyFile` writes them, and the IRI those resolve against. */
export interface Named {
  iri: string;
  files: Map<string, Uint8Array>;
}

export interface Facts {
  iri: string;
  bytes: Uint8Array;
}

export interface Document {
  iri: string;
  bytes: Uint8Array;
  envelope?: string;
  facts?: Facts;
}

export interface Description {
  graph: Uint8Array;
  iri: string;
  identifier?: string;
  version?: string;
  sourceMediaType?: string;
  envelopes: string[];
  loadFiles: string[];
  crateFiles: string[];
  vocabulary?: { repository?: string; files: string[] };
}

export interface ConvertOptions {
  format?: Format;
  findingsRelativeTo?: string;
  /** Times each query, and counts its executions and rows, into `Conversion.queries`. */
  profile?: boolean;
  /** The engine that answers a query: the plan it compiles to, with the general evaluator behind it (the default), the general evaluator alone, or both, failing where they differ. */
  executor?: "compiled" | "spareval" | "both";
  /** The heap, in MiB, the module may hold while converting. A convert that passes it stops with a `BridgeError` of kind `"bridge"`, and the adapter must be loaded again. The default is 1024. */
  memoryBudgetMiB?: number;
  /** The seconds a convert may run. A convert that passes it stops as one past its memory budget does. The default is 120. */
  timeBudgetSeconds?: number;
}

/** A CONSTRUCT's rows are the triples it constructed; an ASK's, 1 when it answered true. */
export interface QueryProfile {
  iri: string;
  executions: number;
  rows: number;
  seconds: number;
  /** Why the query was not compiled; absent where it was. */
  fallback?: string;
  /** Executions a compiled plan gave up on, which the general evaluator answered. */
  giveUps: number;
}

export interface Conversion {
  graph: Uint8Array;
  findings: Uint8Array;
  records: number;
  triples: number;
  findingCount: number;
  detected?: boolean;
  unvalidated?: string;
  /** In the order each query first ran, where `ConvertOptions.profile` asked for them. */
  queries?: QueryProfile[];
}

export interface TestOptions {
  datasets?: boolean;
  executor?: "compiled" | "spareval" | "both";
  /** As in `ConvertOptions`, for each entry's convert. */
  memoryBudgetMiB?: number;
  timeBudgetSeconds?: number;
}

export interface TestEntry {
  name: string;
  outcome: "passed" | "failed" | "cantTell" | "untested" | "inapplicable";
  description: string;
  seconds: number;
}

export interface TestReport {
  earl: Uint8Array;
  entries: TestEntry[];
  bridge: { name: string; version: string; profiles: string[] };
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Named")]
    pub type NamedValue;
    #[wasm_bindgen(typescript_type = "Document")]
    pub type DocumentValue;
    #[wasm_bindgen(typescript_type = "Format")]
    pub type FormatValue;
    #[wasm_bindgen(typescript_type = "Description")]
    pub type DescriptionValue;
    #[wasm_bindgen(typescript_type = "ConvertOptions")]
    pub type ConvertOptionsValue;
    #[wasm_bindgen(typescript_type = "Conversion")]
    pub type ConversionValue;
    #[wasm_bindgen(typescript_type = "TestOptions")]
    pub type TestOptionsValue;
    #[wasm_bindgen(typescript_type = "TestReport")]
    pub type TestReportValue;
}

static FAULTED: AtomicBool = AtomicBool::new(false);

fn record_panic() {
    FAULTED.store(true, Ordering::SeqCst);
}

fn guard() -> cascade_bridge::Result<()> {
    if FAULTED.load(Ordering::SeqCst) {
        return Err(cascade_bridge::Error::new(
            ErrorKind::Bridge,
            "an earlier call on this module instance panicked, so the instance must be discarded",
        ));
    }
    Ok(())
}

#[wasm_bindgen(start)]
fn start() {
    std::panic::set_hook(Box::new(|panic| {
        record_panic();
        let payload = panic.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|said| (*said).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| panic.to_string());
        wasm_bindgen::throw_val(thrown(cascade_bridge::Error::new(
            ErrorKind::Bridge,
            message,
        )));
    }));
}

fn thrown(error: cascade_bridge::Error) -> JsValue {
    let thrown = JsValue::from(js_sys::Error::new(error.message()));
    set(&thrown, "name", &"BridgeError".into());
    let kind = match error.kind() {
        ErrorKind::Document => "document",
        ErrorKind::Facts => "facts",
        ErrorKind::Adapter => "adapter",
        ErrorKind::Vocabulary => "vocabulary",
        ErrorKind::Missing { map, path } => {
            let map = match map {
                Map::Adapter => "adapter",
                Map::Vocabulary => "vocabulary",
            };
            set(&thrown, "map", &map.into());
            set(&thrown, "path", &path.as_str().into());
            "missing"
        }
        ErrorKind::Bridge => "bridge",
    };
    set(&thrown, "kind", &kind.into());
    thrown
}

fn mistaken(said: String) -> JsValue {
    js_sys::TypeError::new(&said).into()
}

fn set(object: &JsValue, key: &str, value: &JsValue) {
    // Each object set on is one this module just made, which takes any property.
    let _ = Reflect::set(object, &key.into(), value);
}

fn get(object: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    Reflect::get(object, &key.into())
}

fn absent(value: &JsValue) -> bool {
    value.is_undefined() || value.is_null()
}

fn text(object: &JsValue, key: &str, what: &str) -> Result<String, JsValue> {
    get(object, key)?
        .as_string()
        .ok_or_else(|| mistaken(format!("{what}.{key} is not a string")))
}

fn optional_text(object: &JsValue, key: &str, what: &str) -> Result<Option<String>, JsValue> {
    let value = get(object, key)?;
    if absent(&value) {
        return Ok(None);
    }
    value
        .as_string()
        .map(Some)
        .ok_or_else(|| mistaken(format!("{what}.{key} is not a string")))
}

fn bytes(object: &JsValue, key: &str, what: &str) -> Result<Vec<u8>, JsValue> {
    get(object, key)?
        .dyn_into::<Uint8Array>()
        .map(|bytes| bytes.to_vec())
        .map_err(|_| mistaken(format!("{what}.{key} is not a Uint8Array")))
}

fn named(value: &JsValue, what: &str) -> Result<(String, Files), JsValue> {
    let iri = text(value, "iri", what)?;
    let map = get(value, "files")?
        .dyn_into::<js_sys::Map>()
        .map_err(|_| mistaken(format!("{what}.files is not a Map")))?;
    let mut files = Files::new();
    for entry in map.entries() {
        let entry = Array::from(&entry?);
        let path = entry
            .get(0)
            .as_string()
            .ok_or_else(|| mistaken(format!("a key of {what}.files is not a string")))?;
        let bytes = entry
            .get(1)
            .dyn_into::<Uint8Array>()
            .map_err(|_| mistaken(format!("{what}.files holds {path} as no Uint8Array")))?;
        files.insert(path, bytes.to_vec());
    }
    Ok((iri, files))
}

fn vocabulary_named(vocabulary: Option<NamedValue>) -> Result<Option<(String, Files)>, JsValue> {
    vocabulary
        .filter(|vocabulary| !absent(vocabulary))
        .map(|vocabulary| named(&vocabulary, "vocabulary"))
        .transpose()
}

fn as_named((iri, files): &(String, Files)) -> Named<'_> {
    Named { iri, files }
}

struct DocumentRead {
    iri: String,
    bytes: Vec<u8>,
    envelope: Option<String>,
    facts: Option<(String, Vec<u8>)>,
}

impl DocumentRead {
    fn of(value: &JsValue) -> Result<Self, JsValue> {
        let facts = get(value, "facts")?;
        Ok(Self {
            iri: text(value, "iri", "document")?,
            bytes: bytes(value, "bytes", "document")?,
            envelope: optional_text(value, "envelope", "document")?,
            facts: if absent(&facts) {
                None
            } else {
                Some((
                    text(&facts, "iri", "document.facts")?,
                    bytes(&facts, "bytes", "document.facts")?,
                ))
            },
        })
    }

    fn document(&self) -> cascade_bridge::Document<'_> {
        cascade_bridge::Document {
            iri: &self.iri,
            bytes: &self.bytes,
            envelope: self.envelope.as_deref(),
            facts: self
                .facts
                .as_ref()
                .map(|(iri, bytes)| cascade_bridge::Facts { iri, bytes }),
        }
    }
}

fn format(value: Option<&JsValue>) -> Result<Format, JsValue> {
    match value.filter(|value| !absent(value)) {
        None => Ok(Format::Turtle),
        Some(value) => value
            .as_string()
            .and_then(|name| Format::named(&name))
            .ok_or_else(|| mistaken("a format is \"turtle\" or \"ntriples\"".to_owned())),
    }
}

fn strings(values: &[String]) -> JsValue {
    values
        .iter()
        .map(|value| JsValue::from(value.as_str()))
        .collect::<Array>()
        .into()
}

fn optional(value: Option<&str>) -> JsValue {
    value.map_or(JsValue::UNDEFINED, JsValue::from)
}

fn array(bytes: &[u8]) -> JsValue {
    Uint8Array::from(bytes).into()
}

fn profiles(queries: &[QueryProfile]) -> JsValue {
    queries
        .iter()
        .map(|query| {
            let value = JsValue::from(Object::new());
            set(&value, "iri", &query.iri.as_str().into());
            set(&value, "executions", &(query.executions as f64).into());
            set(&value, "rows", &(query.rows as f64).into());
            set(&value, "seconds", &query.elapsed.as_secs_f64().into());
            set(&value, "giveUps", &(query.give_ups as f64).into());
            set(&value, "fallback", &optional(query.fallback.as_deref()));
            value
        })
        .collect::<Array>()
        .into()
}

fn executor(options: &JsValue) -> Result<Executor, JsValue> {
    match optional_text(options, "executor", "options")? {
        None => Ok(Executor::default()),
        Some(name) => Executor::named(&name).ok_or_else(|| {
            mistaken(format!(
                "options.executor {name} is not compiled, spareval or both"
            ))
        }),
    }
}

fn positive(options: &JsValue, key: &str) -> Result<Option<f64>, JsValue> {
    let value = get(options, key)?;
    if absent(&value) {
        return Ok(None);
    }
    value
        .as_f64()
        .filter(|number| number.is_finite() && *number > 0.0)
        .map(Some)
        .ok_or_else(|| mistaken(format!("options.{key} is not a positive number")))
}

fn budgets(options: &JsValue) -> Result<Budgets, JsValue> {
    const MEBIBYTE: f64 = 1_048_576.0;
    let defaults = Budgets::default();
    let time = positive(options, "timeBudgetSeconds")?
        .map(|seconds| {
            Duration::try_from_secs_f64(seconds)
                .map_err(|_| mistaken("options.timeBudgetSeconds is too large".to_owned()))
        })
        .transpose()?;
    Ok(Budgets {
        memory_bytes: positive(options, "memoryBudgetMiB")?
            .map_or(defaults.memory_bytes, |mebibytes| {
                (mebibytes * MEBIBYTE) as usize
            }),
        time: time.unwrap_or(defaults.time),
    })
}

fn guarded() -> Result<(), JsValue> {
    guard().map_err(thrown)
}

#[wasm_bindgen]
pub fn describe(
    adapter_iri: &str,
    metadata: &[u8],
    format: Option<FormatValue>,
) -> Result<DescriptionValue, JsValue> {
    guarded()?;
    let format = self::format(format.as_deref())?;
    let described = cascade_bridge::describe(adapter_iri, metadata).map_err(thrown)?;
    let value = JsValue::from(Object::new());
    set(
        &value,
        "graph",
        &array(&described.graph(format).map_err(thrown)?),
    );
    set(&value, "iri", &described.iri.as_str().into());
    set(
        &value,
        "identifier",
        &optional(described.identifier.as_deref()),
    );
    set(&value, "version", &optional(described.version.as_deref()));
    set(
        &value,
        "sourceMediaType",
        &optional(described.source_media_type.as_deref()),
    );
    set(&value, "envelopes", &strings(&described.envelopes));
    set(&value, "loadFiles", &strings(&described.load_files));
    set(&value, "crateFiles", &strings(&described.crate_files));
    if described.vocabulary_repository.is_some() || !described.vocabulary_files.is_empty() {
        let vocabulary = JsValue::from(Object::new());
        set(
            &vocabulary,
            "repository",
            &optional(described.vocabulary_repository.as_deref()),
        );
        set(&vocabulary, "files", &strings(&described.vocabulary_files));
        set(&value, "vocabulary", &vocabulary);
    }
    Ok(value.unchecked_into())
}

#[wasm_bindgen]
pub struct Adapter {
    loaded: Loaded,
}

#[wasm_bindgen]
impl Adapter {
    pub fn load(adapter: &NamedValue, vocabulary: Option<NamedValue>) -> Result<Adapter, JsValue> {
        guarded()?;
        let adapter = named(adapter, "adapter")?;
        let vocabulary = vocabulary_named(vocabulary)?;
        let loaded = cascade_bridge::load(as_named(&adapter), vocabulary.as_ref().map(as_named))
            .map_err(thrown)?;
        Ok(Self { loaded })
    }

    pub fn accepts(&self, document: &DocumentValue) -> Result<bool, JsValue> {
        guarded()?;
        let read = DocumentRead::of(document)?;
        self.loaded.accepts(&read.document()).map_err(thrown)
    }

    pub fn convert(
        &self,
        document: &DocumentValue,
        options: Option<ConvertOptionsValue>,
    ) -> Result<ConversionValue, JsValue> {
        guarded()?;
        let read = DocumentRead::of(document)?;
        let (format, relative_to, profiled, executor, budgets) =
            match options.filter(|options| !absent(options)) {
                Some(options) => (
                    self::format(Some(&get(&options, "format")?))?,
                    optional_text(&options, "findingsRelativeTo", "options")?,
                    get(&options, "profile")?.is_truthy(),
                    self::executor(&options)?,
                    self::budgets(&options)?,
                ),
                None => (
                    Format::Turtle,
                    None,
                    false,
                    Executor::default(),
                    Budgets::default(),
                ),
            };
        let (conversion, queries) = if profiled {
            let (conversion, queries) = self
                .loaded
                .convert_profiled_with(&read.document(), executor, budgets)
                .map_err(thrown)?;
            (conversion, Some(queries))
        } else {
            (
                self.loaded
                    .convert_with(&read.document(), executor, budgets)
                    .map_err(thrown)?,
                None,
            )
        };
        let value = JsValue::from(Object::new());
        if let Some(queries) = queries {
            set(&value, "queries", &profiles(&queries));
        }
        set(
            &value,
            "graph",
            &array(&conversion.graph(format).map_err(thrown)?),
        );
        set(
            &value,
            "findings",
            &array(
                &conversion
                    .findings(format, relative_to.as_deref())
                    .map_err(thrown)?,
            ),
        );
        set(&value, "records", &(conversion.records as f64).into());
        set(&value, "triples", &(conversion.triples as f64).into());
        set(
            &value,
            "findingCount",
            &(conversion.finding_count as f64).into(),
        );
        set(
            &value,
            "detected",
            &conversion
                .detected
                .map_or(JsValue::UNDEFINED, JsValue::from_bool),
        );
        set(
            &value,
            "unvalidated",
            &optional(conversion.unvalidated.as_deref()),
        );
        Ok(value.unchecked_into())
    }
}

#[wasm_bindgen]
pub fn test(
    adapter: &NamedValue,
    vocabulary: Option<NamedValue>,
    options: Option<TestOptionsValue>,
) -> Result<TestReportValue, JsValue> {
    guarded()?;
    let adapter = named(adapter, "adapter")?;
    let vocabulary = vocabulary_named(vocabulary)?;
    let (datasets, executor, budgets) = match options.filter(|options| !absent(options)) {
        Some(options) => (
            get(&options, "datasets")?.is_truthy(),
            self::executor(&options)?,
            self::budgets(&options)?,
        ),
        None => (false, Executor::default(), Budgets::default()),
    };
    let report = cascade_bridge::test(
        as_named(&adapter),
        vocabulary.as_ref().map(as_named),
        cascade_bridge::TestOptions {
            datasets,
            executor,
            budgets,
        },
    )
    .map_err(thrown)?;
    let entries: Array = report
        .entries
        .iter()
        .map(|entry| {
            let value = JsValue::from(Object::new());
            set(&value, "name", &entry.name.as_str().into());
            set(&value, "outcome", &entry.outcome.as_str().into());
            set(&value, "description", &entry.description.as_str().into());
            set(&value, "seconds", &entry.elapsed.as_secs_f64().into());
            value
        })
        .collect();
    let bridge = JsValue::from(Object::new());
    set(&bridge, "name", &cascade_bridge::NAME.into());
    set(&bridge, "version", &cascade_bridge::VERSION.into());
    set(&bridge, "profiles", &strings(&report.profiles));
    let value = JsValue::from(Object::new());
    set(&value, "earl", &array(&report.earl));
    set(&value, "entries", &entries.into());
    set(&value, "bridge", &bridge);
    Ok(value.unchecked_into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cascade_bridge::ErrorKind;

    #[test]
    fn the_guard_refuses_as_a_fault_in_the_bridge_once_a_panic_is_recorded() {
        record_panic();
        let refused = guard().err().map(|error| error.kind().clone());
        assert_eq!(refused, Some(ErrorKind::Bridge));
    }
}
