use std::path::PathBuf;
use std::process::Command;

fn tiny() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bridge/tests/tiny-adapter")
}

#[test]
fn prints_a_line_per_entry_and_exits_non_zero_when_an_entry_fails() {
    let run = Command::new(env!("CARGO_BIN_EXE_cascade-bridge"))
        .args(["test", &tiny().to_string_lossy()])
        .output()
        .expect("run the command");
    let stdout = String::from_utf8(run.stdout).expect("utf-8");
    assert_eq!(run.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("passed       pass "), "{stdout}");
    assert!(stdout.contains("failed       graph-fail"), "{stdout}");
    assert!(
        stdout.contains("2 passed, 2 failed, 1 cantTell, 1 untested"),
        "{stdout}"
    );
}

#[test]
fn refuses_an_unknown_command_with_a_usage_line() {
    let run = Command::new(env!("CARGO_BIN_EXE_cascade-bridge"))
        .arg("convert")
        .output()
        .expect("run the command");
    assert_eq!(run.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&run.stderr).contains("usage: cascade-bridge test"));
}
