// The lift of the sparql-1.1 profile: XML into Facade-X-shaped triples.
//
// A mapping never sees the document. It sees one unit, lifted with the unit
// element as the root; the detect query sees the skeleton, which is the whole
// document with every unit reduced to an empty container. One pass yields
// both, so no unit is ever lifted inside another's graph, and a unit is handed
// over as soon as its end tag is read: a release the size of a disk costs one
// unit of memory, not one document.
//
// The same pass writes each unit out again as XML, since a validator brings
// its own parser and the decoded document is behind a reader by then. It
// carries the declarations in scope where the unit stood, so a unit whose
// prefixes were bound by an ancestor still parses on its own.
//
// Triples go into the store as triples. There is no N-Triples text in
// between, so nothing is serialised on one side of a call and parsed back on
// the other.
use crate::decode::{decode, is_xml_space, normalise_attribute_value, normalise_line_endings};
use crate::error::Result;
use oxigraph::model::{BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{BufRead, Cursor};

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const FX: &str = "http://sparql.xyz/facade-x/ns/";
pub const XYZ: &str = "http://sparql.xyz/facade-x/data/";

/// The declaration a unit is written out under, and validated under: by the
/// time a unit is written its characters are characters, whatever bytes the
/// document arrived as.
pub(crate) const UTF_8_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";

/// What RFC 3987 calls `iunreserved`: the characters an IRI carries as
/// themselves. Almost every XML name character is one, so almost every name
/// reads in an IRI as it read in the document.
fn iunreserved(character: char) -> bool {
    matches!(character,
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~'
        | '\u{A0}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFEF}'
        | '\u{10000}'..='\u{1FFFD}' | '\u{20000}'..='\u{2FFFD}' | '\u{30000}'..='\u{3FFFD}'
        | '\u{40000}'..='\u{4FFFD}' | '\u{50000}'..='\u{5FFFD}' | '\u{60000}'..='\u{6FFFD}'
        | '\u{70000}'..='\u{7FFFD}' | '\u{80000}'..='\u{8FFFD}' | '\u{90000}'..='\u{9FFFD}'
        | '\u{A0000}'..='\u{AFFFD}' | '\u{B0000}'..='\u{BFFFD}' | '\u{C0000}'..='\u{CFFFD}'
        | '\u{D0000}'..='\u{DFFFD}' | '\u{E1000}'..='\u{EFFFD}')
}

/// Anything else is percent-encoded as its UTF-8 octets, which is what
/// percent-encoding is defined over, so an odd name costs a readable IRI and
/// never a parse failure. `%` is no XML name character, so no name can spell
/// another name's encoding and two names never land on one IRI.
fn name(namespace: &str, local: &str) -> Result<NamedNode> {
    let mut iri = String::with_capacity(namespace.len() + local.len());
    iri.push_str(namespace);
    for character in local.chars() {
        if iunreserved(character) {
            iri.push(character);
            continue;
        }
        let mut octets = [0; 4];
        for octet in character.encode_utf8(&mut octets).as_bytes() {
            iri.push_str(&format!("%{octet:02X}"));
        }
    }
    Ok(NamedNode::new(iri)?)
}

fn member(index: usize) -> Result<NamedNode> {
    Ok(NamedNode::new(format!("{RDF}_{index}"))?)
}

fn triple(subject: &BlankNode, predicate: NamedNode, object: impl Into<Term>) -> Quad {
    Quad::new(
        NamedOrBlankNode::from(subject.clone()),
        predicate,
        object,
        GraphName::DefaultGraph,
    )
}

/// One step of an XPath: an element, and its place among its siblings of the
/// same name. Every address this crate writes is written of these, so the
/// record a finding stood in and the element a schema rule was broken on are
/// spelled the one way.
#[derive(Clone)]
pub(crate) struct Step {
    pub(crate) local: String,
    pub(crate) namespace: Option<String>,
    pub(crate) position: usize,
}

impl Step {
    /// An XPath carries no prefix bindings and a selector is read where nothing
    /// can supply them, so a namespace is written out in full.
    pub(crate) fn write(&self, indexed: bool) -> String {
        let named = match &self.namespace {
            Some(namespace) => format!(
                "*[local-name()='{}' and namespace-uri()='{namespace}']",
                self.local
            ),
            None => self.local.clone(),
        };
        match indexed {
            true => format!("{named}[{}]", self.position),
            false => named,
        }
    }
}

/// One record, lifted, with what a stage outside the lift needs to say where
/// it stood and to hand its own parser the same characters.
pub struct Unit {
    pub store: Store,
    pub xml: String,
    path: Vec<Step>,
}

impl Unit {
    /// The XPath from the document element to this record. The document
    /// element takes no index, having no siblings to be one of.
    pub fn selector(&self) -> String {
        self.path
            .iter()
            .enumerate()
            .map(|(depth, step)| format!("/{}", step.write(depth > 0)))
            .collect()
    }
}

/// An element as the parser read it: what the lift names it by, and what the
/// document wrote it as.
struct Element<'a> {
    local: &'a str,
    namespace: Option<&'a str>,
    qname: &'a str,
    type_iri: NamedNode,
    attributes: Vec<(NamedNode, String)>,
    written: Vec<(String, String)>,
    declarations: Vec<(String, String)>,
}

struct Frame {
    id: BlankNode,
    members: usize,
    /// Whether this element is inside the unit currently being lifted.
    unit: bool,
    qname: String,
    declarations: Vec<(String, String)>,
    /// Its step, for an element the document element reaches without passing
    /// through a record.
    step: Option<Step>,
    /// How many children of each name have been opened, the next one's place
    /// among its siblings of that name being one more.
    siblings: HashMap<(String, Option<String>), usize>,
}

/// A start tag as XML, its declarations written before its attributes.
fn start_tag(
    qname: &str,
    written: &[(String, String)],
    declarations: &[(String, String)],
) -> String {
    let mut tag = format!("<{qname}");
    for (name, value) in declarations.iter().chain(written) {
        tag.push_str(&format!(" {name}=\"{}\"", value.replace('"', "&quot;")));
    }
    tag.push('>');
    tag
}

/// The element names and triples of the pass, without the reader: the parser
/// borrows its own buffer, and these fields have to stay reachable while it
/// does.
struct Builder {
    unit_name: Option<String>,
    stack: Vec<Frame>,
    text: String,
    next: usize,
    unit: Vec<Quad>,
    skeleton: Vec<Quad>,
    rdf_type: NamedNode,
    fx_root: NamedNode,
    document_element: Option<String>,
    document_step: Option<Step>,
    raw: String,
    unit_path: Vec<Step>,
}

impl Builder {
    fn new(unit_name: Option<String>) -> Result<Self> {
        Ok(Self {
            unit_name,
            stack: Vec::new(),
            text: String::new(),
            next: 0,
            unit: Vec::new(),
            skeleton: Vec::new(),
            rdf_type: NamedNode::new(format!("{RDF}type"))?,
            fx_root: NamedNode::new(format!("{FX}root"))?,
            document_element: None,
            document_step: None,
            raw: String::new(),
            unit_path: Vec::new(),
        })
    }

    fn fresh(&mut self) -> BlankNode {
        self.next += 1;
        BlankNode::new_unchecked(format!("b{}", self.next))
    }

    fn inside_unit(&self) -> bool {
        self.stack.last().is_some_and(|f| f.unit)
    }

    /// Characters of the unit, as the document wrote them.
    fn write(&mut self, xml: &str) {
        if self.inside_unit() {
            self.raw.push_str(xml);
        }
    }

    /// Every declaration the open elements bind, the innermost binding of a
    /// name winning.
    fn in_scope(&self, own: &[(String, String)]) -> Vec<(String, String)> {
        let mut scope: Vec<(String, String)> = Vec::new();
        for (name, value) in self
            .stack
            .iter()
            .flat_map(|frame| frame.declarations.iter())
            .chain(own)
        {
            match scope.iter_mut().find(|(taken, _)| taken == name) {
                Some(bound) => bound.1.clone_from(value),
                None => scope.push((name.clone(), value.clone())),
            }
        }
        scope
    }

    fn flush(&mut self) -> Result<()> {
        if let Some(top) = self.stack.last_mut() {
            if !is_xml_space(&self.text) {
                top.members += 1;
                let quad = triple(
                    &top.id,
                    member(top.members)?,
                    Literal::new_simple_literal(&self.text),
                );
                if top.unit {
                    self.unit.push(quad);
                } else {
                    self.skeleton.push(quad);
                }
            }
        }
        self.text.clear();
        Ok(())
    }

    fn open(&mut self, element: Element<'_>) -> Result<()> {
        self.flush()?;
        let rdf_type = self.rdf_type.clone();
        if self.stack.is_empty() {
            self.document_element = Some(element.local.to_owned());
        }
        let frame = |id: BlankNode, unit: bool, step: Option<Step>| Frame {
            id,
            members: 0,
            unit,
            qname: element.qname.to_owned(),
            declarations: element.declarations.clone(),
            step,
            siblings: HashMap::new(),
        };

        if self.inside_unit() {
            self.raw.push_str(&start_tag(
                element.qname,
                &element.written,
                &element.declarations,
            ));
            let id = self.fresh();
            let top = self.stack.last_mut().expect("checked above");
            top.members += 1;
            let slot = triple(&top.id, member(top.members)?, id.clone());
            self.unit.push(slot);
            self.unit.push(triple(&id, rdf_type, element.type_iri));
            for (predicate, value) in element.attributes {
                self.unit
                    .push(triple(&id, predicate, Literal::new_simple_literal(value)));
            }
            self.stack.push(frame(id, true, None));
            return Ok(());
        }

        // Outside any unit: the element belongs to the skeleton.
        let id = self.fresh();
        let name = (
            element.local.to_owned(),
            element.namespace.map(str::to_owned),
        );
        let mut position = 1;
        match self.stack.last_mut() {
            Some(top) => {
                let seen = top.siblings.entry(name).or_default();
                *seen += 1;
                position = *seen;
                top.members += 1;
                let slot = triple(&top.id, member(top.members)?, id.clone());
                self.skeleton.push(slot);
            }
            None => self
                .skeleton
                .push(triple(&id, rdf_type.clone(), self.fx_root.clone())),
        }
        self.skeleton
            .push(triple(&id, rdf_type.clone(), element.type_iri.clone()));
        let step = Step {
            local: element.local.to_owned(),
            namespace: element.namespace.map(str::to_owned),
            position,
        };
        if self.stack.is_empty() {
            self.document_step = Some(step.clone());
        }

        if self.unit_name.as_deref() == Some(element.local) {
            self.unit_path = self
                .stack
                .iter()
                .filter_map(|frame| frame.step.clone())
                .chain([step.clone()])
                .collect();
            self.raw.clear();
            self.raw.push_str(UTF_8_DECLARATION);
            let scope = self.in_scope(&element.declarations);
            self.raw
                .push_str(&start_tag(element.qname, &element.written, &scope));

            let unit_id = self.fresh();
            self.unit.clear();
            self.unit
                .push(triple(&unit_id, rdf_type.clone(), self.fx_root.clone()));
            self.unit.push(triple(&unit_id, rdf_type, element.type_iri));
            for (predicate, value) in element.attributes {
                self.unit.push(triple(
                    &unit_id,
                    predicate,
                    Literal::new_simple_literal(value),
                ));
            }
            self.stack.push(frame(unit_id, true, Some(step)));
            return Ok(());
        }

        for (predicate, value) in element.attributes {
            self.skeleton
                .push(triple(&id, predicate, Literal::new_simple_literal(value)));
        }
        self.stack.push(frame(id, false, Some(step)));
        Ok(())
    }

    /// Closes an element, and says whether the outermost unit ended here.
    fn close(&mut self) -> Result<bool> {
        self.flush()?;
        let Some(frame) = self.stack.pop() else {
            return Ok(false);
        };
        if frame.unit {
            self.raw.push_str(&format!("</{}>", frame.qname));
        }
        Ok(frame.unit && !self.inside_unit())
    }
}

/// One pass over a document, yielding a unit at a time. When the iterator ends
/// the skeleton is complete.
pub struct Lift<R: BufRead> {
    reader: NsReader<R>,
    buffer: Vec<u8>,
    builder: Builder,
    done: bool,
}

/// Lift a slice. With `unit` given, every outermost element of that local name
/// becomes its own lifted unit and an empty container in the skeleton; without
/// it, the skeleton is the whole document lifted and there are no units.
pub fn lift_slice<'a>(bytes: &'a [u8], unit: Option<&str>) -> Result<Lift<Box<dyn BufRead + 'a>>> {
    lift_text(decode(bytes)?, unit)
}

/// Lift a document already read as characters.
pub fn lift_text<'a>(
    text: Cow<'a, str>,
    unit: Option<&str>,
) -> Result<Lift<Box<dyn BufRead + 'a>>> {
    let reader: Box<dyn BufRead + 'a> = match text {
        Cow::Borrowed(text) => Box::new(Cursor::new(text.as_bytes())),
        Cow::Owned(text) => Box::new(Cursor::new(text.into_bytes())),
    };
    Lift::new(reader, unit)
}

impl<R: BufRead> Lift<R> {
    pub fn new(reader: R, unit: Option<&str>) -> Result<Self> {
        let mut reader = NsReader::from_reader(reader);
        // An empty element is an element: <e/> and <e></e> lift alike.
        reader.config_mut().expand_empty_elements = true;
        Ok(Self {
            reader,
            buffer: Vec::new(),
            builder: Builder::new(unit.map(str::to_owned))?,
            done: false,
        })
    }

    /// The local name of the document's own element, once it has been read.
    pub fn document_element(&self) -> Option<&str> {
        self.builder.document_element.as_deref()
    }

    /// The XPath that selects the document element, which a finding about the
    /// document rather than about a record carries.
    pub fn document_selector(&self) -> Option<String> {
        self.builder
            .document_step
            .as_ref()
            .map(|step| format!("/{}", step.write(false)))
    }

    /// The whole document with every unit emptied: what a detect query reads.
    /// Complete only once the units have been drained.
    pub fn into_skeleton(mut self) -> Result<Store> {
        while self.next_unit()?.is_some() {}
        store_of(self.builder.skeleton)
    }

    pub fn next_unit(&mut self) -> Result<Option<Unit>> {
        if self.done {
            return Ok(None);
        }
        loop {
            self.buffer.clear();
            let (namespace, event) = self.reader.read_resolved_event_into(&mut self.buffer)?;
            let namespace = match namespace {
                ResolveResult::Bound(ns) => Some(String::from_utf8(ns.as_ref().to_vec())?),
                _ => None,
            };
            match event {
                Event::Start(start) => {
                    let local = String::from_utf8(start.local_name().as_ref().to_vec())?;
                    let qname = String::from_utf8(start.name().as_ref().to_vec())?;
                    let type_iri = name(namespace.as_deref().unwrap_or(XYZ), &local)?;
                    let mut attributes = Vec::new();
                    let mut written = Vec::new();
                    let mut declarations = Vec::new();
                    for attribute in start.attributes() {
                        let attribute = attribute?;
                        let key = attribute.key;
                        let raw = std::str::from_utf8(&attribute.value)?.to_owned();
                        // A namespace declaration is not an attribute, but it
                        // is what makes a unit's own prefixes mean anything
                        // once the unit is read apart from its document.
                        if key.as_ref() == b"xmlns" || key.as_ref().starts_with(b"xmlns:") {
                            declarations.push((std::str::from_utf8(key.as_ref())?.to_owned(), raw));
                            continue;
                        }
                        written.push((std::str::from_utf8(key.as_ref())?.to_owned(), raw));
                        let (resolved, local) = self.reader.resolve_attribute(key);
                        let namespace = match resolved {
                            ResolveResult::Bound(ns) => {
                                Some(String::from_utf8(ns.as_ref().to_vec())?)
                            }
                            _ => None,
                        };
                        let predicate = name(
                            namespace.as_deref().unwrap_or(XYZ),
                            std::str::from_utf8(local.as_ref())?,
                        )?;
                        let raw = std::str::from_utf8(&attribute.value)?;
                        let value = quick_xml::escape::unescape(&normalise_attribute_value(raw))?
                            .into_owned();
                        attributes.push((predicate, value));
                    }
                    self.builder.open(Element {
                        local: &local,
                        namespace: namespace.as_deref(),
                        qname: &qname,
                        type_iri,
                        attributes,
                        written,
                        declarations,
                    })?;
                }
                Event::End(_) => {
                    if self.builder.close()? {
                        let quads = std::mem::take(&mut self.builder.unit);
                        return Ok(Some(Unit {
                            store: store_of(quads)?,
                            xml: std::mem::take(&mut self.builder.raw),
                            path: std::mem::take(&mut self.builder.unit_path),
                        }));
                    }
                }
                Event::Text(text) => {
                    let raw = text.into_inner();
                    let raw = std::str::from_utf8(&raw)?;
                    self.builder.write(raw);
                    let raw = normalise_line_endings(raw);
                    self.builder
                        .text
                        .push_str(&quick_xml::escape::unescape(&raw)?);
                }
                Event::CData(data) => {
                    let raw = data.into_inner();
                    let raw = std::str::from_utf8(&raw)?;
                    self.builder.write(&format!("<![CDATA[{raw}]]>"));
                    let raw = normalise_line_endings(raw);
                    self.builder.text.push_str(&raw);
                }
                // Comments, processing instructions, the document type
                // declaration and the XML declaration are dropped, and take no
                // number: text on either side of one is a single text child.
                Event::Eof => {
                    self.done = true;
                    return Ok(None);
                }
                _ => {}
            }
        }
    }
}

impl<R: BufRead> Iterator for Lift<R> {
    type Item = Result<Unit>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_unit().transpose()
    }
}

fn store_of(quads: Vec<Quad>) -> Result<Store> {
    let store = Store::new()?;
    store.extend(quads)?;
    Ok(store)
}
