//! The `cascade-bridge` command, which reads folders into maps and calls the library.

mod command;
mod folder;
mod iri;

pub use command::run;
pub use iri::{file_iri, file_iri_to_path, path_to_file_iri};
