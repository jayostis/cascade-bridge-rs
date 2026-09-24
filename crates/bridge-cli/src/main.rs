use cascade_bridge::{
    convert, earl_report, file_iri, load_adapter, prepare, require_vocabularies, run_manifest,
    serialise, serialise_at, unvalidated_output, DirectoryResolver, EntryResult, GraphFormat,
    Outcome, ReportSubject, RunOptions, Source, OFFERED_PROFILES,
};
use std::io::Write;
use std::process::ExitCode;

const USAGE: &str = "usage: cascade-bridge test <adapter-dir> [--vocabularies <directory>] [--earl <out.ttl>] [--datasets]
       cascade-bridge convert <adapter-dir> <document.xml> [--vocabularies <directory>] [--out <file>] [--findings <file>] [--format turtle|ntriples]";

const FAILING: [Outcome; 2] = [Outcome::Failed, Outcome::Inapplicable];

fn subject() -> ReportSubject {
    ReportSubject {
        iri: "https://github.com/jayostis/cascade-bridge-rs".to_owned(),
        name: "Cascade Bridge for Rust".to_owned(),
        version: "0.0.0".to_owned(),
    }
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("cascade-bridge: {message}");
            ExitCode::from(2)
        }
    }
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

fn resolver(directory: &str, vocabularies: Option<&str>) -> Result<DirectoryResolver, String> {
    let resolver = DirectoryResolver::new(directory).map_err(|e| e.to_string())?;
    match vocabularies {
        Some(checkout) => resolver
            .with_vocabularies(checkout)
            .map_err(|e| e.to_string()),
        None => Ok(resolver),
    }
}

enum Command {
    Test(Test),
    Convert(Convert),
}

fn parse(argv: Vec<String>) -> Option<Command> {
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

fn run(argv: Vec<String>) -> Result<ExitCode, String> {
    let Some(command) = parse(argv) else {
        eprintln!("{USAGE}");
        return Ok(ExitCode::from(2));
    };
    match command {
        Command::Test(arguments) => test(arguments),
        Command::Convert(arguments) => convert_document(arguments),
    }
}

fn test(arguments: Test) -> Result<ExitCode, String> {
    let subject = subject();
    let resolver = resolver(&arguments.directory, arguments.vocabularies.as_deref())?;
    let adapter = load_adapter(&resolver).map_err(|e| e.to_string())?;
    require_vocabularies(&adapter, &resolver).map_err(|e| e.to_string())?;
    println!(
        "Adapter  {}  ({})",
        adapter.identifier.as_deref().unwrap_or(&adapter.root),
        adapter.root
    );
    println!(
        "Bridge   {} {}, offers {}",
        subject.name,
        subject.version,
        OFFERED_PROFILES
            .iter()
            .map(|p| p.split('#').nth(1).unwrap_or(p))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();

    let results = run_manifest(
        &adapter,
        &resolver,
        RunOptions {
            datasets: arguments.datasets,
        },
    )
    .map_err(|e| e.to_string())?;

    let width = results
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    for result in &results {
        println!(
            "  {:<12} {:<width$}  {:>6} s  {}",
            result.outcome.as_str(),
            result.name,
            format!("{:.2}", result.elapsed.as_secs_f64()),
            result.description,
            width = width
        );
    }
    println!();
    println!("{}", tally(&results));

    if let Some(path) = &arguments.earl {
        let report = earl_report(&results, &subject).map_err(|e| e.to_string())?;
        std::fs::write(path, report).map_err(|e| e.to_string())?;
        println!("EARL     {path}");
    }

    Ok(if results.iter().any(|r| FAILING.contains(&r.outcome)) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn convert_document(arguments: Convert) -> Result<ExitCode, String> {
    let resolver = resolver(&arguments.directory, arguments.vocabularies.as_deref())?;
    let adapter = load_adapter(&resolver).map_err(|e| e.to_string())?;
    let unvalidated = match &arguments.findings {
        Some(_) => {
            require_vocabularies(&adapter, &resolver).map_err(|e| e.to_string())?;
            None
        }
        None => unvalidated_output(&adapter, &resolver),
    };
    let prepared = prepare(&adapter, &resolver).map_err(|e| e.to_string())?;
    let document =
        std::fs::read(&arguments.document).map_err(|e| format!("{}: {e}", arguments.document))?;
    let iri = file_iri(&arguments.document).map_err(|e| e.to_string())?;
    let conversion = convert(
        &prepared,
        Source {
            iri: &iri,
            envelope: None,
            xml: &document,
        },
    )
    .map_err(|e| e.to_string())?;
    let graph = serialise(&conversion.quads, arguments.format, &prepared.prefixes)
        .map_err(|e| e.to_string())?;

    if let Some(unvalidated) = unvalidated {
        eprintln!("cascade-bridge: {unvalidated}");
    }
    eprintln!(
        "Adapter  {}  ({})",
        adapter.identifier.as_deref().unwrap_or(&adapter.root),
        adapter.root
    );
    eprintln!(
        "Document {}  {} record(s), {} triples, {} finding(s)",
        arguments.document,
        conversion.units,
        conversion.triples(),
        conversion.annotations()
    );
    eprintln!(
        "Detect   {}",
        match conversion.detected {
            Some(true) => "true".to_owned(),
            Some(false) =>
                "false: this adapter does not claim this document, converted anyway".to_owned(),
            None => "the adapter names no bridge:detectQuery".to_owned(),
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
        let at = file_iri(path).map_err(|e| e.to_string())?;
        let written = serialise_at(
            &conversion.findings,
            arguments.format,
            &prepared.findings_prefixes,
            Some(&at),
        )
        .map_err(|e| e.to_string())?;
        std::fs::write(path, written).map_err(|e| format!("{path}: {e}"))?;
        eprintln!("Findings {path}");
    }

    match &arguments.out {
        Some(path) => {
            std::fs::write(path, graph).map_err(|e| format!("{path}: {e}"))?;
            eprintln!("Graph    {path}");
        }
        // A pipe's reader may go away mid-graph; the caller is owed a status, not a panic.
        None => {
            let mut stdout = std::io::stdout();
            stdout
                .write_all(graph.as_bytes())
                .and_then(|()| stdout.flush())
                .map_err(|e| format!("standard output: {e}"))?;
        }
    }
    Ok(ExitCode::SUCCESS)
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
