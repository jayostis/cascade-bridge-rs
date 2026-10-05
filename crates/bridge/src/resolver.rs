use crate::error::{Error, Map, Result};
use crate::library::Named;
use oxiri::Iri;

pub(crate) trait Resolver {
    /// The IRI the crate's root entity resolves to, ending in "/".
    fn root(&self) -> &str;
    /// The IRI the vocabulary's files resolve against, ending in "/".
    fn vocabularies(&self) -> Option<&str> {
        None
    }
    fn read(&self, iri: &str) -> Result<Vec<u8>>;
    fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
        Err(Error::vocabulary(format!("no vocabulary was given: {iri}")))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Maps<'a> {
    pub(crate) adapter: Named<'a>,
    pub(crate) vocabulary: Option<Named<'a>>,
}

impl Resolver for Maps<'_> {
    fn root(&self) -> &str {
        self.adapter.iri
    }

    fn vocabularies(&self) -> Option<&str> {
        self.vocabulary.map(|vocabulary| vocabulary.iri)
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        served(self.adapter, Map::Adapter, iri)
    }

    fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
        match self.vocabulary {
            Some(vocabulary) => served(vocabulary, Map::Vocabulary, iri),
            None => Err(Error::vocabulary(format!("no vocabulary was given: {iri}"))),
        }
    }
}

fn served(named: Named<'_>, map: Map, iri: &str) -> Result<Vec<u8>> {
    let what = match map {
        Map::Adapter => "the adapter",
        Map::Vocabulary => "the vocabulary",
    };
    let Some(path) = key(named.iri, iri) else {
        let outside = format!("{iri}: not inside {what}");
        return Err(match map {
            Map::Adapter => Error::adapter(outside),
            Map::Vocabulary => Error::vocabulary(outside),
        });
    };
    match named.files.get(&path) {
        Some(bytes) => Ok(bytes.clone()),
        None => Err(Error::missing(
            map,
            path.clone(),
            format!("{iri}: {what} holds no {path}"),
        )),
    }
}

/// The path `iri` names in the map at `root`, or none where it is not inside it.
pub(crate) fn key(root: &str, iri: &str) -> Option<String> {
    let bare = iri.split('#').next().unwrap_or(iri);
    let dotted = bare.replace("%2e", ".").replace("%2E", ".");
    Iri::parse(dotted.as_str()).ok()?;
    let path_at = match dotted.find("://") {
        Some(scheme) => dotted[scheme + 3..]
            .find('/')
            .map_or(dotted.len(), |slash| scheme + 3 + slash),
        None => dotted.find(':')? + 1,
    };
    let (head, path) = dotted.split_at(path_at);
    let resolved = format!("{head}{}", without_dot_segments(path));
    let path = resolved.strip_prefix(root)?;
    (!path.is_empty()).then(|| path.to_owned())
}

fn without_dot_segments(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let mut kept: Vec<&str> = Vec::new();
    for (at, segment) in segments.iter().enumerate() {
        let last = at + 1 == segments.len();
        match *segment {
            "." => {}
            ".." => {
                if kept.len() > 1 {
                    kept.pop();
                }
            }
            segment => {
                kept.push(segment);
                continue;
            }
        }
        if last {
            kept.push("");
        }
    }
    kept.join("/")
}

#[cfg(test)]
mod boundary;
