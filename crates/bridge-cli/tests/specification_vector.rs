// The vector the specification writes and this Bridge reproduces: the
// synthetic adapter of jayostis/cascade-bridge-spec, whose findings fixtures
// are the contract for what an accounting entry reports and what a census
// counts.
//
// It reads a checkout of another repository, which a checkout of this one does
// not carry, so it is named rather than run by default: `cargo test --
// --ignored`. The compatibility job runs it there, after the specification's
// start action has left that checkout beside this one at the version the run
// picked.
use std::path::PathBuf;
use std::process::Command;

/// The specification's checkout, beside this one or wherever
/// `CASCADE_BRIDGE_SPEC` names it.
fn specification() -> PathBuf {
    match std::env::var_os("CASCADE_BRIDGE_SPEC") {
        Some(named) => PathBuf::from(named),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../cascade-bridge-spec"),
    }
}

#[test]
#[ignore = "reads a checkout of jayostis/cascade-bridge-spec beside this one"]
fn reproduces_every_finding_the_specification_s_synthetic_adapter_expects() {
    let specification = specification();
    let adapter = specification.join("fixtures/synthetic-adapter");
    let vocabularies = specification.join("fixtures/synthetic-vocabularies");
    assert!(
        adapter.is_dir(),
        "no synthetic adapter at {}",
        adapter.display()
    );
    let run = Command::new(env!("CARGO_BIN_EXE_cascade-bridge"))
        .arg("test")
        .arg(&adapter)
        .arg("--vocabularies")
        .arg(&vocabularies)
        .output()
        .expect("run the command");
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    let printed = format!("{stdout}{}", String::from_utf8_lossy(&run.stderr));
    // An entry's line: its outcome, its name, its time, and what the run said.
    let entries: Vec<(&str, &str, &str)> = stdout
        .lines()
        .filter(|line| line.starts_with("  "))
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let outcome = fields.next()?;
            let name = fields.next()?;
            let (_, said) = line.split_once(" s  ")?;
            Some((outcome, name, said))
        })
        .collect();
    // Which entries the specification's manifest names is the specification's to
    // change, so they are read rather than listed. Every one this Bridge can
    // judge passes; an entry it cannot judge is let through only for the reason
    // its kind gives, so a judged entry going quiet fails here.
    assert!(!entries.is_empty(), "{printed}");
    for (outcome, name, said) in &entries {
        let excused = match *outcome {
            "passed" => true,
            "cantTell" => said.contains("bridge:InputOnlyTest"),
            "untested" => said.contains("datasets are not fetched"),
            _ => false,
        };
        assert!(excused, "{name} is {outcome}: {said}\n{printed}");
    }
    assert!(
        entries.iter().any(|(outcome, ..)| *outcome == "passed"),
        "{printed}"
    );
    assert_eq!(run.status.code(), Some(0), "{printed}");
}
