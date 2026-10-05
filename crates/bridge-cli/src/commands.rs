use crate::folder::{Folder, Inputs};
use crate::iri::file_iri;
use cascade_bridge::{
    describe, load, test, Description, Document, Facts, Format, Outcome, TestOptions, NAME, VERSION,
};
use std::fmt::Write as _;
use std::io::Write as _;

const USAGE: &str = "usage: cascade-bridge test <adapter-dir> [--vocabularies <directory>] [--earl <out.ttl>] [--datasets]
       cascade-bridge convert <adapter-dir> <document> [--envelope <iri>] [--facts <file>] [--vocabularies <directory>] [--out <file>] [--findings <file>] [--format turtle|ntriples]
       cascade-bridge library <calls file> <results directory>";

const METADATA: &str = "ro-crate-metadata.json";

struct Test {
    directory: String,
    vocabularies: Option<String>,
    earl: Option<String>,
    datasets: bool,
}

struct Convert {
    directory: String,
    document: String,
    envelope: Option<String>,
    facts: Option<String>,
    vocabularies: Option<String>,
    out: Option<String>,
    findings: Option<String>,
    format: Format,
}

enum Command {
    Test(Test),
    Convert(Convert),
    Library { calls: String, results: String },
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
                envelope: None,
                facts: None,
                vocabularies: None,
                out: None,
                findings: None,
                format: Format::Turtle,
            };
            while let Some(flag) = argv.next() {
                match flag.as_str() {
                    "--envelope" => arguments.envelope = Some(argv.next()?),
                    "--facts" => arguments.facts = Some(argv.next()?),
                    "--vocabularies" => arguments.vocabularies = Some(argv.next()?),
                    "--out" => arguments.out = Some(argv.next()?),
                    "--findings" => arguments.findings = Some(argv.next()?),
                    "--format" => arguments.format = Format::named(&argv.next()?)?,
                    _ => return None,
                }
            }
            Some(Command::Convert(arguments))
        }
        "library" => {
            let command = Command::Library {
                calls: argv.next()?,
                results: argv.next()?,
            };
            argv.next().is_none().then_some(command)
        }
        _ => None,
    }
}

/// The command's exit status: 0 where it did what it was asked and every entry
/// held, 1 where an entry did not, 2 where it could not do what it was asked.
pub fn run(argv: impl IntoIterator<Item = String>) -> u8 {
    let Some(command) = parse(argv) else {
        eprintln!("{USAGE}");
        return 2;
    };
    let ran = match command {
        Command::Test(arguments) => run_test(&arguments),
        Command::Convert(arguments) => convert(&arguments),
        Command::Library { calls, results } => crate::library::run(&calls, &results),
    };
    ran.unwrap_or_else(|reason| {
        eprintln!("cascade-bridge: {reason}");
        2
    })
}

// A pipe's reader may go away mid-graph; the caller is owed a status, not a panic.
fn out(bytes: &[u8]) -> Result<(), String> {
    let mut stdout = std::io::stdout();
    stdout
        .write_all(bytes)
        .and_then(|()| stdout.flush())
        .map_err(|e| format!("standard output: {e}"))
}

fn written(path: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{path}: {e}"))
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}

fn iri_of(path: &str) -> Result<String, String> {
    file_iri(path).map_err(|e| format!("{path}: {e}"))
}

/// The adapter's folder and the vocabulary's, with what the crate describes.
fn opened(directory: &str, vocabularies: Option<&str>) -> Result<(Inputs, Description), String> {
    let adapter = Folder::at(directory)?;
    let vocabulary = vocabularies.map(Folder::at).transpose()?;
    let description =
        describe(&adapter.iri, &adapter.read(METADATA)?).map_err(|e| e.to_string())?;
    let vocabulary = vocabulary.map(|folder| folder.with(&description.vocabulary_files));
    Ok((
        Inputs {
            adapter,
            vocabulary,
        },
        description,
    ))
}

fn adapter_line(identifier: Option<&str>, iri: &str) -> String {
    format!("Adapter  {}  ({iri})", identifier.unwrap_or(iri))
}

const FAILING: [Outcome; 2] = [Outcome::Failed, Outcome::Inapplicable];

fn tally(outcomes: &[Outcome]) -> String {
    let mut counts: Vec<(Outcome, usize)> = Vec::new();
    for outcome in outcomes {
        match counts.iter_mut().find(|(o, _)| o == outcome) {
            Some((_, n)) => *n += 1,
            None => counts.push((*outcome, 1)),
        }
    }
    counts
        .iter()
        .map(|(outcome, n)| format!("{n} {}", outcome.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn run_test(arguments: &Test) -> Result<u8, String> {
    let (mut inputs, description) =
        opened(&arguments.directory, arguments.vocabularies.as_deref())?;
    inputs.adapter = inputs.adapter.with_every_file()?;
    let options = TestOptions {
        datasets: arguments.datasets,
    };
    let report = inputs.complete(|adapter, vocabulary| test(adapter, vocabulary, options))?;
    let mut said = format!(
        "{}\nBridge   {NAME} {VERSION}, offers {}\n\n",
        adapter_line(description.identifier.as_deref(), &description.iri),
        report
            .profiles
            .iter()
            .map(|p| p.split('#').nth(1).unwrap_or(p))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let width = report
        .entries
        .iter()
        .map(|entry| entry.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    for entry in &report.entries {
        let _ = writeln!(
            said,
            "  {:<12} {:<width$}  {:>6} s  {}",
            entry.outcome.as_str(),
            entry.name,
            format!("{:.2}", entry.elapsed.as_secs_f64()),
            entry.description,
        );
    }
    let outcomes: Vec<Outcome> = report.entries.iter().map(|entry| entry.outcome).collect();
    let _ = writeln!(said, "\n{}", tally(&outcomes));
    out(said.as_bytes())?;
    if let Some(path) = &arguments.earl {
        written(path, &report.earl)?;
        out(format!("EARL     {path}\n").as_bytes())?;
    }
    Ok(u8::from(outcomes.iter().any(|o| FAILING.contains(o))))
}

/// Resolved as the adapter's `ro-crate-metadata.json` resolves the IRIs it writes.
fn crate_named(adapter: &str, named: &str) -> Result<String, String> {
    let base = oxiri::Iri::parse(format!("{adapter}{METADATA}")).map_err(|e| e.to_string())?;
    Ok(base
        .resolve(named)
        .map_err(|e| format!("--envelope {named}: {e}"))?
        .into_inner())
}

fn convert(arguments: &Convert) -> Result<u8, String> {
    let (mut inputs, description) =
        opened(&arguments.directory, arguments.vocabularies.as_deref())?;
    inputs.adapter = inputs.adapter.with(&description.load_files);
    let loaded = inputs.complete(load)?;
    let bytes = read(&arguments.document)?;
    let iri = iri_of(&arguments.document)?;
    let envelope = arguments
        .envelope
        .as_deref()
        .map(|named| crate_named(&inputs.adapter.iri, named))
        .transpose()?;
    let facts = match &arguments.facts {
        Some(path) => Some((iri_of(path)?, read(path)?)),
        None => None,
    };
    let conversion = loaded
        .convert(&Document {
            iri: &iri,
            bytes: &bytes,
            envelope: envelope.as_deref(),
            facts: facts.as_ref().map(|(iri, bytes)| Facts { iri, bytes }),
        })
        .map_err(|e| e.to_string())?;
    if let (Some(_), Some(unvalidated)) = (&arguments.findings, &conversion.unvalidated) {
        return Err(unvalidated.clone());
    }
    let graph = conversion
        .graph(arguments.format)
        .map_err(|e| e.to_string())?;
    if let Some(unvalidated) = &conversion.unvalidated {
        eprintln!("cascade-bridge: {unvalidated}");
    }
    eprint!(
        "{}\nDocument {iri}  {} record(s), {} triples, {} finding(s)\nDetect   {}\n",
        adapter_line(loaded.identifier(), loaded.iri()),
        conversion.records,
        conversion.triples,
        conversion.finding_count,
        match conversion.detected {
            Some(true) => "true",
            Some(false) => "false: this adapter does not claim this document, converted anyway",
            None => "the adapter names no bridge:detectQuery",
        }
    );

    // Before the graph, so a findings file that cannot be written leaves standard output empty.
    if let Some(path) = &arguments.findings {
        // Created first, since its own IRI names the document; not emptied, so a run that
        // fails leaves a committed oracle as it was.
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)
            .map_err(|e| format!("{path}: {e}"))?;
        let at = iri_of(path)?;
        let findings = conversion
            .findings(arguments.format, Some(&at))
            .map_err(|e| e.to_string())?;
        written(path, &findings)?;
        eprintln!("Findings {path}");
    }

    match &arguments.out {
        Some(path) => {
            written(path, &graph)?;
            eprintln!("Graph    {path}");
        }
        None => out(&graph)?,
    }
    Ok(0)
}
