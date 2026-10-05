use crate::iri::{path_to_file_iri, percent_decode, push_encoded};
use cascade_bridge::{ErrorKind, Files, Map, Named};
use std::fs;
use std::path::{Path, PathBuf};

/// A directory read into a map, its symbolic links skipped, so the map holds only
/// files inside it.
pub struct Folder {
    path: PathBuf,
    pub iri: String,
    pub files: Files,
}

impl Folder {
    pub fn at(directory: &str) -> Result<Self, String> {
        let path = fs::canonicalize(directory).map_err(|e| format!("{directory}: {e}"))?;
        let iri = format!("{}/", path_to_file_iri(&path));
        Ok(Self {
            path,
            iri,
            files: Files::new(),
        })
    }

    pub fn named(&self) -> Named<'_> {
        Named {
            iri: &self.iri,
            files: &self.files,
        }
    }

    /// The file at `key`, a path as a map is keyed.
    pub fn read(&self, key: &str) -> Result<Vec<u8>, String> {
        let unread = |reason: &dyn std::fmt::Display| format!("{}{key}: {reason}", self.iri);
        let decoded = percent_decode(key).ok_or_else(|| unread(&"not a path"))?;
        let mut path = self.path.clone();
        for segment in decoded.split('/') {
            path.push(segment);
            let linked = fs::symlink_metadata(&path)
                .map_err(|e| unread(&e))?
                .file_type()
                .is_symlink();
            if linked {
                return Err(unread(&"a symbolic link, which is not read"));
            }
        }
        fs::read(&path).map_err(|e| unread(&e))
    }

    /// Reads each file at `keys` that the directory holds; a key it lacks is left out.
    pub fn with(mut self, keys: &[String]) -> Self {
        for key in keys {
            if let Ok(bytes) = self.read(key) {
                self.files.insert(key.clone(), bytes);
            }
        }
        self
    }

    /// Reads every regular file under the directory but those under `.git/`.
    pub fn with_every_file(mut self) -> Result<Self, String> {
        let mut pending = vec![(self.path.clone(), String::new())];
        while let Some((directory, prefix)) = pending.pop() {
            for entry in fs::read_dir(&directory).map_err(|e| at(&directory, &e))? {
                let entry = entry.map_err(|e| at(&directory, &e))?;
                let kind = entry.file_type().map_err(|e| at(&entry.path(), &e))?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let mut key = prefix.clone();
                push_encoded(&mut key, &name);
                if kind.is_dir() && !(prefix.is_empty() && name == ".git") {
                    pending.push((entry.path(), format!("{key}/")));
                } else if kind.is_file() {
                    let bytes = fs::read(entry.path()).map_err(|e| at(&entry.path(), &e))?;
                    self.files.insert(key, bytes);
                }
            }
        }
        Ok(self)
    }
}

fn at(path: &Path, error: &std::io::Error) -> String {
    format!("{}: {error}", path.display())
}

/// The adapter's folder and the vocabulary's, each read into a map as a load asks for files.
pub struct Inputs {
    pub adapter: Folder,
    pub vocabulary: Option<Folder>,
}

impl Inputs {
    /// What `call` answers once the maps hold every file it asks for, each file a map
    /// lacks read from its folder; a file the folder lacks too ends the call.
    pub fn complete<T>(
        &mut self,
        call: impl Fn(Named<'_>, Option<Named<'_>>) -> cascade_bridge::Result<T>,
    ) -> Result<T, String> {
        loop {
            let error = match call(
                self.adapter.named(),
                self.vocabulary.as_ref().map(Folder::named),
            ) {
                Ok(answer) => return Ok(answer),
                Err(error) => error,
            };
            let ErrorKind::Missing { map, path } = error.kind() else {
                return Err(error.to_string());
            };
            let folder = match map {
                Map::Adapter => Some(&mut self.adapter),
                Map::Vocabulary => self.vocabulary.as_mut(),
            };
            let Some(folder) = folder.filter(|folder| !folder.files.contains_key(path)) else {
                return Err(error.to_string());
            };
            let bytes = folder.read(path)?;
            folder.files.insert(path.clone(), bytes);
        }
    }
}
