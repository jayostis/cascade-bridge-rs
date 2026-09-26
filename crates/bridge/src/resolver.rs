use crate::error::{Error, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub trait Resolver {
    /// The IRI the crate's root entity resolves to, ending in "/".
    fn root(&self) -> &str;
    /// The root of the checkout the engine command named, ending in "/".
    fn vocabularies(&self) -> Option<&str> {
        None
    }
    fn read(&self, iri: &str) -> Result<Vec<u8>>;
    fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
        Err(Error::msg(format!("this host reads no vocabulary: {iri}")))
    }
}

pub fn file_iri(path: impl AsRef<Path>) -> Result<String> {
    let resolved = fs::canonicalize(path.as_ref())
        .map_err(|e| Error::msg(format!("{}: {e}", path.as_ref().display())))?;
    Ok(path_to_file_iri(&resolved))
}

pub struct DirectoryResolver {
    adapter: Directory,
    vocabularies: Option<Directory>,
}

struct Directory {
    iri: String,
    path: PathBuf,
}

impl Directory {
    fn at(dir: impl AsRef<Path>) -> Result<Self> {
        let path = fs::canonicalize(dir.as_ref())
            .map_err(|e| Error::msg(format!("{}: {e}", dir.as_ref().display())))?;
        let mut iri = path_to_file_iri(&path);
        iri.push('/');
        Ok(Self { iri, path })
    }

    fn read(&self, iri: &str, what: &str) -> Result<Vec<u8>> {
        let bare = iri.split('#').next().unwrap_or(iri);
        // Asking the filesystem about another server's path would contact it.
        let inside = (authority(bare) == authority(&self.iri))
            .then(|| file_iri_to_path(bare))
            .flatten()
            // Decided on the resolved path: "%2e%2e" and a symbolic link both hide from the IRI.
            .and_then(|path| resolve(&path))
            .filter(|path| path.starts_with(&self.path));
        let Some(path) = inside else {
            return Err(unread(iri, &format!("not inside {what}")));
        };
        fs::read(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::missing(format!("{iri}: {e}")),
            _ => unread(iri, &e.to_string()),
        })
    }
}

pub fn unread(iri: &str, reason: &str) -> Error {
    Error::msg(format!("{iri}: {reason}"))
}

impl DirectoryResolver {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            adapter: Directory::at(dir)?,
            vocabularies: None,
        })
    }

    pub fn with_vocabularies(self, dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            vocabularies: Some(Directory::at(dir)?),
            ..self
        })
    }
}

impl Resolver for DirectoryResolver {
    fn root(&self) -> &str {
        &self.adapter.iri
    }

    fn vocabularies(&self) -> Option<&str> {
        self.vocabularies
            .as_ref()
            .map(|directory| directory.iri.as_str())
    }

    fn read(&self, iri: &str) -> Result<Vec<u8>> {
        self.adapter.read(iri, "the adapter")
    }

    fn read_vocabulary(&self, iri: &str) -> Result<Vec<u8>> {
        match &self.vocabularies {
            Some(directory) => directory.read(iri, "the vocabularies"),
            None => Err(Error::msg(format!(
                "the command named no vocabularies: {iri}"
            ))),
        }
    }
}

/// A missing file resolves through its nearest existing ancestor, so it is still
/// judged against the boundary.
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

pub fn path_to_file_iri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    // A Windows canonical path opens with the extended-length prefix, which Node's leaves off.
    let unc = text.strip_prefix("//?/UNC/").or_else(|| {
        text.strip_prefix("//")
            .filter(|rest| !rest.starts_with("?/"))
    });
    let (server, text) = match unc {
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
pub fn authority(iri: &str) -> Option<&str> {
    let rest = iri.strip_prefix("file://")?;
    let server = &rest[..rest.find('/').unwrap_or(rest.len())];
    Some(if server == "localhost" { "" } else { server })
}

pub fn file_iri_to_path(iri: &str) -> Option<PathBuf> {
    let rest = iri.strip_prefix("file://")?;
    let (server, rest) = rest.split_at(rest.find('/')?);
    let decoded = percent_decode(rest)?;
    if !server.is_empty() && server != "localhost" {
        return Some(PathBuf::from(format!(
            "//{}{decoded}",
            percent_decode(server)?
        )));
    }
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
mod boundary;
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
    fn writes_a_drive_path_as_node_s_realpath_spells_it_under_the_empty_authority() {
        assert_eq!(
            path_to_file_iri(Path::new(r"C:\dev\adapter")),
            "file:///C:/dev/adapter"
        );
    }

    #[test]
    fn writes_a_network_path_as_node_s_realpath_spells_it_with_its_server_as_the_authority() {
        let iri = path_to_file_iri(Path::new(r"\\server\share\adapter"));
        assert_eq!(iri, "file://server/share/adapter");
        let path = file_iri_to_path(&iri).expect("a file IRI");
        assert_eq!(path, PathBuf::from("//server/share/adapter"));
        assert_eq!(path_to_file_iri(&path), iri);
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
