// One error type, carrying a sentence. A harness puts that sentence in an
// entry's description, so it is written to be read there and not only in a
// stack trace.
use std::fmt;

#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub fn msg(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! from_error {
    ($($t:ty),* $(,)?) => {
        $(impl From<$t> for Error {
            fn from(e: $t) -> Self {
                Self(e.to_string())
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
