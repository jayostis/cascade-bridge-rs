use cascade_bridge::Counted;
use std::alloc::System;
use std::process::ExitCode;

#[global_allocator]
static ALLOCATOR: Counted<System> = Counted(System);

fn main() -> ExitCode {
    ExitCode::from(cascade_bridge_cli::run(std::env::args().skip(1)))
}
