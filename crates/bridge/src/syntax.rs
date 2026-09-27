use crate::annotation::{Record, SelectorType};
use crate::error::{Error, Result};
use crate::lift::{self, xml, Admission, Lift, Paths, Reading};
use crate::load::{Adapter, Envelope};
use crate::resolver::Resolver;
use crate::terms::{OA_REFINED_BY, RDF_VALUE};
use crate::validate::{json_schema, xsd, Schema};
use crate::{decode, json, xpath};
use oxrdf::{BlankNode, NamedOrBlankNode, Quad, Term};
use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};

/// Which lift, schema language and addressing apply to a source document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Syntax {
    Xml,
    Json,
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
        if essence == "application/json" || essence.ends_with("+json") {
            return Ok(Self::Json);
        }
        Err(Error::msg(format!(
            "the adapter's bridge:sourceMediaType is {named}; this Bridge lifts XML \
             (application/xml, text/xml or a type with the +xml suffix) and JSON \
             (application/json or a type with the +json suffix)"
        )))
    }

    /// The adapter's `bridge:elementNameOfEachRecord`: a JSON document is split by its
    /// envelope's path instead.
    pub(crate) fn records(self, adapter: &Adapter) -> Result<Option<String>> {
        match self {
            Self::Xml => adapter
                .element_name_of_each_record
                .clone()
                .map(Some)
                .ok_or_else(|| Error::msg("the adapter names no bridge:elementNameOfEachRecord")),
            Self::Json => Ok(None),
        }
    }

    pub(crate) fn admission(self, envelope: &Envelope) -> Result<Admission> {
        match self {
            Self::Xml => Ok(Admission {
                root: envelope.doc_root_element_name.clone(),
                ..Admission::default()
            }),
            Self::Json => {
                if let Some(path) = &envelope.json_path_of_each_record {
                    json::path(path)
                        .map_err(|e| Error::msg(format!("envelope {}: {e}", envelope.iri)))?;
                }
                Ok(Admission {
                    root: envelope.doc_root_member_name.clone(),
                    value: envelope.doc_root_member_value.clone(),
                    records: envelope.json_path_of_each_record.clone(),
                })
            }
        }
    }

    pub(crate) fn decode(self, bytes: &[u8]) -> Result<Cow<'_, str>> {
        match self {
            Self::Xml => decode::decode(bytes),
            Self::Json => json::decode(bytes).map(Cow::Borrowed),
        }
    }

    pub(crate) fn lift<'a>(
        self,
        text: &'a str,
        reading: &Reading<'_>,
        paths: Paths,
    ) -> Result<Box<dyn Lift + 'a>> {
        match self {
            Self::Xml => Ok(Box::new(xml::lift_text(
                Cow::Borrowed(text),
                reading.element,
                paths,
            )?)),
            Self::Json => Ok(Box::new(lift::json::lift(text, reading, paths)?)),
        }
    }

    pub(crate) fn schema(self, iri: &str, resolver: &dyn Resolver) -> Result<Box<dyn Schema>> {
        match self {
            Self::Xml => Ok(Box::new(xsd::compile(iri, resolver)?)),
            Self::Json => Ok(Box::new(json_schema::compile(iri, resolver)?)),
        }
    }

    pub(crate) fn selector_type(self) -> SelectorType {
        match self {
            Self::Xml => SelectorType::XPath,
            Self::Json => SelectorType::JsonPointer,
        }
    }

    pub(crate) fn addresses(self) -> Box<dyn Addresses> {
        match self {
            Self::Xml => Box::new(xpath::Followed::default()),
            Self::Json => Box::new(json::addressing::Followed),
        }
    }

    pub(crate) fn respelling<'a>(self, text: &'a str) -> Box<dyn Respelling + 'a> {
        match self {
            Self::Xml => Box::new(xpath::Spelled::of(text)),
            Self::Json => Box::new(json::addressing::Spelled::of(text)),
        }
    }
}

/// Follows, within the record, each address a findings query wrote to refine a
/// finding's record selector.
pub(crate) trait Addresses {
    /// Each address that selects other than one node, reported against `document`.
    fn unresolved(&self, document: &Record, record: &str, queried: &[Quad]) -> Result<Vec<Quad>>;
}

/// Spells alike two addresses that select one node of the whole document.
pub(crate) trait Respelling {
    fn respelled(&self, findings: Vec<Quad>) -> Respelled;
}

pub(crate) struct Respelled {
    pub(crate) findings: Vec<Quad>,
    pub(crate) missed: BTreeSet<String>,
}

/// The address each refinement of a finding's record selector writes.
pub(crate) fn refinements(findings: &[Quad]) -> BTreeSet<String> {
    let refined: HashSet<&BlankNode> = findings
        .iter()
        .filter(|quad| quad.predicate.as_str() == OA_REFINED_BY)
        .filter_map(|quad| match &quad.object {
            Term::BlankNode(node) => Some(node),
            _ => None,
        })
        .collect();
    if refined.is_empty() {
        return BTreeSet::new();
    }
    findings
        .iter()
        .filter(|quad| quad.predicate.as_str() == RDF_VALUE)
        .filter(|quad| {
            matches!(&quad.subject, NamedOrBlankNode::BlankNode(node) if refined.contains(node))
        })
        .filter_map(|quad| match &quad.object {
            Term::Literal(literal) => Some(literal.value().to_owned()),
            _ => None,
        })
        .collect()
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
    fn lifts_as_json_the_json_media_type_or_one_with_the_json_suffix() {
        for named in [
            "application/json",
            "application/fhir+json",
            "Application/JSON; charset=utf-8",
        ] {
            assert_eq!(Syntax::of(Some(named)).ok(), Some(Syntax::Json), "{named}");
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
