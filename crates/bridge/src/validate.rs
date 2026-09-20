// XSD 1.0 validation of the source: every record against the adapter's
// bridge:sourceSchema, every document against its envelope's
// bridge:documentSchema. It reports; it never refuses.
//
// A schema and everything it includes are read through the host, against the
// including schema's own IRI, before the validator is built: the loader the
// validator is given answers out of what was read and can reach nothing else,
// so an xs:include the host refuses is refused here and nothing is fetched.
//
// An error's address is taken from the validator's element events and not from
// ValidationError::element_path, which holds bare local names and so cannot
// write a step for an element in a namespace.
use crate::error::{Error, Result};
use crate::lift::{Step, UTF_8_DECLARATION};
use crate::rdf::BRIDGE_SCHEMA_RULE_UNNAMED;
use crate::resolver::Resolver;
use oxiri::Iri;
use quick_xml::events::Event;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use xsd_schema::error::{SchemaError, SchemaResult};
use xsd_schema::validation::{
    drive_quick_xml_with, ElementStartView, EndElementInfo, SchemaValidator, ValidationError,
    ValidationEventHandler, ValidationFlags, ValidationSink, ValidationWarning,
};
use xsd_schema::{SchemaLoader, SchemaSet, SchemaSetBuilder};

const XSD: &str = "http://www.w3.org/2001/XMLSchema";
const DIRECTIVES: [&[u8]; 4] = [b"include", b"import", b"redefine", b"override"];

const XMLSCHEMA_1: &str = "https://www.w3.org/TR/xmlschema-1/#";
const XMLSCHEMA_2: &str = "https://www.w3.org/TR/xmlschema-2/#";

/// The rules each Recommendation defines, of every code `xsd-schema` writes.
/// A code neither names is a schema failure W3C names no rule for, so a
/// validator that grows one cannot produce a body that opens nothing.
const DEFINED_BY_STRUCTURES: [&str; 10] = [
    "cos-st-restricts",
    "cvc-assess-attr",
    "cvc-attribute",
    "cvc-complex-type",
    "cvc-elt",
    "cvc-id",
    "cvc-identity-constraint",
    "cvc-simple-type",
    "cvc-type",
    "src-resolve",
];
const DEFINED_BY_DATATYPES: [&str; 13] = [
    "cos-applicable-facets",
    "cvc-datatype-valid",
    "cvc-enumeration-valid",
    "cvc-fractionDigits-valid",
    "cvc-length-valid",
    "cvc-maxExclusive-valid",
    "cvc-maxInclusive-valid",
    "cvc-maxLength-valid",
    "cvc-minExclusive-valid",
    "cvc-minInclusive-valid",
    "cvc-minLength-valid",
    "cvc-pattern-valid",
    "cvc-totalDigits-valid",
];

pub struct Schema {
    set: SchemaSet,
}

/// One XSD error: W3C's code for the rule it broke, and where the instance
/// broke it.
pub struct SchemaFinding {
    constraint: &'static str,
    within: Option<String>,
}

impl SchemaFinding {
    /// The anchor of the rule, read from the code up to its first dot: a rule
    /// and not one of its clauses.
    pub fn body(&self) -> String {
        let rule = self
            .constraint
            .split_once('.')
            .map_or(self.constraint, |(rule, _)| rule);
        if DEFINED_BY_STRUCTURES.contains(&rule) {
            return format!("{XMLSCHEMA_1}{rule}");
        }
        if DEFINED_BY_DATATYPES.contains(&rule) {
            return format!("{XMLSCHEMA_2}{rule}");
        }
        BRIDGE_SCHEMA_RULE_UNNAMED.to_owned()
    }

    /// The element the rule was broken on, relative to the element validated,
    /// and nothing where it is that element itself.
    pub fn within(&self) -> Option<&str> {
        self.within.as_deref()
    }
}

/// Where the validator is, shared by the handler that moves it and the sink
/// that reads it: the two are separate objects, and an error arrives on one
/// while only the other knows which element drew it.
type At = Rc<RefCell<Vec<Step>>>;

/// The steps below the element validated, each indexed among its siblings of
/// its name.
fn within(at: &[Step]) -> Option<String> {
    let below = at.get(1..).unwrap_or_default();
    (!below.is_empty()).then(|| {
        below
            .iter()
            .map(|step| step.write(true))
            .collect::<Vec<String>>()
            .join("/")
    })
}

/// The element the validator is in, kept as it walks: a step is pushed before
/// the element is validated, so an error the element itself draws is addressed
/// to it and not to its parent.
struct Walked {
    at: At,
    siblings: Vec<HashMap<(String, Option<String>), usize>>,
}

impl ValidationEventHandler for Walked {
    type Error = Infallible;

    fn before_element(
        &mut self,
        view: ElementStartView<'_>,
    ) -> std::result::Result<(), Infallible> {
        let namespace = (!view.namespace_uri.is_empty()).then(|| view.namespace_uri.to_owned());
        let position = match self.siblings.last_mut() {
            Some(siblings) => {
                let seen = siblings
                    .entry((view.local_name.to_owned(), namespace.clone()))
                    .or_default();
                *seen += 1;
                *seen
            }
            None => 1,
        };
        self.siblings.push(HashMap::new());
        self.at.borrow_mut().push(Step {
            local: view.local_name.to_owned(),
            namespace,
            position,
        });
        Ok(())
    }

    fn after_end_element(
        &mut self,
        _info: &EndElementInfo,
        _depth: usize,
    ) -> std::result::Result<(), Infallible> {
        self.siblings.pop();
        self.at.borrow_mut().pop();
        Ok(())
    }
}

/// Each error the run draws, addressed to wherever the walk had reached.
struct Reported {
    at: At,
    findings: Rc<RefCell<Vec<SchemaFinding>>>,
}

impl ValidationSink for Reported {
    fn on_error(&mut self, error: ValidationError) {
        let within = within(&self.at.borrow());
        self.findings.borrow_mut().push(SchemaFinding {
            constraint: error.constraint,
            within,
        });
    }

    fn on_warning(&mut self, _warning: ValidationWarning) {}
}

impl Schema {
    /// Every XSD error the instance draws, empty when it is valid.
    pub fn errors(&self, xml: &str) -> Result<Vec<SchemaFinding>> {
        let document = utf8_declaration(xml);
        let validator = SchemaValidator::new(
            &self.set,
            ValidationFlags::default() | ValidationFlags::PROCESS_IDENTITY_CONSTRAINTS,
        );
        let at: At = Rc::default();
        let findings: Rc<RefCell<Vec<SchemaFinding>>> = Rc::default();
        let mut walked = Walked {
            at: Rc::clone(&at),
            siblings: Vec::new(),
        };
        let mut runtime = validator.start_run(Reported {
            at,
            findings: Rc::clone(&findings),
        });
        drive_quick_xml_with(document.as_bytes(), &mut runtime, &self.set, &mut walked)
            .map_err(|e| Error::msg(e.to_string()))?;
        // Driven this way the run is not ended for us, and a diagnostic an
        // end-of-document check draws is drawn here or nowhere.
        runtime
            .end_validation()
            .map_err(|e| Error::msg(e.to_string()))?;
        let broken = std::mem::take(&mut *findings.borrow_mut());
        Ok(broken)
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
/// longer carry the encoding the document was written in. A target that merely
/// begins with `xml` is another instruction's, and the document has none.
fn utf8_declaration(xml: &str) -> String {
    let declaration = xml
        .strip_prefix("<?xml")
        .filter(|rest| rest.starts_with([' ', '\t', '\r', '\n', '?']));
    let body = match declaration {
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

    #[test]
    fn keeps_a_leading_instruction_whose_target_only_begins_with_xml() {
        assert_eq!(
            utf8_declaration("<?xml-stylesheet href=\"s.xsl\"?><r/>"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><?xml-stylesheet href=\"s.xsl\"?><r/>"
        );
    }
}
