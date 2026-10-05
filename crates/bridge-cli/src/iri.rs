use std::path::{Path, PathBuf};

/// The `file:` IRI of the file or directory a path reaches, its symbolic links resolved.
pub fn file_iri(path: impl AsRef<Path>) -> std::io::Result<String> {
    Ok(path_to_file_iri(&std::fs::canonicalize(path)?))
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

pub(crate) fn push_encoded(out: &mut String, text: &str) {
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(char::from(byte))
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
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

pub(crate) fn percent_decode(s: &str) -> Option<String> {
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
