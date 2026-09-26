use crate::error::Result;
use oxiri::Iri;
use oxrdf::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
use oxrdf::{Dataset, NamedNode, NamedOrBlankNode, Quad, Term};
use oxrdfio::{RdfFormat, RdfSerializer};
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap, HashSet};

pub(crate) use crate::terms::{OA_HAS_SELECTOR, OA_REFINED_BY, RDF_VALUE};

pub(crate) const FINDINGS_PREFIXES: [(&str, &str); 4] = [
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("oa", "http://www.w3.org/ns/oa#"),
    ("sh", "http://www.w3.org/ns/shacl#"),
    ("bridge", "https://ns.cascadeprotocol.org/bridge/v1-draft#"),
];

pub(crate) fn canonical_lines(quads: impl IntoIterator<Item = Quad>) -> Result<BTreeSet<String>> {
    let mut dataset = Dataset::new();
    for quad in quads {
        dataset.insert(&quad);
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
        hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
    });
    Ok(dataset.iter().map(|quad| quad.to_string()).collect())
}

#[derive(Default)]
struct Joined(HashMap<String, String>);

impl Joined {
    fn root(&mut self, node: &str) -> String {
        let mut at = node.to_owned();
        while let Some(up) = self.0.get(&at) {
            if up == &at {
                break;
            }
            at = up.clone();
        }
        self.0.insert(node.to_owned(), at.clone());
        at
    }

    fn join(&mut self, one: &str, two: &str) {
        let (one, two) = (self.root(one), self.root(two));
        if one != two {
            self.0.insert(one, two);
        }
    }
}

fn blank_of(quad: &Quad) -> Option<&str> {
    match (&quad.subject, &quad.object) {
        (NamedOrBlankNode::BlankNode(node), _) => Some(node.as_str()),
        (_, Term::BlankNode(node)) => Some(node.as_str()),
        _ => None,
    }
}

/// Each part of the graph no blank node reaches out of, canonicalised on its own
/// as one line: two graphs are isomorphic exactly when these multisets are equal.
pub(crate) fn canonical_parts(quads: impl IntoIterator<Item = Quad>) -> Result<Vec<String>> {
    let quads: HashSet<Quad> = quads.into_iter().collect();
    let mut joined = Joined::default();
    for quad in &quads {
        if let (NamedOrBlankNode::BlankNode(subject), Term::BlankNode(object)) =
            (&quad.subject, &quad.object)
        {
            joined.join(subject.as_str(), object.as_str());
        }
    }

    let mut parts: HashMap<String, Vec<Quad>> = HashMap::new();
    for quad in quads {
        let key = match blank_of(&quad) {
            Some(node) => format!("_:{}", joined.root(node)),
            None => quad.to_string(),
        };
        parts.entry(key).or_default().push(quad);
    }

    let mut written: Vec<String> = Vec::with_capacity(parts.len());
    for part in parts.into_values() {
        written.push(
            canonical_lines(part)?
                .into_iter()
                .collect::<Vec<String>>()
                .join(" "),
        );
    }
    written.sort();
    Ok(written)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphFormat {
    Turtle,
    NTriples,
}

impl GraphFormat {
    pub(crate) fn named(name: &str) -> Option<Self> {
        match name {
            "turtle" => Some(Self::Turtle),
            "ntriples" => Some(Self::NTriples),
            _ => None,
        }
    }

    fn format(self) -> RdfFormat {
        match self {
            Self::Turtle => RdfFormat::Turtle,
            Self::NTriples => RdfFormat::NTriples,
        }
    }
}

fn iris(quad: &Quad) -> [Option<&str>; 3] {
    [
        match &quad.subject {
            NamedOrBlankNode::NamedNode(n) => Some(n.as_str()),
            NamedOrBlankNode::BlankNode(_) => None,
        },
        Some(quad.predicate.as_str()),
        match &quad.object {
            Term::NamedNode(n) => Some(n.as_str()),
            Term::Literal(l) => Some(l.datatype().as_str()),
            Term::BlankNode(_) => None,
            Term::Triple(_) => None,
        },
    ]
}

fn namespaces(quads: &[Quad]) -> HashSet<&str> {
    let mut namespaces = HashSet::new();
    for iri in quads.iter().flat_map(iris).flatten() {
        if let Some(cut) = iri.rfind('#').or_else(|| iri.rfind('/')) {
            namespaces.insert(&iri[..=cut]);
        }
    }
    namespaces
}

pub(crate) fn serialise(
    quads: &[Quad],
    format: GraphFormat,
    prefixes: &[(String, String)],
) -> Result<String> {
    serialise_at(quads, format, prefixes, None)
}

fn relative_to(base: &Iri<&str>, iri: &str) -> Option<String> {
    let target = Iri::parse(iri).ok()?;
    if target.scheme() != base.scheme()
        || target.authority() != base.authority()
        || [
            target.query(),
            target.fragment(),
            base.query(),
            base.fragment(),
        ]
        .iter()
        .any(Option::is_some)
    {
        return None;
    }
    // A reference resolves against the file's directory, not the file.
    let mut here: Vec<&str> = base.path().split('/').collect();
    here.pop()?;
    let there: Vec<&str> = target.path().split('/').collect();
    // A reader removes dot segments again, so a carried step would name another file.
    let dotted = |steps: &[&str]| steps.iter().any(|step| matches!(*step, "." | ".."));
    if dotted(&here) || dotted(&there) {
        return None;
    }
    let shared = here
        .iter()
        .zip(&there)
        .take_while(|(here, there)| here == there)
        .count();
    let down = &there[shared..];
    // An empty step would read as "//", an authority; a colon in the first
    // step would read as a scheme.
    if down.is_empty() || down.iter().any(|step| step.is_empty()) || down[0].contains(':') {
        return None;
    }
    Some(format!(
        "{}{}",
        "../".repeat(here.len() - shared),
        down.join("/")
    ))
}

fn relative(quads: &[Quad], base: &Iri<&str>) -> Vec<Quad> {
    let named = |node: &NamedNode| match relative_to(base, node.as_str()) {
        Some(reference) => NamedNode::new_unchecked(reference),
        None => node.clone(),
    };
    quads
        .iter()
        .map(|quad| {
            Quad::new(
                match &quad.subject {
                    NamedOrBlankNode::NamedNode(node) => NamedOrBlankNode::from(named(node)),
                    blank => blank.clone(),
                },
                named(&quad.predicate),
                match &quad.object {
                    Term::NamedNode(node) => Term::from(named(node)),
                    other => other.clone(),
                },
                quad.graph_name.clone(),
            )
        })
        .collect()
}

/// Every IRI the file standing at `at` can name relative to itself is named that way.
pub(crate) fn serialise_at(
    quads: &[Quad],
    format: GraphFormat,
    prefixes: &[(String, String)],
    at: Option<&str>,
) -> Result<String> {
    let named = match (at, format) {
        (Some(at), GraphFormat::Turtle) => Cow::Owned(relative(quads, &Iri::parse(at)?)),
        _ => Cow::Borrowed(quads),
    };
    let namespaces = namespaces(&named);
    let mut serializer = RdfSerializer::from_format(format.format());
    for (prefix, namespace) in prefixes {
        if namespaces.contains(namespace.as_str()) {
            serializer = serializer.with_prefix(prefix, namespace)?;
        }
    }
    let mut serializer = serializer.for_writer(Vec::new());
    let mut written: HashSet<&Quad> = HashSet::new();
    for quad in named.iter() {
        if written.insert(quad) {
            serializer.serialize_quad(quad)?;
        }
    }
    Ok(String::from_utf8(serializer.finish()?)?)
}

#[cfg(test)]
mod serialising;
#[cfg(test)]
mod tests {
    use super::relative_to;
    use oxiri::Iri;

    const ORACLE: &str = "file:///checkout/fixtures/findings/two.ttl";

    fn named(iri: &str) -> Option<String> {
        relative_to(&Iri::parse(ORACLE).expect("the file's own IRI"), iri)
    }

    fn resolves_back_to(iri: &str) {
        let Some(reference) = named(iri) else {
            return;
        };
        let base = Iri::parse(ORACLE).expect("the file's own IRI");
        assert_eq!(
            base.resolve(&reference)
                .unwrap_or_else(|e| panic!("{iri} named {reference}: {e}"))
                .as_str(),
            iri,
            "{iri} named {reference}"
        );
    }

    #[test]
    fn emits_no_reference_that_resolves_anywhere_but_the_iri_it_was_made_from() {
        for iri in [
            // A dot segment, which the reader removes a second time and so
            // arrives at a file the graph never named.
            "file:///checkout/fixtures/../in/two.xml",
            "file:///checkout/fixtures/./in/two.xml",
            "file:///checkout/fixtures/in/./two.xml",
            "file:///checkout/fixtures/findings/./x.ttl",
            "file:///checkout/fixtures/findings/../x.ttl",
            // A percent-encoded one, which is a name and not a step.
            "file:///checkout/fixtures/%2E%2E/in/two.xml",
            "file:///checkout/fixtures/findings/%2E/x.ttl",
            // A first step a reader would take for a scheme.
            "file:///checkout/fixtures/findings/http:x.ttl",
            "file:///checkout/fixtures/http:x.ttl",
            // Characters a reference carries as they stand, or encoded.
            "file:///checkout/fixtures/in/two%2Fone.xml",
            "file:///checkout/fixtures/in/two%20one.xml",
            "file:///checkout/fixtures/in/two%23one.xml",
            "file:///checkout/fixtures/in/caf%C3%A9.xml",
            "file:///checkout/fixtures/in/caf\u{e9}.xml",
            "file:///checkout/fixtures/in/\u{4e2d}\u{6587}.xml",
            // Shapes that are no file standing beside this one at all.
            "file:///checkout/fixtures/findings/",
            "file:///",
            "file:///checkout//in/two.xml",
            "file://localhost/checkout/fixtures/in/two.xml",
            "file:///C:/checkout/fixtures/in/two.xml",
            "urn:example:catalog#noteHasNoTerm",
            ORACLE,
        ] {
            resolves_back_to(iri);
        }
    }

    #[test]
    fn reads_a_percent_encoded_dot_segment_as_a_name_and_not_a_step() {
        assert_eq!(
            named("file:///checkout/fixtures/%2E%2E/in/two.xml").as_deref(),
            Some("../%2E%2E/in/two.xml")
        );
    }

    #[test]
    fn names_a_document_beside_the_file_by_the_way_up_and_back_down_to_it() {
        assert_eq!(
            named("file:///checkout/fixtures/in/two.xml").as_deref(),
            Some("../in/two.xml")
        );
        assert_eq!(
            named("file:///checkout/fixtures/findings/other.ttl").as_deref(),
            Some("other.ttl")
        );
        assert_eq!(
            named("file:///elsewhere/in/two.xml").as_deref(),
            Some("../../../elsewhere/in/two.xml")
        );
        assert_eq!(named(ORACLE).as_deref(), Some("two.ttl"));
    }

    #[test]
    fn names_in_full_every_iri_the_file_cannot_name_relative_to_itself() {
        for iri in [
            // Another scheme, which is every body a gap scheme declares.
            "urn:example:catalog#noteHasNoTerm",
            "https://ns.cascadeprotocol.org/bridge/v1-draft#pathNotAccounted",
            // Another host: a relative reference cannot cross one.
            "file://elsewhere/checkout/fixtures/in/two.xml",
            // A fragment, which resolution would carry along with the path.
            "file:///checkout/fixtures/in/two.xml#record",
        ] {
            assert_eq!(named(iri), None, "{iri}");
        }
    }
}
