// The vector the specification writes and this Bridge reproduces: the
// synthetic adapter of jayostis/cascade-bridge-spec, whose findings fixtures
// are the contract for what an accounting entry reports and what a census
// counts.
//
// It reads a checkout of another repository, which a checkout of this one does
// not carry and CI does not have, so it is run by name rather than by default:
// `cargo test -- --ignored`, or the compatibility job, which runs the same
// command through compatibility.json.
use std::path::PathBuf;
use std::process::Command;

/// The specification's checkout, beside this one or wherever
/// `CASCADE_BRIDGE_SPEC` names it.
fn synthetic_adapter() -> PathBuf {
    let root = match std::env::var_os("CASCADE_BRIDGE_SPEC") {
        Some(named) => PathBuf::from(named),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../cascade-bridge-spec"),
    };
    root.join("fixtures/synthetic-adapter")
}

#[test]
#[ignore = "reads a checkout of jayostis/cascade-bridge-spec beside this one"]
fn reproduces_every_finding_the_specification_s_synthetic_adapter_expects() {
    let adapter = synthetic_adapter();
    assert!(
        adapter.is_dir(),
        "no synthetic adapter at {}",
        adapter.display()
    );
    let run = Command::new(env!("CARGO_BIN_EXE_cascade-bridge"))
        .args(["test", &adapter.to_string_lossy()])
        .output()
        .expect("run the command");
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    let outcomes: Vec<(&str, &str)> = stdout
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?, fields.next()?))
        })
        .collect();
    for entry in [
        "example-0001",
        "example-0003",
        "example-0004",
        "example-0005",
    ] {
        assert!(outcomes.contains(&("passed", entry)), "{stdout}");
    }
    assert!(
        stdout.contains("4 passed, 1 cantTell, 1 untested"),
        "{stdout}"
    );
    assert_eq!(run.status.code(), Some(0), "{stdout}");
}
