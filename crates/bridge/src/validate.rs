// XSD 1.0 validation of the source: every record against the adapter's
// bridge:sourceSchema, every document against its envelope's
// bridge:documentSchema. It reports; it never refuses.
//
// A schema and everything it includes are read through the host, against the
// including schema's own IRI, before the validator is built: the loader the
// validator is given answers out of what was read and can reach nothing else,
// so an xs:include the host refuses is refused here and nothing is fetched.
use crate::error::{Error, Result};
use crate::resolver::Resolver;
use oxiri::Iri;
use quick_xml::events::Event;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use xsd_schema::error::{SchemaError, SchemaResult};
use xsd_schema::validation::{
    drive_quick_xml, CollectingValidationSink, SchemaValidator, ValidationFlags,
};
use xsd_schema::{SchemaLoader, SchemaSet, SchemaSetBuilder};

const XSD: &str = "http://www.w3.org/2001/XMLSchema";
const DIRECTIVES: [&[u8]; 4] = [b"include", b"import", b"redefine", b"override"];

/// The XML declaration a re-serialised record carries, and the one every
/// document is handed to the validator under: the bytes are already characters
/// by the time they reach here, whatever the document said they were.
const UTF_8_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";

pub struct Schema {
    set: SchemaSet,
}

impl Schema {
    /// The message of each XSD error the instance draws, empty when it is
    /// valid.
    pub fn errors(&self, xml: &str) -> Result<Vec<String>> {
        let document = utf8_declaration(xml);
        let validator = SchemaValidator::new(
            &self.set,
            ValidationFlags::default() | ValidationFlags::PROCESS_IDENTITY_CONSTRAINTS,
        );
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        {
            let sink = CollectingValidationSink {
                errors: &mut errors,
                warnings: &mut warnings,
            };
            let mut runtime = validator.start_run(sink);
            drive_quick_xml(document.as_bytes(), &mut runtime, &self.set)
                .map_err(|e| Error::msg(e.to_string()))?;
        }
        Ok(errors.into_iter().map(|e| e.message).collect())
    }
}

/// Read a schema and everything it names, then compile it.
pub fn compile(iri: &str, resolver: &dyn Resolver) -> Result<Schema> {
    let mut read: HashMap<String, String> = HashMap::new();
    let mut pending = vec![iri.to_owned()];
    while let Some(location) = pending.pop() {
        let key = key(&location);
        if read.contains_key(&key) {
            continue;
        }
        let text = String::from_utf8(resolver.read(&location)?)?;
        let base =
            Iri::parse(location.clone()).map_err(|e| Error::msg(format!("{location}: {e}")))?;
        for named in directives(&text)? {
            let joined = base
                .resolve(&named)
                .map_err(|e| Error::msg(format!("{location} names {named}: {e}")))?;
            pending.push(joined.into_inner());
        }
        read.insert(key, text);
    }

    let primary = read
        .get(&key(iri))
        .ok_or_else(|| Error::msg(format!("{iri} was not read")))?
        .clone();
    let unanswered = Arc::new(Mutex::new(Vec::new()));
    let loader = Preloaded {
        documents: read,
        unanswered: Arc::clone(&unanswered),
    };
    let compiled = SchemaSetBuilder::with_loader(Box::new(loader))
        .add_bytes(primary.as_bytes(), iri)
        .and_then(|builder| builder.compile())
        .map_err(|e| Error::msg(format!("{iri}: {e}")))?;

    let unanswered = unanswered.lock().expect("no other thread holds the loader");
    if !unanswered.is_empty() {
        return Err(Error::msg(format!(
            "{iri} names a schema this Bridge did not read: {}",
            unanswered.join(", ")
        )));
    }
    Ok(Schema {
        set: compiled.into_schema_set(),
    })
}

/// Every schemaLocation an xs:include, xs:import, xs:redefine or xs:override
/// names, as the schema writes it.
fn directives(text: &str) -> Result<Vec<String>> {
    let mut reader = quick_xml::NsReader::from_str(text);
    reader.config_mut().expand_empty_elements = true;
    let mut named = Vec::new();
    loop {
        let (namespace, event) = reader.read_resolved_event()?;
        match event {
            Event::Start(start) => {
                if !matches!(namespace, quick_xml::name::ResolveResult::Bound(ns) if ns.as_ref() == XSD.as_bytes())
                {
                    continue;
                }
                if !DIRECTIVES.contains(&start.local_name().as_ref()) {
                    continue;
                }
                for attribute in start.attributes() {
                    let attribute = attribute?;
                    if attribute.key.as_ref() == b"schemaLocation" {
                        named.push(attribute.unescape_value()?.into_owned());
                    }
                }
            }
            Event::Eof => return Ok(named),
            _ => {}
        }
    }
}

/// One name for a location however it is spelled. `xsd-schema` resolves a
/// relative schemaLocation as a filesystem path, which collapses the empty
/// authority a file IRI carries, so the string it asks for is not the string
/// the host was given.
fn key(location: &str) -> String {
    let path = match location.split_once(':') {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("file") => rest,
        _ => return location.to_owned(),
    };
    let path = path.replace('\\', "/");
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.trim_start_matches('/').split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    format!("file:///{}", segments.join("/"))
}

/// The document with its XML declaration replaced, since the characters no
/// longer carry the encoding the document was written in.
fn utf8_declaration(xml: &str) -> String {
    let body = match xml.strip_prefix("<?xml") {
        Some(rest) => match rest.find("?>") {
            Some(end) => &rest[end + 2..],
            None => return format!("{UTF_8_DECLARATION}{xml}"),
        },
        None => xml,
    };
    format!("{UTF_8_DECLARATION}{body}")
}

/// The schemas the host supplied, and the record of anything asked for that it
/// did not: a location this answers nothing for loads nothing, and
/// `xsd-schema` treats that as non-fatal, so it is caught here instead.
struct Preloaded {
    documents: HashMap<String, String>,
    unanswered: Arc<Mutex<Vec<String>>>,
}

impl std::fmt::Debug for Preloaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the schemas the host supplied")
    }
}

impl SchemaLoader for Preloaded {
    fn load(&self, location: &str) -> SchemaResult<String> {
        match self.documents.get(&key(location)) {
            Some(text) => Ok(text.clone()),
            None => {
                self.unanswered
                    .lock()
                    .expect("no other thread holds the loader")
                    .push(location.to_owned());
                Err(SchemaError::resolution(format!(
                    "not read through the host: {location}"
                )))
            }
        }
    }

    fn can_load(&self, _location: &str) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{key, utf8_declaration};

    #[test]
    fn names_a_file_iri_the_same_however_many_slashes_its_authority_was_written_with() {
        assert_eq!(key("file:/a/b/c.xsd"), key("file:///a/b/c.xsd"));
        assert_eq!(key("file:///a/b/../c.xsd"), key("file:///a/c.xsd"));
        assert_eq!(key("urn:example:c.xsd"), "urn:example:c.xsd");
    }

    #[test]
    fn replaces_the_declaration_of_a_document_that_says_it_is_not_utf_8() {
        assert_eq!(
            utf8_declaration("<?xml version=\"1.0\" encoding=\"UTF-16\"?><r/>"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><r/>"
        );
        assert_eq!(
            utf8_declaration("<r/>"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><r/>"
        );
    }
}
