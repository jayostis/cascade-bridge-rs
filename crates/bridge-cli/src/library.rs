use cascade_bridge::{
    describe, load, test, Document, ErrorKind, Facts, Files, Format, Loaded, Map, Named,
    TestOptions,
};
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, String> {
    value
        .get(name)
        .ok_or_else(|| format!("the calls file gives no {name} in {value}"))
}

fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    field(value, name)?
        .as_str()
        .ok_or_else(|| format!("the calls file's {name} is not a string: {value}"))
}

fn format(value: &Value) -> Result<Format, String> {
    let named = text(value, "format")?;
    Format::named(named).ok_or_else(|| format!("the calls file names the format {named}"))
}

#[derive(Clone)]
struct MapRead {
    iri: String,
    files: Files,
}

impl MapRead {
    fn of(value: &Value) -> Result<Self, String> {
        let mut files = Files::new();
        let listed = field(value, "files")?
            .as_object()
            .ok_or_else(|| format!("a map's files are not an object: {value}"))?;
        for (key, path) in listed {
            let path = path
                .as_str()
                .ok_or_else(|| format!("{key} is given no path"))?;
            files.insert(key.clone(), read(path)?);
        }
        Ok(Self {
            iri: text(value, "iri")?.to_owned(),
            files,
        })
    }

    fn named(&self) -> Named<'_> {
        Named {
            iri: &self.iri,
            files: &self.files,
        }
    }
}

#[derive(Clone)]
struct Maps {
    adapter: MapRead,
    vocabulary: Option<MapRead>,
}

impl Maps {
    fn of(value: &Value) -> Result<Self, String> {
        Ok(Self {
            adapter: MapRead::of(field(value, "adapter")?)?,
            vocabulary: value.get("vocabulary").map(MapRead::of).transpose()?,
        })
    }

    fn load(&self) -> Answer<Loaded> {
        guarded(|| {
            load(
                self.adapter.named(),
                self.vocabulary.as_ref().map(MapRead::named),
            )
        })
    }
}

struct DocumentRead {
    iri: String,
    bytes: Vec<u8>,
    envelope: Option<String>,
    facts: Option<(String, Vec<u8>)>,
}

impl DocumentRead {
    fn of(value: &Value) -> Result<Self, String> {
        let document = field(value, "document")?;
        Ok(Self {
            iri: text(document, "iri")?.to_owned(),
            bytes: read(text(document, "path")?)?,
            envelope: document
                .get("envelope")
                .map(|envelope| {
                    envelope
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| format!("an envelope that is not a string: {envelope}"))
                })
                .transpose()?,
            facts: document
                .get("facts")
                .map(|facts| {
                    Ok::<_, String>((text(facts, "iri")?.to_owned(), read(text(facts, "path")?)?))
                })
                .transpose()?,
        })
    }

    fn document(&self) -> Document<'_> {
        Document {
            iri: &self.iri,
            bytes: &self.bytes,
            envelope: self.envelope.as_deref(),
            facts: self.facts.as_ref().map(|(iri, bytes)| Facts { iri, bytes }),
        }
    }
}

type Answer<T> = Result<T, Value>;

/// A panic is a failure of the Bridge itself, as every other way the Bridge fails is.
fn guarded<T>(call: impl FnOnce() -> cascade_bridge::Result<T>) -> Answer<T> {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(Ok(answer)) => Ok(answer),
        Ok(Err(error)) => Err(failure(error.kind(), error.message())),
        Err(panic) => {
            let said = panic
                .downcast_ref::<&str>()
                .map(|said| (*said).to_owned())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            Err(failure(&ErrorKind::Bridge, &said))
        }
    }
}

fn failure(kind: &ErrorKind, message: &str) -> Value {
    let mut failed = json!({ "message": message });
    let named = match kind {
        ErrorKind::Document => "documentFailure",
        ErrorKind::Facts => "factsFailure",
        ErrorKind::Adapter => "adapterFailure",
        ErrorKind::Vocabulary => "vocabularyFailure",
        ErrorKind::Missing { map, path } => {
            failed["map"] = json!(match map {
                Map::Adapter => "adapter",
                Map::Vocabulary => "vocabulary",
            });
            failed["path"] = json!(path);
            "fileMissingFailure"
        }
        ErrorKind::Bridge => "bridgeFailure",
    };
    failed["kind"] = json!(named);
    json!({ "failure": failed })
}

fn faulted(answer: &Value) -> bool {
    answer["failure"]["kind"] == "bridgeFailure"
}

/// The latest load of a case that did not fail, and the adapter it loaded, while it stands.
#[derive(Default)]
struct Case {
    maps: Option<Maps>,
    loaded: Option<Loaded>,
}

impl Case {
    fn loaded(&mut self) -> Option<Answer<&Loaded>> {
        let maps = self.maps.as_ref()?;
        if self.loaded.is_none() {
            match maps.load() {
                Ok(loaded) => self.loaded = Some(loaded),
                Err(failed) => return Some(Err(failed)),
            }
        }
        self.loaded.as_ref().map(Ok)
    }
}

struct Written {
    directory: PathBuf,
    n: usize,
}

impl Written {
    fn put(&self, extension: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.directory.join(format!("{}.{extension}", self.n));
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    fn answer(&self, answer: &Value) -> Result<(), String> {
        self.put("json", format!("{answer}\n").as_bytes())
    }
}

fn call(case: &mut Case, call: &Value, written: &Written) -> Result<(), String> {
    let (operation, arguments) = call
        .as_object()
        .and_then(|call| call.iter().next())
        .ok_or_else(|| format!("a call that names no operation: {call}"))?;
    let answer: Answer<Value> = match operation.as_str() {
        "describe" => {
            let metadata = read(text(arguments, "metadata")?)?;
            let format = format(arguments)?;
            let adapter = text(arguments, "adapter")?;
            match guarded(|| describe(adapter, &metadata)?.graph(format)) {
                Ok(graph) => {
                    written.put("graph", &graph)?;
                    Ok(json!({}))
                }
                Err(failed) => Err(failed),
            }
        }
        "load" => {
            let maps = Maps::of(arguments)?;
            maps.load().map(|loaded| {
                case.maps = Some(maps);
                case.loaded = Some(loaded);
                json!({})
            })
        }
        "ask" => {
            let document = DocumentRead::of(arguments)?;
            let Some(loaded) = case.loaded() else {
                return Ok(());
            };
            loaded
                .and_then(|loaded| guarded(|| loaded.accepts(&document.document())))
                .map(|answer| json!({ "answer": answer }))
        }
        "convert" => {
            let document = DocumentRead::of(arguments)?;
            let format = format(arguments)?;
            let Some(loaded) = case.loaded() else {
                return Ok(());
            };
            let converted = loaded.and_then(|loaded| {
                guarded(|| {
                    let conversion = loaded.convert(&document.document())?;
                    let findings = match conversion.unvalidated {
                        Some(_) => None,
                        None => Some(conversion.findings(format, None)?),
                    };
                    Ok((conversion.graph(format)?, findings))
                })
            });
            match converted {
                Ok((graph, findings)) => {
                    written.put("graph", &graph)?;
                    if let Some(findings) = findings {
                        written.put("findings", &findings)?;
                    }
                    Ok(json!({}))
                }
                Err(failed) => Err(failed),
            }
        }
        "test" => {
            let maps = Maps::of(arguments)?;
            let tested = guarded(|| {
                test(
                    maps.adapter.named(),
                    maps.vocabulary.as_ref().map(MapRead::named),
                    TestOptions::default(),
                )
            });
            match tested {
                Ok(report) => {
                    written.put("report", &report.earl)?;
                    Ok(json!({}))
                }
                Err(failed) => Err(failed),
            }
        }
        other => return Err(format!("the calls file names the operation {other}")),
    };
    let answer = answer.unwrap_or_else(|failed| {
        if faulted(&failed) {
            case.loaded = None;
        }
        failed
    });
    written.answer(&answer)
}

/// Makes each call the calls file lists and writes what each returned under `results`.
pub fn run(calls: &str, results: &str) -> Result<u8, String> {
    let listed: Value =
        serde_json::from_slice(&read(calls)?).map_err(|e| format!("{calls}: {e}"))?;
    let cases = field(&listed, "cases")?
        .as_array()
        .ok_or_else(|| format!("{calls}: its cases are not an array"))?;
    for listed in cases {
        let name = text(listed, "name")?;
        let directory = Path::new(results).join(name);
        std::fs::create_dir_all(&directory).map_err(|e| format!("{}: {e}", directory.display()))?;
        let mut case = Case::default();
        let calls = field(listed, "calls")?
            .as_array()
            .ok_or_else(|| format!("{name}: its calls are not an array"))?;
        for (at, listed) in calls.iter().enumerate() {
            let written = Written {
                directory: directory.clone(),
                n: at + 1,
            };
            call(&mut case, listed, &written)?;
        }
    }
    Ok(0)
}
