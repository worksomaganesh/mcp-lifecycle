use std::io::Write;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
MCP Lifecycle & Leak Lab
Local-first tooling for MCP stdio lifecycle reliability testing.

USAGE:
    mcp-lifecycle [--help | --version]

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

Development status: Phase 0 foundation. No MCP server is launched.
";

/// Handle the foundation CLI arguments.
///
/// The CLI is deliberately limited to help and version output until later
/// phases add profile validation and an explicitly gated run command.
pub fn run_cli<I, S, O, E>(args: I, out: &mut O, err: &mut E) -> u8
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    O: Write,
    E: Write,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();

    match args.as_slice() {
        [] => write_help(out, err),
        [flag] if flag == "--help" || flag == "-h" => write_help(out, err),
        [flag] if flag == "--version" || flag == "-V" => {
            if writeln!(out, "mcp-lifecycle {VERSION}").is_ok() {
                0
            } else {
                let _ = writeln!(err, "error: could not write version output");
                1
            }
        }
        _ => {
            let _ = writeln!(err, "error: unsupported argument");
            let _ = writeln!(err, "Run 'mcp-lifecycle --help' for usage.");
            2
        }
    }
}

fn write_help<O: Write, E: Write>(out: &mut O, err: &mut E) -> u8 {
    if write!(out, "{HELP}").is_ok() {
        0
    } else {
        let _ = writeln!(err, "error: could not write help output");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::{VERSION, run_cli};

    #[test]
    fn no_arguments_print_help() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let code = run_cli(std::iter::empty::<String>(), &mut out, &mut err);

        assert_eq!(code, 0);
        assert!(
            String::from_utf8(out)
                .expect("help is valid UTF-8")
                .contains("USAGE:")
        );
        assert!(err.is_empty());
    }

    #[test]
    fn version_flag_prints_package_version() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let code = run_cli(["--version"], &mut out, &mut err);

        assert_eq!(code, 0);
        assert_eq!(
            String::from_utf8(out).expect("version is valid UTF-8"),
            format!("mcp-lifecycle {VERSION}\n")
        );
        assert!(err.is_empty());
    }

    #[test]
    fn unknown_arguments_return_usage_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let code = run_cli(["--run"], &mut out, &mut err);

        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert!(
            String::from_utf8(err)
                .expect("error is valid UTF-8")
                .contains("unsupported argument")
        );
    }
}
