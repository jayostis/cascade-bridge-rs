use crate::error::Result;
use std::collections::BTreeMap;

pub type Files = BTreeMap<String, Vec<u8>>;

pub struct Named<'a> {
    pub iri: &'a str,
    pub files: &'a Files,
}

pub struct Facts<'a> {
    pub iri: &'a str,
    pub bytes: &'a [u8],
}

pub struct Document<'a> {
    pub iri: &'a str,
    pub bytes: &'a [u8],
    pub facts: Option<Facts<'a>>,
    pub envelope: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Turtle,
    NTriples,
}

#[derive(Debug, Default)]
pub struct Description {
    pub identifier: Option<String>,
    pub version: Option<String>,
    pub source_media_type: Option<String>,
    pub envelopes: Vec<String>,
    pub files_to_load: Vec<String>,
    pub files_to_test: Vec<String>,
    pub vocabulary_pin: Option<String>,
    pub vocabulary_files: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Conversion {
    pub graph: Vec<u8>,
    pub findings: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct TestReport {
    pub earl: Vec<u8>,
}

pub struct Adapter {}

pub fn describe(metadata: &[u8], adapter: &str) -> Result<Description> {
    let _ = (metadata, adapter);
    Ok(Description::default())
}

impl Adapter {
    pub fn load(adapter: Named<'_>, vocabulary: Option<Named<'_>>) -> Result<Self> {
        let _ = (adapter, vocabulary);
        Ok(Self {})
    }

    pub fn accepts(&self, document: &Document<'_>) -> Result<bool> {
        let _ = document;
        Ok(false)
    }

    pub fn convert(&self, document: &Document<'_>, format: Format) -> Result<Conversion> {
        let _ = (document, format);
        Ok(Conversion::default())
    }
}

pub fn test(adapter: Named<'_>, vocabulary: Option<Named<'_>>) -> Result<TestReport> {
    let _ = (adapter, vocabulary);
    Ok(TestReport::default())
}
