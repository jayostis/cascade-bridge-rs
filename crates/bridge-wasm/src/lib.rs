//! The engine command for a host that is not a process.
//!
//! What these functions take and return is the node host's business and
//! promises nothing to anyone else.
use cascade_bridge::command::{self, Host};
use cascade_bridge::{unread, Error, Resolver};
use std::path::Path;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// Each method that can fail throws a string saying why.
    #[derive(Clone)]
    pub type JsHost;

    /// Opens the adapter's directory for `read`, and gives its IRI.
    #[wasm_bindgen(method, catch)]
    fn adapter(this: &JsHost, directory: &str) -> Result<String, JsValue>;

    /// Opens the vocabularies checkout for `readVocabulary`, and gives its IRI.
    #[wasm_bindgen(method, catch)]
    fn vocabularies(this: &JsHost, directory: &str) -> Result<String, JsValue>;

    #[wasm_bindgen(method, catch)]
    fn read(this: &JsHost, iri: &str) -> Result<Vec<u8>, JsValue>;

    #[wasm_bindgen(method, catch, js_name = readVocabulary)]
    fn read_vocabulary(this: &JsHost, iri: &str) -> Result<Vec<u8>, JsValue>;

    #[wasm_bindgen(method, catch, js_name = readFile)]
    fn read_file(this: &JsHost, path: &str) -> Result<Vec<u8>, JsValue>;

    #[wasm_bindgen(method, catch, js_name = fileIri)]
    fn file_iri(this: &JsHost, path: &str) -> Result<String, JsValue>;

    #[wasm_bindgen(method, catch)]
    fn create(this: &JsHost, path: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch)]
    fn write(this: &JsHost, path: &str, text: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch)]
    fn out(this: &JsHost, text: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(method)]
    fn err(this: &JsHost, text: &str);
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

fn refused(thrown: JsValue) -> Error {
    Error::msg(thrown.as_string().unwrap_or_else(|| format!("{thrown:?}")))
}

fn unreadable(iri: &str, thrown: JsValue) -> Error {
    match thrown.as_string() {
        Some(reason) if reason == "ENOENT" => Error::missing(format!("{iri}: {reason}")),
        Some(reason) => unread(iri, &reason),
        None => unread(iri, &format!("{thrown:?}")),
    }
}

/// `root` and `vocabularies` name directories, each ending in "/".
struct Directories {
    root: String,
    vocabularies: Option<String>,
    files: JsHost,
}

impl Resolver for Directories {
    fn root(&self) -> &str {
        &self.root
    }

    fn vocabularies(&self) -> Option<&str> {
        self.vocabularies.as_deref()
    }

    fn read(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        self.files.read(iri).map_err(|e| unreadable(iri, e))
    }

    fn read_vocabulary(&self, iri: &str) -> cascade_bridge::Result<Vec<u8>> {
        match &self.vocabularies {
            Some(_) => self
                .files
                .read_vocabulary(iri)
                .map_err(|e| unreadable(iri, e)),
            None => Err(Error::msg(format!(
                "the command named no vocabularies: {iri}"
            ))),
        }
    }
}

impl Host for JsHost {
    fn resolver(
        &mut self,
        directory: &str,
        vocabularies: Option<&str>,
    ) -> cascade_bridge::Result<Box<dyn Resolver>> {
        let root = self.adapter(directory).map_err(refused)?;
        let vocabularies = vocabularies
            .map(|checkout| JsHost::vocabularies(self, checkout).map_err(refused))
            .transpose()?;
        Ok(Box::new(Directories {
            root,
            vocabularies,
            files: self.clone(),
        }))
    }

    fn read(&mut self, path: &str) -> cascade_bridge::Result<Vec<u8>> {
        self.read_file(path).map_err(refused)
    }

    fn file_iri(&mut self, path: &str) -> cascade_bridge::Result<String> {
        JsHost::file_iri(self, path).map_err(refused)
    }

    fn create(&mut self, path: &str) -> cascade_bridge::Result<()> {
        JsHost::create(self, path).map_err(refused)
    }

    fn write(&mut self, path: &str, text: &str) -> cascade_bridge::Result<()> {
        JsHost::write(self, path, text).map_err(refused)
    }

    fn out(&mut self, text: &str) -> cascade_bridge::Result<()> {
        JsHost::out(self, text).map_err(refused)
    }

    fn err(&mut self, text: &str) {
        JsHost::err(self, text);
    }
}

#[wasm_bindgen]
pub fn run(argv: Vec<String>, mut host: JsHost) -> u8 {
    command::run(argv, &mut host)
}

#[wasm_bindgen]
pub fn path_to_file_iri(path: &str) -> String {
    cascade_bridge::path_to_file_iri(Path::new(path))
}

#[wasm_bindgen]
pub fn file_iri_to_path(iri: &str) -> Option<String> {
    cascade_bridge::file_iri_to_path(iri).map(|path| path.to_string_lossy().into_owned())
}

#[wasm_bindgen]
pub fn authority(iri: &str) -> Option<String> {
    cascade_bridge::authority(iri).map(str::to_owned)
}
