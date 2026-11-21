use std::io::Write;
use std::path::Path;

pub mod profile;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
MCP Lifecycle & Leak Lab
Local-first tooling for MCP stdio lifecycle reliability testing.

USAGE:
    mcp-lifecycle [--help | --version]
    mcp-lifecycle validate <profile.toml>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
    validate         Parse and validate a local profile; never launches it

Development status: Phase 1 profile validation. No MCP server is launched.
";

/// Handle the implemented CLI arguments without launching a configured target.
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
        [command, path] if command == "validate" => validate_profile(path, out, err),
        [command] if command == "validate" => {
            let _ = writeln!(err, "error: validate requires a profile path");
            let _ = writeln!(err, "Usage: mcp-lifecycle validate <profile.toml>");
            2
        }
        _ => {
            let _ = writeln!(err, "error: unsupported argument");
            let _ = writeln!(err, "Run 'mcp-lifecycle --help' for usage.");
            2
        }
    }
}

fn validate_profile<O: Write, E: Write>(path: &str, out: &mut O, err: &mut E) -> u8 {
    match profile::load_and_validate_profile(Path::new(path)) {
        Ok(profile) => {
            if writeln!(
                out,
                "Profile is valid (schema version {}).",
                profile.schema_version()
            )
            .is_ok()
            {
                0
            } else {
                let _ = writeln!(err, "error: could not write validation result");
                1
            }
        }
        Err(error) => {
            let _ = writeln!(err, "error: {error}");
            1
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
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEMPORARY_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn new() -> Self {
            let sequence = NEXT_TEMPORARY_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "mcp-lifecycle-cli-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create unique CLI test directory");
            Self(path)
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

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

    #[test]
    fn validate_without_a_profile_path_returns_usage_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let code = run_cli(["validate"], &mut out, &mut err);

        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert!(
            String::from_utf8(err)
                .expect("error is valid UTF-8")
                .contains("validate requires a profile path")
        );
    }

    #[test]
    fn validate_command_does_not_execute_the_configured_executable() {
        let directory = TemporaryDirectory::new();
        let trigger = directory.0.join("mcp-lifecycle-execution-trigger");
        let marker = directory.0.join("mcp-lifecycle-executable-was-run");
        fs::write(&trigger, b"trigger").expect("write Rust executable sentinel trigger");
        let executable = std::env::current_exe().expect("get Rust test executable path");

        let profile = directory.0.join("profile.toml");
        let profile_text = format!(
            "schema_version = 1\n\
             name = \"no-execution-test\"\n\
             executable = {}\n\
             args = {}\n\
             working_directory = {}\n\
             environment_allowlist = []\n\
             protocol_mode = \"modern\"\n\
             \n\
             [limits]\n\
             execution_timeout_ms = 1000\n\
             graceful_shutdown_timeout_ms = 100\n\
             stdout_capture_bytes = 1024\n\
             stderr_capture_bytes = 1024\n\
             max_protocol_frame_bytes = 1024\n\
             max_lifecycle_cycles = 1\n",
            toml::Value::String(
                executable
                    .to_str()
                    .expect("Rust test executable path is Unicode")
                    .to_owned(),
            ),
            toml::Value::Array(vec![
                toml::Value::String("--exact".to_owned()),
                toml::Value::String("tests::configured_executable_sentinel".to_owned()),
            ]),
            toml::Value::String(directory.0.to_string_lossy().into_owned()),
        );
        fs::write(&profile, profile_text).expect("write valid profile fixture");

        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_cli(
            [
                "validate".to_owned(),
                profile
                    .to_str()
                    .expect("temporary path is Unicode")
                    .to_owned(),
            ],
            &mut out,
            &mut err,
        );

        assert_eq!(code, 0);
        assert_eq!(
            String::from_utf8(out).expect("validation output is UTF-8"),
            "Profile is valid (schema version 1).\n"
        );
        assert!(err.is_empty());
        assert!(trigger.is_file(), "the sentinel trigger remains present");
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
        while !marker.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            !marker.exists(),
            "validation must not execute the configured Rust test binary"
        );
    }

    #[test]
    fn validate_command_returns_nonzero_and_field_context_for_invalid_profiles() {
        let directory = TemporaryDirectory::new();
        let profile = directory.0.join("invalid-profile.toml");
        let profile_text = format!(
            "schema_version = 1\n\
             name = \"invalid-profile-test\"\n\
             executable = {}\n\
             args = []\n\
             working_directory = {}\n\
             environment_allowlist = []\n\
             protocol_mode = \"modern\"\n\
             \n\
             [limits]\n\
             execution_timeout_ms = 1\n\
             graceful_shutdown_timeout_ms = 100\n\
             stdout_capture_bytes = 1024\n\
             stderr_capture_bytes = 1024\n\
             max_protocol_frame_bytes = 1024\n\
             max_lifecycle_cycles = 1\n",
            toml::Value::String(
                directory
                    .0
                    .join("unlaunched-server.exe")
                    .to_string_lossy()
                    .into_owned()
            ),
            toml::Value::String(directory.0.to_string_lossy().into_owned()),
        );
        fs::write(&profile, profile_text).expect("write invalid profile fixture");

        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_cli(
            [
                "validate".to_owned(),
                profile
                    .to_str()
                    .expect("temporary path is Unicode")
                    .to_owned(),
            ],
            &mut out,
            &mut err,
        );

        assert_eq!(code, 1);
        assert!(out.is_empty());
        let error = String::from_utf8(err).expect("error output is UTF-8");
        assert!(error.contains("limits.execution_timeout_ms"));
        assert!(!error.contains("unlaunched-server.exe"));
    }

    #[test]
    fn configured_executable_sentinel() {
        if std::path::Path::new("mcp-lifecycle-execution-trigger").is_file() {
            fs::write("mcp-lifecycle-executable-was-run", b"executed")
                .expect("write configured executable sentinel marker");
        }
    }
}
