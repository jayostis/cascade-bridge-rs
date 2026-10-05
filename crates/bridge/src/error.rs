// A harness puts an error's sentence in an entry's description: write it to be read there.
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Document,
    Facts,
    Adapter,
    Vocabulary,
    Missing { map: Map, path: String },
    Bridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Map {
    Adapter,
    Vocabulary,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn document(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Document, message)
    }

    pub(crate) fn facts(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Facts, message)
    }

    pub(crate) fn adapter(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Adapter, message)
    }

    pub(crate) fn vocabulary(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Vocabulary, message)
    }

    pub(crate) fn bridge(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Bridge, message)
    }

    pub(crate) fn missing(map: Map, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::Missing {
                map,
                path: path.into(),
            },
            message,
        )
    }

    /// An error no input explained yet is put down to the input `kind` names.
    pub(crate) fn explained_by(self, kind: ErrorKind) -> Self {
        match self.kind {
            ErrorKind::Bridge => Self { kind, ..self },
            _ => self,
        }
    }

    pub(crate) fn reworded(self, say: impl FnOnce(&str) -> String) -> Self {
        Self {
            message: say(&self.message),
            ..self
        }
    }
}

pub(crate) trait Explained<T> {
    fn explained_by(self, kind: ErrorKind) -> Result<T>;
}

impl<T> Explained<T> for Result<T> {
    fn explained_by(self, kind: ErrorKind) -> Result<T> {
        self.map_err(|error| error.explained_by(kind))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! from_error {
    ($($t:ty),* $(,)?) => {
        $(impl From<$t> for Error {
            fn from(e: $t) -> Self {
                Self::bridge(e.to_string())
            }
        })*
    };
}

from_error!(
    std::io::Error,
    std::str::Utf8Error,
    std::string::FromUtf8Error,
    quick_xml::Error,
    quick_xml::escape::EscapeError,
    quick_xml::encoding::EncodingError,
    quick_xml::events::attributes::AttrError,
    oxrdf::IriParseError,
    oxrdf::BlankNodeIdParseError,
    oxrdfio::RdfParseError,
    oxrdfio::RdfSyntaxError,
    oxigraph::store::StorageError,
    oxigraph::store::LoaderError,
    oxigraph::sparql::SparqlSyntaxError,
    oxigraph::sparql::QueryEvaluationError,
);
