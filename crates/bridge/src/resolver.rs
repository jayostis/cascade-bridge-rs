// Where an adapter's files come from. The library asks for bytes by IRI and
// never touches a filesystem itself, so the same code runs wherever a host can
// supply bytes: a directory here, a fetch in a browser.
//
// This is the one module that may name std::fs or std::path
// (tests/boundary.rs holds it to that).
use crate::error::{Error, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub trait Resolver {
    /// The adapter's root, the IRI the crate's root entity resolves to,
    /// ending in "/".
    fn root(&self) -> &str;
    /// The root of the picked `the-cascade-protocol/spec` checkout the engine
    /// command was given, ending in "/", where it was given one. A path the
    /// crate's `bridge:vocabularyFile` names resolves against this and against
    /// nothing else.
    fn vocabularies(&self) -> Option<&str> {
        None
    }
    fn read(&self, iri: &str) -> Result<Vec<u8>>;
}

/// The IRI a file on this machine is named by, for a document a caller holds
/// rather than one the adapter committed.
pub fn file_iri(path: impl AsRef<Path>) -> Result<String> {
    let resolved = fs::canonicalize(path.as_ref())
        .map_err(|e| Error::msg(format!("{}: {e}", path.as_ref().display())))?;
    Ok(path_to_file_iri(&resolved))
}

/// Resolve an adapter from a directory. Nothing outside it is readable.
pub struct DirectoryResolver {
    root_iri: String,
    root_path: PathBuf,
    vocabularies_iri: Option<String>,
}

impl DirectoryResolver {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self> {
        let root_path = fs::canonicalize(dir.as_ref())
            .map_err(|e| Error::msg(format!("{}: {e}", dir.as_ref().display())))?;
        let mut root_iri = path_to_file_iri(&root_path);
        root_iri.push('/');
        Ok(Self {
            root_iri,
            root_path,
            vocabularies_iri: None,
        })
    }

    /// The checkout of `the-cascade-protocol/spec` the engine command named,
    /// widening what is readable to that directory and to nothing else.
    pub fn with_vocabularies(self, dir: impl AsRef<Path>) -> Result<Self> {
        let path = fs::canonicalize(dir.as_ref())
            .map_err(|e| Error::msg(format!("{}: {e}", dir.as_ref().display())))?;
        let mut iri = path_to_file_iri(&path);
        iri.push('/');
        Ok(Self {
            vocabularies_iri: Some(iri),
            ..self
        })
    }
}

impl Resolver for DirectoryResolver {
    fn root(&self) -> &str {
        &self.root_iri
    }

    fn vocabularies(&self) -> Option<&str> {
        self.vocabularies_iri.as_deref()
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        let outside = || Error::msg(format!("not inside the adapter: {iri}"));
        let bare = iri.split('#').next().unwrap_or(iri);
        // A server in the IRI is a network host. One that is not the
        // adapter's own is refused before the filesystem is asked, since
        // asking would contact it.
        if authority(bare) != authority(&self.root_iri) {
            return Err(outside());
        }
        let path = file_iri_to_path(bare).ok_or_else(outside)?;
        // The boundary is decided on the resolved filesystem path, never on
        // the IRI string: percent-encoding hides "%2e%2e" from a prefix test
        // and a symbolic link hides the destination from both.
        if !resolve(&path).is_some_and(|p| p.starts_with(&self.root_path)) {
            return Err(outside());
        }
        Ok(fs::read(&path)?)
    }
}

/// The path a filesystem would reach, with links followed and dot segments
/// gone. A path that does not exist is resolved through the nearest ancestor
/// that does, so a missing file is still judged against the boundary rather
/// than escaping it on the way to a "not found".
fn resolve(path: &Path) -> Option<PathBuf> {
    if let Ok(p) = fs::canonicalize(path) {
        return Some(p);
    }
    let mut head = path;
    while let Some(parent) = head.parent() {
        head = parent;
        let Ok(mut resolved) = fs::canonicalize(head) else {
            continue;
        };
        for component in path.strip_prefix(head).ok()?.components() {
            match component {
                Component::Normal(c) => resolved.push(c),
                Component::ParentDir => {
                    resolved.pop();
                }
                Component::CurDir => {}
                _ => return None,
            }
        }
        return Some(resolved);
    }
    None
}

fn path_to_file_iri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    // A Windows canonical path opens with the extended-length prefix. A
    // network path's server is the IRI's authority, as RFC 8089 writes a UNC
    // path; a drive letter needs the empty authority's slash that a POSIX
    // path already carries.
    let (server, text) = match text.strip_prefix("//?/UNC/") {
        Some(unc) => unc.split_once('/').unwrap_or((unc, "")),
        None => ("", text.strip_prefix("//?/").unwrap_or(&text)),
    };
    let mut out = String::from("file://");
    push_encoded(&mut out, server);
    if !text.starts_with('/') {
        out.push('/');
    }
    push_encoded(&mut out, text);
    out
}

fn push_encoded(out: &mut String, text: &str) {
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(char::from(byte))
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
}

/// The server a file IRI names, empty for this machine; none for an IRI that
/// is not a file IRI.
fn authority(iri: &str) -> Option<&str> {
    let rest = iri.strip_prefix("file://")?;
    let server = &rest[..rest.find('/').unwrap_or(rest.len())];
    Some(if server == "localhost" { "" } else { server })
}

fn file_iri_to_path(iri: &str) -> Option<PathBuf> {
    let rest = iri.strip_prefix("file://")?;
    let (server, rest) = rest.split_at(rest.find('/')?);
    let decoded = percent_decode(rest)?;
    // Two leading slashes are what Windows reads as a server and a share.
    if !server.is_empty() && server != "localhost" {
        return Some(PathBuf::from(format!(
            "//{}{decoded}",
            percent_decode(server)?
        )));
    }
    // "/C:/x" is a Windows path; "/home/x" is a POSIX one.
    let bytes = decoded.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        return Some(PathBuf::from(&decoded[1..]));
    }
    Some(PathBuf::from(decoded))
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_drive_path_under_the_empty_authority() {
        assert_eq!(
            path_to_file_iri(Path::new(r"\\?\C:\dev\adapter")),
            "file:///C:/dev/adapter"
        );
        assert_eq!(
            file_iri_to_path("file:///C:/dev/adapter/x.json"),
            Some(PathBuf::from("C:/dev/adapter/x.json"))
        );
    }

    #[test]
    fn writes_a_network_path_with_its_server_as_the_authority() {
        assert_eq!(
            path_to_file_iri(Path::new(r"\\?\UNC\server\share\adapter")),
            "file://server/share/adapter"
        );
    }

    #[test]
    fn reads_a_network_iri_back_to_the_network_path() {
        assert_eq!(
            file_iri_to_path("file://server/share/adapter/ro-crate-metadata.json"),
            Some(PathBuf::from(
                "//server/share/adapter/ro-crate-metadata.json"
            ))
        );
    }
}
