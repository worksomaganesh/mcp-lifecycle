use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let code = mcp_lifecycle::run_cli(
        std::env::args().skip(1),
        &mut stdout.lock(),
        &mut stderr.lock(),
    );

    ExitCode::from(code)
}
