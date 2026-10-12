use crate::folder::{Folder, Inputs};
use crate::iri::file_iri;
use cascade_bridge::{
    describe, load, load_skipping, test, Budgets, ConvertOptions, Description, Document, Executor,
    Facts, Format, Outcome, Skip, Stage, StageTime, TestOptions, NAME, VERSION,
};
use std::fmt::Write as _;
use std::io::Write as _;
use std::time::{Duration, Instant};

const USAGE: &str = "usage: cascade-bridge test <adapter-dir> [--vocabularies <directory>] [--earl <out.ttl>] [--fallbacks <out.json>] [--datasets] [--executor compiled|spareval|both] [--memory-budget-mib <1-3072>] [--time-budget-seconds <n>]
       cascade-bridge convert <adapter-dir> <document> [--envelope <iri>] [--facts <file>] [--vocabularies <directory>] [--out <file>] [--findings <file>] [--format turtle|ntriples] [--executor compiled|spareval|both] [--memory-budget-mib <1-3072>] [--time-budget-seconds <n>] [--profile] [--skip schema|shapes|findings]
       cascade-bridge library <calls file> <results directory>

--skip is for measuring only: the run leaves out the source schema check, the output shapes check or the findings queries, and its conversion is not a validated one.";

fn memory_budget(mebibytes: &str) -> Option<usize> {
    Budgets::memory_mib(mebibytes.parse().ok()?)
}

fn time_budget(seconds: &str) -> Option<Duration> {
    let seconds = seconds
        .parse::<f64>()
        .ok()
        .filter(|&seconds| seconds > 0.0)?;
    Duration::try_from_secs_f64(seconds).ok()
}

const METADATA: &str = "ro-crate-metadata.json";

struct Test {
    directory: String,
    vocabularies: Option<String>,
    earl: Option<String>,
    fallbacks: Option<String>,
    datasets: bool,
    executor: Executor,
    budgets: Budgets,
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
    executor: Executor,
    budgets: Budgets,
    profile: bool,
    skip: Vec<Skip>,
}

impl Convert {
    fn options(&self) -> ConvertOptions {
        ConvertOptions {
            executor: self.executor,
            budgets: self.budgets,
        }
    }
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
                fallbacks: None,
                datasets: false,
                executor: Executor::default(),
                budgets: Budgets::default(),
            };
            while let Some(flag) = argv.next() {
                match flag.as_str() {
                    "--vocabularies" => arguments.vocabularies = Some(argv.next()?),
                    "--earl" => arguments.earl = Some(argv.next()?),
                    "--fallbacks" => arguments.fallbacks = Some(argv.next()?),
                    "--datasets" => arguments.datasets = true,
                    "--executor" => arguments.executor = Executor::named(&argv.next()?)?,
                    "--memory-budget-mib" => {
                        arguments.budgets.memory_bytes = memory_budget(&argv.next()?)?;
                    }
                    "--time-budget-seconds" => {
                        arguments.budgets.time = time_budget(&argv.next()?)?;
                    }
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
                executor: Executor::default(),
                budgets: Budgets::default(),
                profile: false,
                skip: Vec::new(),
            };
            while let Some(flag) = argv.next() {
                match flag.as_str() {
                    "--envelope" => arguments.envelope = Some(argv.next()?),
                    "--facts" => arguments.facts = Some(argv.next()?),
                    "--vocabularies" => arguments.vocabularies = Some(argv.next()?),
                    "--out" => arguments.out = Some(argv.next()?),
                    "--findings" => arguments.findings = Some(argv.next()?),
                    "--format" => arguments.format = Format::named(&argv.next()?)?,
                    "--executor" => arguments.executor = Executor::named(&argv.next()?)?,
                    "--memory-budget-mib" => {
                        arguments.budgets.memory_bytes = memory_budget(&argv.next()?)?;
                    }
                    "--time-budget-seconds" => {
                        arguments.budgets.time = time_budget(&argv.next()?)?;
                    }
                    "--profile" => arguments.profile = true,
                    "--skip" => arguments.skip.push(Skip::named(&argv.next()?)?),
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
        executor: arguments.executor,
        budgets: arguments.budgets,
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
    if let Some(path) = &arguments.fallbacks {
        let fallbacks: Vec<serde_json::Value> = report
            .fallbacks
            .iter()
            .map(|fallback| {
                serde_json::json!({
                    "query": fallback.query.strip_prefix(&description.iri).unwrap_or(&fallback.query),
                    "reason": fallback.reason,
                })
            })
            .collect();
        let json = serde_json::to_string_pretty(&fallbacks).map_err(|e| e.to_string())?;
        written(path, format!("{json}\n").as_bytes())?;
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

fn skipped(skip: &[Skip]) -> String {
    let names: Vec<&str> = skip.iter().map(|skip| skip.as_str()).collect();
    names.join(", ")
}

fn serialising(stages: &[StageTime]) -> Duration {
    stages
        .iter()
        .filter(|time| time.stage == Stage::Serialising)
        .map(|time| time.elapsed)
        .sum()
}

/// Each stage's time, and what the clock on the wall counted that no stage did.
fn stage_report(what: &str, wall: Duration, stages: &[StageTime]) -> String {
    let counted: Duration = stages.iter().map(|time| time.elapsed).sum();
    let mut said = format!("{what:<8} {:.3} s\n", wall.as_secs_f64());
    for time in stages {
        let _ = writeln!(
            said,
            "  {:<17} {:.3} s",
            time.stage.as_str(),
            time.elapsed.as_secs_f64()
        );
    }
    let _ = writeln!(
        said,
        "  {:<17} {:.3} s",
        "other",
        wall.saturating_sub(counted).as_secs_f64()
    );
    said
}

fn convert(arguments: &Convert) -> Result<u8, String> {
    if arguments.findings.is_some() && !arguments.skip.is_empty() {
        return Err(format!(
            "--findings is not written by a run that skips {}: its findings are missing, and would pass for those of a validated conversion",
            skipped(&arguments.skip)
        ));
    }
    if !arguments.skip.is_empty() {
        eprintln!(
            "cascade-bridge: MEASUREMENT ONLY: this run skips {}; its conversion is not a validated one",
            skipped(&arguments.skip)
        );
    }
    let (mut inputs, _) = opened(&arguments.directory, arguments.vocabularies.as_deref())?;
    inputs.adapter = inputs.adapter.with_every_file()?;
    let loading = Instant::now();
    let loaded = inputs.complete(|adapter, vocabulary| {
        if arguments.profile || !arguments.skip.is_empty() {
            load_skipping(adapter, vocabulary, &arguments.skip)
        } else {
            load(adapter, vocabulary)
        }
    })?;
    let loaded_in = loading.elapsed();
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
    let document = Document {
        iri: &iri,
        bytes: &bytes,
        envelope: envelope.as_deref(),
        facts: facts.as_ref().map(|(iri, bytes)| Facts { iri, bytes }),
    };
    let converting = Instant::now();
    let conversion = if arguments.profile {
        let (conversion, queries) = loaded
            .convert_profiled_with(&document, arguments.options())
            .map_err(|e| e.to_string())?;
        for query in &queries {
            eprintln!(
                "Query    {}  {} run(s), {} row(s), {:.3} s, {} give-up(s), {}, {}",
                query.iri,
                query.executions,
                query.rows,
                query.elapsed.as_secs_f64(),
                query.give_ups,
                query.stage.map_or("in no stage", Stage::as_str),
                match (arguments.executor, query.fallback.as_deref()) {
                    (Executor::Spareval, _) => "evaluated".to_owned(),
                    (_, Some(why)) => format!("evaluated: {why}"),
                    (_, None) => "compiled".to_owned(),
                },
            );
        }
        conversion
    } else {
        loaded
            .convert_with(&document, arguments.options())
            .map_err(|e| e.to_string())?
    };
    let converted_in = converting.elapsed();
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
    if !conversion.skipped.is_empty() {
        eprintln!(
            "Skipped  {}: a measurement, not a validated conversion",
            skipped(&conversion.skipped)
        );
    }

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

    if let Some(stages) = conversion.stages() {
        eprint!(
            "{}{}",
            stage_report("Load", loaded_in, loaded.load_stages()),
            stage_report("Convert", converted_in + serialising(&stages), &stages)
        );
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
