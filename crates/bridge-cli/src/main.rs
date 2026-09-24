use cascade_bridge::command::{self, Host};
use cascade_bridge::{file_iri, DirectoryResolver, Error, Resolver, Result};
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(command::run(std::env::args().skip(1), &mut Process))
}

struct Process;

fn at(path: &str) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |e| Error::msg(format!("{path}: {e}"))
}

impl Host for Process {
    fn resolver(
        &mut self,
        directory: &str,
        vocabularies: Option<&str>,
    ) -> Result<Box<dyn Resolver>> {
        let resolver = DirectoryResolver::new(directory)?;
        Ok(Box::new(match vocabularies {
            Some(checkout) => resolver.with_vocabularies(checkout)?,
            None => resolver,
        }))
    }

    fn read(&mut self, path: &str) -> Result<Vec<u8>> {
        std::fs::read(path).map_err(at(path))
    }

    fn file_iri(&mut self, path: &str) -> Result<String> {
        file_iri(path)
    }

    fn create(&mut self, path: &str) -> Result<()> {
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)
            .map(drop)
            .map_err(at(path))
    }

    fn write(&mut self, path: &str, text: &str) -> Result<()> {
        std::fs::write(path, text).map_err(at(path))
    }

    // A pipe's reader may go away mid-graph; the caller is owed a status, not a panic.
    fn out(&mut self, text: &str) -> Result<()> {
        let mut stdout = std::io::stdout();
        stdout
            .write_all(text.as_bytes())
            .and_then(|()| stdout.flush())
            .map_err(|e| Error::msg(format!("standard output: {e}")))
    }

    fn err(&mut self, text: &str) {
        eprint!("{text}");
    }
}
