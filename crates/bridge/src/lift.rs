// The lift of the sparql-1.1 profile: XML into Facade-X-shaped triples.
//
// A mapping never sees the document. It sees one unit, lifted with the unit
// element as the root; the detect query sees the skeleton, which is the whole
// document with every unit reduced to an empty container. One pass yields
// both, so no unit is ever lifted inside another's graph, and a unit is handed
// over as soon as its end tag is read: a release the size of a disk costs one
// unit of memory, not one document.
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
use std::io::{BufRead, Cursor};

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const FX: &str = "http://sparql.xyz/facade-x/ns/";
pub const XYZ: &str = "http://sparql.xyz/facade-x/data/";

/// A name that is not a valid IRI character sequence is percent-encoded, so an
/// odd local name costs a readable IRI and never a parse failure. The
/// specification leaves the IRI of a name outside ASCII unspecified; this is
/// the same spelling the Cascade Bridge for JavaScript produces, so the two
/// Bridges agree until it is settled.
fn name(namespace: &str, local: &str) -> Result<NamedNode> {
    let mut iri = String::with_capacity(namespace.len() + local.len());
    iri.push_str(namespace);
    for unit in local.encode_utf16() {
        match u8::try_from(unit) {
            Ok(b @ (b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-')) => {
                iri.push(char::from(b))
            }
            _ => iri.push_str(&format!("%{unit:02X}")),
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

struct Frame {
    id: BlankNode,
    members: usize,
    /// Whether this element is inside the unit currently being lifted.
    unit: bool,
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
        })
    }

    fn fresh(&mut self) -> BlankNode {
        self.next += 1;
        BlankNode::new_unchecked(format!("b{}", self.next))
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

    fn open(
        &mut self,
        local: &str,
        element: NamedNode,
        attributes: Vec<(NamedNode, String)>,
    ) -> Result<()> {
        self.flush()?;
        let rdf_type = self.rdf_type.clone();

        if self.stack.last().is_some_and(|f| f.unit) {
            let id = self.fresh();
            let top = self.stack.last_mut().expect("checked above");
            top.members += 1;
            let slot = triple(&top.id, member(top.members)?, id.clone());
            self.unit.push(slot);
            self.unit.push(triple(&id, rdf_type, element));
            for (predicate, value) in attributes {
                self.unit
                    .push(triple(&id, predicate, Literal::new_simple_literal(value)));
            }
            self.stack.push(Frame {
                id,
                members: 0,
                unit: true,
            });
            return Ok(());
        }

        // Outside any unit: the element belongs to the skeleton.
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
            .push(triple(&id, rdf_type.clone(), element.clone()));

        if self.unit_name.as_deref() == Some(local) {
            let unit_id = self.fresh();
            self.unit.clear();
            self.unit
                .push(triple(&unit_id, rdf_type.clone(), self.fx_root.clone()));
            self.unit.push(triple(&unit_id, rdf_type, element));
            for (predicate, value) in attributes {
                self.unit.push(triple(
                    &unit_id,
                    predicate,
                    Literal::new_simple_literal(value),
                ));
            }
            self.stack.push(Frame {
                id: unit_id,
                members: 0,
                unit: true,
            });
            return Ok(());
        }

        for (predicate, value) in attributes {
            self.skeleton
                .push(triple(&id, predicate, Literal::new_simple_literal(value)));
        }
        self.stack.push(Frame {
            id,
            members: 0,
            unit: false,
        });
        Ok(())
    }

    /// Closes an element, and says whether the outermost unit ended here.
    fn close(&mut self) -> Result<bool> {
        self.flush()?;
        let Some(frame) = self.stack.pop() else {
            return Ok(false);
        };
        Ok(frame.unit && !self.stack.last().is_some_and(|f| f.unit))
    }
}

/// One pass over a document, yielding a store per unit. When the iterator ends
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
    let reader: Box<dyn BufRead + 'a> = match decode(bytes)? {
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

    /// The whole document with every unit emptied: what a detect query reads.
    /// Complete only once the units have been drained.
    pub fn into_skeleton(mut self) -> Result<Store> {
        while self.next_unit()?.is_some() {}
        store_of(self.builder.skeleton)
    }

    pub fn next_unit(&mut self) -> Result<Option<Store>> {
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
                    let element = name(namespace.as_deref().unwrap_or(XYZ), &local)?;
                    let mut attributes = Vec::new();
                    for attribute in start.attributes() {
                        let attribute = attribute?;
                        let key = attribute.key;
                        // A namespace declaration is not an attribute.
                        if key.as_ref() == b"xmlns" || key.as_ref().starts_with(b"xmlns:") {
                            continue;
                        }
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
                    self.builder.open(&local, element, attributes)?;
                }
                Event::End(_) => {
                    if self.builder.close()? {
                        let quads = std::mem::take(&mut self.builder.unit);
                        return Ok(Some(store_of(quads)?));
                    }
                }
                Event::Text(text) => {
                    let raw = text.into_inner();
                    let raw = normalise_line_endings(std::str::from_utf8(&raw)?);
                    self.builder
                        .text
                        .push_str(&quick_xml::escape::unescape(&raw)?);
                }
                Event::CData(data) => {
                    let raw = data.into_inner();
                    let raw = normalise_line_endings(std::str::from_utf8(&raw)?);
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
    type Item = Result<Store>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_unit().transpose()
    }
}

fn store_of(quads: Vec<Quad>) -> Result<Store> {
    let store = Store::new()?;
    store.extend(quads)?;
    Ok(store)
}
