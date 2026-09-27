use crate::annotation::{Record, SelectorType};
use crate::decode::decode;
use crate::error::{Error, Result};
use crate::lift::{xml, Lift, Paths};
use crate::load::Adapter;
use crate::resolver::Resolver;
use crate::validate::{xsd, Schema};
use crate::xpath;
use oxrdf::Quad;
use std::borrow::Cow;
use std::collections::BTreeSet;

/// Which lift, schema language and addressing apply to a source document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Syntax {
    Xml,
}

impl Syntax {
    /// By the type, or by its structured syntax suffix (RFC 6839); parameters aside.
    pub(crate) fn of(media_type: Option<&str>) -> Result<Self> {
        let named =
            media_type.ok_or_else(|| Error::msg("the adapter names no bridge:sourceMediaType"))?;
        let essence = named
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if matches!(essence.as_str(), "application/xml" | "text/xml") || essence.ends_with("+xml") {
            return Ok(Self::Xml);
        }
        Err(Error::msg(format!(
            "the adapter's bridge:sourceMediaType is {named}; this Bridge lifts XML: \
             application/xml, text/xml or a type with the +xml suffix"
        )))
    }

    /// What picks each record out of a document.
    pub(crate) fn records(self, adapter: &Adapter) -> Result<String> {
        match self {
            Self::Xml => adapter
                .element_name_of_each_record
                .clone()
                .ok_or_else(|| Error::msg("the adapter names no bridge:elementNameOfEachRecord")),
        }
    }

    pub(crate) fn decode(self, bytes: &[u8]) -> Result<Cow<'_, str>> {
        match self {
            Self::Xml => decode(bytes),
        }
    }

    pub(crate) fn lift<'a>(
        self,
        text: &'a str,
        records: &str,
        paths: Paths,
    ) -> Result<Box<dyn Lift + 'a>> {
        match self {
            Self::Xml => Ok(Box::new(xml::lift_text(
                Cow::Borrowed(text),
                Some(records),
                paths,
            )?)),
        }
    }

    pub(crate) fn schema(self, iri: &str, resolver: &dyn Resolver) -> Result<Box<dyn Schema>> {
        match self {
            Self::Xml => Ok(Box::new(xsd::compile(iri, resolver)?)),
        }
    }

    pub(crate) fn selector_type(self) -> SelectorType {
        match self {
            Self::Xml => SelectorType::XPath,
        }
    }

    pub(crate) fn addresses(self) -> Box<dyn Addresses> {
        match self {
            Self::Xml => Box::new(xpath::Followed::default()),
        }
    }

    pub(crate) fn respelling<'a>(self, text: &'a str) -> Box<dyn Respelling + 'a> {
        match self {
            Self::Xml => Box::new(xpath::Spelled::of(text)),
        }
    }
}

/// Follows, within the record, each address that refines a finding's record selector.
pub(crate) trait Addresses {
    /// Each address that selects other than one node, reported against `document`.
    fn unresolved(&self, document: &Record, record: &str, findings: &[Quad]) -> Result<Vec<Quad>>;
}

/// Spells alike two addresses that select one node of the whole document.
pub(crate) trait Respelling {
    fn respelled(&self, findings: Vec<Quad>) -> Respelled;
}

pub(crate) struct Respelled {
    pub(crate) findings: Vec<Quad>,
    pub(crate) missed: BTreeSet<String>,
}

#[cfg(test)]
mod tests {
    use super::Syntax;

    #[test]
    fn lifts_as_xml_an_xml_media_type_or_one_with_the_xml_suffix() {
        for named in [
            "application/xml",
            "text/xml",
            "application/hl7-sda+xml",
            "Application/XML; charset=UTF-8",
        ] {
            assert_eq!(Syntax::of(Some(named)).ok(), Some(Syntax::Xml), "{named}");
        }
    }

    #[test]
    fn refuses_a_media_type_of_no_syntax_this_bridge_lifts() {
        for named in ["text/plain", "application/xml-dtd", "application/xmlish"] {
            let refusal = Syntax::of(Some(named)).expect_err(named).to_string();
            assert!(refusal.contains(named), "{refusal}");
        }
    }

    #[test]
    fn refuses_an_adapter_that_names_no_source_media_type() {
        let refusal = Syntax::of(None).expect_err("refused").to_string();
        assert!(refusal.contains("bridge:sourceMediaType"), "{refusal}");
    }
}
