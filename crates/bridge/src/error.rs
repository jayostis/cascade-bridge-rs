// A harness puts an error's sentence in an entry's description: write it to be read there.
use std::fmt;

#[derive(Debug)]
pub struct Error {
    message: String,
    missing: bool,
}

impl Error {
    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            missing: false,
        }
    }

    /// A file asked for that does not exist, as against one refused.
    pub fn missing(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            missing: true,
        }
    }

    pub fn is_missing(&self) -> bool {
        self.missing
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => Self::missing(e.to_string()),
            _ => Self::msg(e.to_string()),
        }
    }
}

macro_rules! from_error {
    ($($t:ty),* $(,)?) => {
        $(impl From<$t> for Error {
            fn from(e: $t) -> Self {
                Self::msg(e.to_string())
            }
        })*
    };
}

from_error!(
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
