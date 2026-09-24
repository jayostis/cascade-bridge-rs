// The skeleton the detect query reads is the whole document with every unit reduced
// to an empty container. A unit is handed over as soon as its end tag is read, and
// written out again as XML with the declarations in scope where it stood, for a
// validator that brings its own parser.
use crate::decode::{decode, is_xml_space, normalise_attribute_value, normalise_line_endings};
use crate::error::Result;
use oxigraph::model::{BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Cursor};

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const FX: &str = "http://sparql.xyz/facade-x/ns/";
pub const XYZ: &str = "http://sparql.xyz/facade-x/data/";

pub(crate) const UTF_8_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";

/// RFC 3987's `iunreserved`.
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

/// `%` is no XML name character, so two names never land on one IRI.
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

/// One step of an XPath; `position` is among the siblings of the same name.
#[derive(Clone)]
pub(crate) struct Step {
    pub(crate) local: String,
    pub(crate) namespace: Option<String>,
    pub(crate) position: usize,
}

impl Step {
    /// A namespace is written out in full: a selector is read where no prefix is bound.
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

/// `within` reaches the path's first occurrence from the record, and is none for
/// an attribute of the record element.
pub(crate) struct Occurrence {
    pub(crate) path: String,
    pub(crate) within: Option<String>,
    pub(crate) count: usize,
}

pub(crate) struct Valued {
    pub(crate) path: String,
    pub(crate) value: String,
    pub(crate) within: Option<String>,
    pub(crate) count: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Paths {
    Kept { valued: HashSet<String> },
    Dropped,
}

struct Open {
    path: String,
    within: Option<String>,
    valued: bool,
    childless: bool,
}

#[derive(Default)]
struct Census {
    below: Vec<Open>,
    seen: HashMap<String, usize>,
    occurrences: Vec<Occurrence>,
    held: HashMap<(String, String), usize>,
    values: Vec<Valued>,
}

impl Census {
    fn of(record: &Step, attributes: &[Attribute], valued: &HashSet<String>) -> Self {
        let mut census = Self::default();
        let path = format!("/{}", record.write(false));
        census.attributes(&path, None, attributes, valued);
        // The record element is no path: it stands here to spell the paths below it.
        census.below.push(Open {
            path,
            within: None,
            valued: false,
            childless: true,
        });
        census
    }

    fn open(&mut self, step: Step, attributes: &[Attribute], valued: &HashSet<String>) {
        let (path, within) = {
            let parent = self.below.last_mut().expect("the record element is open");
            parent.childless = false;
            let within = match &parent.within {
                Some(above) => format!("{above}/{}", step.write(true)),
                None => step.write(true),
            };
            (format!("{}/{}", parent.path, step.write(false)), within)
        };
        self.add(path.clone(), Some(&within));
        self.attributes(&path, Some(&within), attributes, valued);
        self.below.push(Open {
            valued: valued.contains(&path),
            path,
            within: Some(within),
            childless: true,
        });
    }

    fn close(&mut self, text: Option<&str>) {
        let Some(closed) = self.below.pop() else {
            return;
        };
        if let Some(text) = text {
            self.hold(&closed.path, closed.within.as_deref(), text);
        }
    }

    fn wants_value(&self) -> bool {
        self.below
            .last()
            .is_some_and(|open| open.valued && open.childless)
    }

    fn attributes(
        &mut self,
        element: &str,
        within: Option<&str>,
        attributes: &[Attribute],
        valued: &HashSet<String>,
    ) {
        for (step, value) in attributes.iter().filter_map(Attribute::named) {
            let path = format!("{element}/@{}", step.write(false));
            if valued.contains(&path) {
                self.hold(&path, within, value);
            }
            self.add(path, within);
        }
    }

    fn hold(&mut self, path: &str, within: Option<&str>, value: &str) {
        let held = (path.to_owned(), value.to_owned());
        match self.held.get(&held) {
            Some(&first) => self.values[first].count += 1,
            None => {
                self.held.insert(held, self.values.len());
                self.values.push(Valued {
                    path: path.to_owned(),
                    value: value.to_owned(),
                    within: within.map(str::to_owned),
                    count: 1,
                });
            }
        }
    }

    fn add(&mut self, path: String, within: Option<&str>) {
        match self.seen.get(&path) {
            Some(&first) => self.occurrences[first].count += 1,
            None => {
                self.seen.insert(path.clone(), self.occurrences.len());
                self.occurrences.push(Occurrence {
                    path,
                    within: within.map(str::to_owned),
                    count: 1,
                });
            }
        }
    }
}

pub struct Unit {
    pub store: Store,
    pub xml: String,
    path: Vec<Step>,
    occurrences: Vec<Occurrence>,
    values: Vec<Valued>,
}

impl Unit {
    pub fn selector(&self) -> String {
        self.path
            .iter()
            .enumerate()
            .map(|(depth, step)| format!("/{}", step.write(depth > 0)))
            .collect()
    }

    pub(crate) fn occurrences(&self) -> &[Occurrence] {
        &self.occurrences
    }

    pub(crate) fn values(&self) -> &[Valued] {
        &self.values
    }
}

struct Element<'a> {
    local: &'a str,
    namespace: Option<&'a str>,
    qname: &'a str,
    type_iri: NamedNode,
    attributes: Vec<Attribute>,
    written: Vec<(String, String)>,
    declarations: Vec<(String, String)>,
}

/// `step` is spelled only by a lift that keeps paths, the one reader of it.
struct Attribute {
    predicate: NamedNode,
    value: String,
    step: Option<Step>,
}

impl Attribute {
    fn named(&self) -> Option<(&Step, &str)> {
        Some((self.step.as_ref()?, self.value.as_str()))
    }
}

struct Frame {
    id: BlankNode,
    members: usize,
    /// Whether this element is inside the unit currently being lifted.
    unit: bool,
    qname: String,
    declarations: Vec<(String, String)>,
    /// Only for an element reached from the document element without passing a record.
    step: Option<Step>,
    siblings: HashMap<(String, Option<String>), usize>,
}

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

/// Apart from the reader, whose buffer the parser borrows while these stay reachable.
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
    paths: Paths,
    census: Option<Census>,
}

impl Builder {
    fn new(unit_name: Option<String>, paths: Paths) -> Result<Self> {
        Ok(Self {
            unit_name,
            paths,
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
            census: None,
        })
    }

    fn fresh(&mut self) -> BlankNode {
        self.next += 1;
        BlankNode::new_unchecked(format!("b{}", self.next))
    }

    fn inside_unit(&self) -> bool {
        self.stack.last().is_some_and(|f| f.unit)
    }

    fn keeps_paths(&self) -> bool {
        matches!(self.paths, Paths::Kept { .. })
    }

    fn write(&mut self, xml: &str) {
        if self.inside_unit() {
            self.raw.push_str(xml);
        }
    }

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

        let name = (
            element.local.to_owned(),
            element.namespace.map(str::to_owned),
        );
        let position = match self.stack.last_mut() {
            Some(top) => {
                let seen = top.siblings.entry(name).or_default();
                *seen += 1;
                *seen
            }
            None => 1,
        };
        let step = Step {
            local: element.local.to_owned(),
            namespace: element.namespace.map(str::to_owned),
            position,
        };

        if self.inside_unit() {
            self.raw.push_str(&start_tag(
                element.qname,
                &element.written,
                &element.declarations,
            ));
            if let (Some(census), Paths::Kept { valued }) = (&mut self.census, &self.paths) {
                census.open(step.clone(), &element.attributes, valued);
            }
            let id = self.fresh();
            let top = self.stack.last_mut().expect("checked above");
            top.members += 1;
            let slot = triple(&top.id, member(top.members)?, id.clone());
            self.unit.push(slot);
            self.unit.push(triple(&id, rdf_type, element.type_iri));
            for attribute in element.attributes {
                self.unit.push(triple(
                    &id,
                    attribute.predicate,
                    Literal::new_simple_literal(attribute.value),
                ));
            }
            self.stack.push(frame(id, true, Some(step)));
            return Ok(());
        }

        let id = self.fresh();
        match self.stack.last_mut() {
            Some(top) => {
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
            let census = match &self.paths {
                Paths::Kept { valued } => Some(Census::of(&step, &element.attributes, valued)),
                Paths::Dropped => None,
            };
            self.census = census;
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
            for attribute in element.attributes {
                self.unit.push(triple(
                    &unit_id,
                    attribute.predicate,
                    Literal::new_simple_literal(attribute.value),
                ));
            }
            self.stack.push(frame(unit_id, true, Some(step)));
            return Ok(());
        }

        for attribute in element.attributes {
            self.skeleton.push(triple(
                &id,
                attribute.predicate,
                Literal::new_simple_literal(attribute.value),
            ));
        }
        self.stack.push(frame(id, false, Some(step)));
        Ok(())
    }

    /// Whether the outermost unit ended here.
    fn close(&mut self) -> Result<bool> {
        let value = self
            .census
            .as_ref()
            .is_some_and(Census::wants_value)
            .then(|| self.text.clone());
        self.flush()?;
        let Some(frame) = self.stack.pop() else {
            return Ok(false);
        };
        if frame.unit {
            self.raw.push_str(&format!("</{}>", frame.qname));
            if let Some(census) = &mut self.census {
                census.close(value.as_deref());
            }
        }
        Ok(frame.unit && !self.inside_unit())
    }
}

pub struct Lift<R: BufRead> {
    reader: NsReader<R>,
    buffer: Vec<u8>,
    builder: Builder,
    done: bool,
}

/// Every outermost element named `unit` is lifted on its own; with none, the
/// skeleton is the whole document.
pub fn lift_slice<'a>(bytes: &'a [u8], unit: Option<&str>) -> Result<Lift<Box<dyn BufRead + 'a>>> {
    lift_text(decode(bytes)?, unit, Paths::Dropped)
}

pub fn lift_text<'a>(
    text: Cow<'a, str>,
    unit: Option<&str>,
    paths: Paths,
) -> Result<Lift<Box<dyn BufRead + 'a>>> {
    let reader: Box<dyn BufRead + 'a> = match text {
        Cow::Borrowed(text) => Box::new(Cursor::new(text.as_bytes())),
        Cow::Owned(text) => Box::new(Cursor::new(text.into_bytes())),
    };
    Lift::new(reader, unit, paths)
}

impl<R: BufRead> Lift<R> {
    pub fn new(reader: R, unit: Option<&str>, paths: Paths) -> Result<Self> {
        let mut reader = NsReader::from_reader(reader);
        reader.config_mut().expand_empty_elements = true;
        Ok(Self {
            reader,
            buffer: Vec::new(),
            builder: Builder::new(unit.map(str::to_owned), paths)?,
            done: false,
        })
    }

    pub fn document_element(&self) -> Option<&str> {
        self.builder.document_element.as_deref()
    }

    pub fn document_selector(&self) -> Option<String> {
        self.builder
            .document_step
            .as_ref()
            .map(|step| format!("/{}", step.write(false)))
    }

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
                    let kept = self.builder.keeps_paths();
                    let mut written = Vec::new();
                    let mut declarations = Vec::new();
                    for attribute in start.attributes() {
                        let attribute = attribute?;
                        let key = attribute.key;
                        let raw = std::str::from_utf8(&attribute.value)?.to_owned();
                        // A declaration is no attribute, but a unit read apart from
                        // its document needs it.
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
                        let local = std::str::from_utf8(local.as_ref())?;
                        let predicate = name(namespace.as_deref().unwrap_or(XYZ), local)?;
                        let raw = std::str::from_utf8(&attribute.value)?;
                        let value = quick_xml::escape::unescape(&normalise_attribute_value(raw))?
                            .into_owned();
                        let step = kept.then(|| Step {
                            local: local.to_owned(),
                            namespace: namespace.clone(),
                            position: 1,
                        });
                        attributes.push(Attribute {
                            predicate,
                            value,
                            step,
                        });
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
                        let (occurrences, values) = self
                            .builder
                            .census
                            .take()
                            .map(|census| (census.occurrences, census.values))
                            .unwrap_or_default();
                        return Ok(Some(Unit {
                            store: store_of(quads)?,
                            xml: std::mem::take(&mut self.builder.raw),
                            path: std::mem::take(&mut self.builder.unit_path),
                            occurrences,
                            values,
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
                // No part of the graph, so text on either side is one text child; but XPath
                // counts them, so a unit written out again carries them.
                Event::Comment(comment) => {
                    let raw = comment.into_inner();
                    let raw = std::str::from_utf8(&raw)?;
                    self.builder.write(&format!("<!--{raw}-->"));
                }
                Event::PI(instruction) => {
                    let raw = instruction.into_inner();
                    let raw = std::str::from_utf8(&raw)?;
                    self.builder.write(&format!("<?{raw}?>"));
                }
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
