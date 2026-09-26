use crate::earl::{earl_report, ReportSubject};
use crate::error::Result;
use crate::harness::{run_manifest, EntryResult, Outcome, RunOptions, OFFERED_PROFILES};
use crate::load::{load_adapter, Adapter};
use crate::rdf::{serialise, serialise_at, GraphFormat};
use crate::resolver::Resolver;
use crate::run::{self, prepare, Source};
use crate::vocabulary::{require_vocabularies, unvalidated_output};
use std::fmt::Write;

const USAGE: &str = "usage: cascade-bridge test <adapter-dir> [--vocabularies <directory>] [--earl <out.ttl>] [--datasets]
       cascade-bridge convert <adapter-dir> <document.xml> [--vocabularies <directory>] [--out <file>] [--findings <file>] [--format turtle|ntriples]";

/// Every path is the command's argument as it was given. An error is printed
/// as the reason the command stopped, so it names what it could not do.
pub trait Host {
    fn resolver(
        &mut self,
        directory: &str,
        vocabularies: Option<&str>,
    ) -> Result<Box<dyn Resolver>>;
    fn read(&mut self, path: &str) -> Result<Vec<u8>>;
    fn file_iri(&mut self, path: &str) -> Result<String>;
    /// A file that was already there is left as it was.
    fn create(&mut self, path: &str) -> Result<()>;
    fn write(&mut self, path: &str, text: &str) -> Result<()>;
    fn out(&mut self, text: &str) -> Result<()>;
    fn err(&mut self, text: &str);
}

struct Test {
    directory: String,
    vocabularies: Option<String>,
    earl: Option<String>,
    datasets: bool,
}

struct Convert {
    directory: String,
    document: String,
    vocabularies: Option<String>,
    out: Option<String>,
    findings: Option<String>,
    format: GraphFormat,
}

enum Command {
    Test(Test),
    Convert(Convert),
}

fn parse(argv: impl IntoIterator<Item = String>) -> Option<Command> {
    let mut argv = argv.into_iter();
    match argv.next()?.as_str() {
        "test" => {
            let mut arguments = Test {
                directory: argv.next()?,
                vocabularies: None,
                earl: None,
                datasets: false,
            };
            while let Some(flag) = argv.next() {
                match flag.as_str() {
                    "--vocabularies" => arguments.vocabularies = Some(argv.next()?),
                    "--earl" => arguments.earl = Some(argv.next()?),
                    "--datasets" => arguments.datasets = true,
                    _ => return None,
                }
            }
            Some(Command::Test(arguments))
        }
        "convert" => {
            let mut arguments = Convert {
                directory: argv.next()?,
                document: argv.next()?,
                vocabularies: None,
                out: None,
                findings: None,
                format: GraphFormat::Turtle,
            };
            while let Some(flag) = argv.next() {
                match flag.as_str() {
                    "--vocabularies" => arguments.vocabularies = Some(argv.next()?),
                    "--out" => arguments.out = Some(argv.next()?),
                    "--findings" => arguments.findings = Some(argv.next()?),
                    "--format" => arguments.format = GraphFormat::named(&argv.next()?)?,
                    _ => return None,
                }
            }
            Some(Command::Convert(arguments))
        }
        _ => None,
    }
}

/// The command's exit status: 0 where it did what it was asked and every entry
/// held, 1 where an entry did not, 2 where it could not do what it was asked.
pub fn run(argv: impl IntoIterator<Item = String>, host: &mut dyn Host) -> u8 {
    let Some(command) = parse(argv) else {
        host.err(&format!("{USAGE}\n"));
        return 2;
    };
    let ran = match command {
        Command::Test(arguments) => test(&arguments, host),
        Command::Convert(arguments) => convert(&arguments, host),
    };
    ran.unwrap_or_else(|error| {
        host.err(&format!("cascade-bridge: {error}\n"));
        2
    })
}

const FAILING: [Outcome; 2] = [Outcome::Failed, Outcome::Inapplicable];

fn subject() -> ReportSubject {
    ReportSubject {
        iri: "https://github.com/jayostis/cascade-bridge-rs".to_owned(),
        name: "Cascade Bridge for Rust".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

fn adapter_line(adapter: &Adapter) -> String {
    format!(
        "Adapter  {}  ({})",
        adapter.identifier.as_deref().unwrap_or(&adapter.root),
        adapter.root
    )
}

fn tally(results: &[EntryResult]) -> String {
    let mut counts: Vec<(Outcome, usize)> = Vec::new();
    for result in results {
        match counts.iter_mut().find(|(o, _)| *o == result.outcome) {
            Some((_, n)) => *n += 1,
            None => counts.push((result.outcome, 1)),
        }
    }
    counts
        .iter()
        .map(|(outcome, n)| format!("{n} {}", outcome.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn test(arguments: &Test, host: &mut dyn Host) -> Result<u8> {
    let subject = subject();
    let resolver = host.resolver(&arguments.directory, arguments.vocabularies.as_deref())?;
    let adapter = load_adapter(resolver.as_ref())?;
    require_vocabularies(&adapter, resolver.as_ref())?;
    host.out(&format!(
        "{}\nBridge   {} {}, offers {}\n\n",
        adapter_line(&adapter),
        subject.name,
        subject.version,
        OFFERED_PROFILES
            .iter()
            .map(|p| p.split('#').nth(1).unwrap_or(p))
            .collect::<Vec<_>>()
            .join(", ")
    ))?;
    let results = run_manifest(
        &adapter,
        resolver.as_ref(),
        RunOptions {
            datasets: arguments.datasets,
        },
    )?;
    let width = results
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let mut summary = String::new();
    for result in &results {
        let _ = writeln!(
            summary,
            "  {:<12} {:<width$}  {:>6} s  {}",
            result.outcome.as_str(),
            result.name,
            format!("{:.2}", result.elapsed.as_secs_f64()),
            result.description,
        );
    }
    let _ = writeln!(summary, "\n{}", tally(&results));
    host.out(&summary)?;
    if let Some(path) = &arguments.earl {
        let report = earl_report(&results, &subject)?;
        host.write(path, &report)?;
        host.out(&format!("EARL     {path}\n"))?;
    }
    Ok(u8::from(
        results.iter().any(|r| FAILING.contains(&r.outcome)),
    ))
}

fn convert(arguments: &Convert, host: &mut dyn Host) -> Result<u8> {
    let resolver = host.resolver(&arguments.directory, arguments.vocabularies.as_deref())?;
    let adapter = load_adapter(resolver.as_ref())?;
    let unvalidated = match &arguments.findings {
        Some(_) => {
            require_vocabularies(&adapter, resolver.as_ref())?;
            None
        }
        None => unvalidated_output(&adapter, resolver.as_ref()),
    };
    let prepared = prepare(&adapter, resolver.as_ref())?;
    let xml = host.read(&arguments.document)?;
    let iri = host.file_iri(&arguments.document)?;
    let conversion = run::convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            xml: &xml,
        },
    )?;
    let graph = serialise(&conversion.quads, arguments.format, &prepared.prefixes)?;
    if let Some(unvalidated) = unvalidated {
        host.err(&format!("cascade-bridge: {unvalidated}\n"));
    }
    host.err(&format!(
        "{}\nDocument {iri}  {} record(s), {} triples, {} finding(s)\nDetect   {}\n",
        adapter_line(&adapter),
        conversion.units,
        conversion.triples(),
        conversion.annotations(),
        match conversion.detected {
            Some(true) => "true",
            Some(false) => "false: this adapter does not claim this document, converted anyway",
            None => "the adapter names no bridge:detectQuery",
        }
    ));

    // Before the graph, so a findings file that cannot be written leaves standard output empty.
    if let Some(path) = &arguments.findings {
        // Created first, since its own IRI names the document; not emptied, so a run that
        // fails leaves a committed oracle as it was.
        host.create(path)?;
        let at = host.file_iri(path)?;
        let written = serialise_at(
            &conversion.findings,
            arguments.format,
            &prepared.findings_prefixes,
            Some(&at),
        )?;
        host.write(path, &written)?;
        host.err(&format!("Findings {path}\n"));
    }

    match &arguments.out {
        Some(path) => {
            host.write(path, &graph)?;
            host.err(&format!("Graph    {path}\n"));
        }
        None => host.out(&graph)?,
    }
    Ok(0)
}
