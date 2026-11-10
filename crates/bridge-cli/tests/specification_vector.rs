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

/// Passing the specification's own adapters · cascade-bridge test
///
/// The Bridge specification ships two small adapters whose test manifests hold an entry of every
/// test type. A Bridge that passes them judges an adapter's tests as the specification says, so
/// these run the whole of each through `test`.
mod passing_the_specifications_adapters {
    use super::*;

    /// passes every entry of the synthetic XML adapter that a Bridge can judge
    #[test]
    #[ignore = "reads a checkout of jayostis/cascade-bridge-spec beside this one"]
    fn passes_every_entry_of_the_synthetic_xml_adapter_that_a_bridge_can_judge() {
        passes_every_entry_of("synthetic-adapter");
    }

    /// passes every entry of the synthetic JSON adapter that a Bridge can judge
    #[test]
    #[ignore = "reads a checkout of jayostis/cascade-bridge-spec beside this one"]
    fn passes_every_entry_of_the_synthetic_json_adapter_that_a_bridge_can_judge() {
        passes_every_entry_of("synthetic-json-adapter");
    }
}

fn passes_every_entry_of(synthetic: &str) {
    let specification = specification();
    let adapter = specification.join("fixtures").join(synthetic);
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
    // An entry this Bridge cannot judge is let through only for the reason its kind gives.
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
