use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(cascade_bridge_cli::run(std::env::args().skip(1)))
}
