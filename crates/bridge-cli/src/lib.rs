//! The `cascade-bridge` command, which reads folders into maps and calls the library.

mod commands;
mod folder;
mod iri;

pub use commands::run;
pub use iri::{file_iri, file_iri_to_path, path_to_file_iri};
