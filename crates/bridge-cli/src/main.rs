// cascade-bridge test <adapter-dir> [--earl <out.ttl>] [--datasets]
use cascade_bridge::{
    earl_report, load_adapter, run_manifest, DirectoryResolver, EntryResult, Outcome,
    ReportSubject, RunOptions, OFFERED_PROFILES,
};
use std::process::ExitCode;

const USAGE: &str = "usage: cascade-bridge test <adapter-dir> [--earl <out.ttl>] [--datasets]";

/// A run proves nothing when an entry failed or could not be run at all.
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

struct Arguments {
    directory: String,
    earl: Option<String>,
    datasets: bool,
}

fn parse(argv: Vec<String>) -> Option<Arguments> {
    let mut argv = argv.into_iter();
    if argv.next()? != "test" {
        return None;
    }
    let directory = argv.next()?;
    let mut arguments = Arguments {
        directory,
        earl: None,
        datasets: false,
    };
    while let Some(flag) = argv.next() {
        match flag.as_str() {
            "--earl" => arguments.earl = Some(argv.next()?),
            "--datasets" => arguments.datasets = true,
            _ => return None,
        }
    }
    Some(arguments)
}

fn run(argv: Vec<String>) -> Result<ExitCode, String> {
    let Some(arguments) = parse(argv) else {
        eprintln!("{USAGE}");
        return Ok(ExitCode::from(2));
    };

    let subject = subject();
    let resolver = DirectoryResolver::new(&arguments.directory).map_err(|e| e.to_string())?;
    let adapter = load_adapter(&resolver).map_err(|e| e.to_string())?;
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

/// The outcomes in the order they were first reached, which is the order the
/// entries are in.
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
