use super::{
    CURRENT_SCHEMA_VERSION, MAX_PROFILE_FILE_BYTES, ProfileError, ProtocolMode,
    load_and_validate_profile, parse_profile, read_profile,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMPORARY_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEMPORARY_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "mcp-lifecycle-profile-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create unique profile test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn valid_profile(directory: &Path) -> String {
    let executable = directory.join("configured-server-that-is-never-launched.exe");
    format!(
        "schema_version = {CURRENT_SCHEMA_VERSION}\n\
         name = \"test server\"\n\
         executable = {}\n\
         args = [\"--stdio\", \"profile fixture\"]\n\
         working_directory = {}\n\
         environment_allowlist = [\"PATH\", \"HOME\"]\n\
         protocol_mode = \"modern\"\n\
         \n\
         [limits]\n\
         execution_timeout_ms = 1000\n\
         graceful_shutdown_timeout_ms = 100\n\
         stdout_capture_bytes = 1024\n\
         stderr_capture_bytes = 1024\n\
         max_protocol_frame_bytes = 1024\n\
         max_lifecycle_cycles = 1\n",
        toml::Value::String(executable.to_string_lossy().into_owned()),
        toml::Value::String(directory.to_string_lossy().into_owned()),
    )
}

fn replace_root_field(profile: &str, field: &str, value: &str) -> String {
    let prefix = format!("{field} = ");
    let mut replaced = false;
    profile
        .lines()
        .map(|line| {
            if !replaced && line.starts_with(&prefix) {
                replaced = true;
                format!("{prefix}{value}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn replace_limit(profile: &str, field: &str, value: i64) -> String {
    let prefix = format!("{field} = ");
    let mut in_limits = false;
    let mut replaced = false;
    profile
        .lines()
        .map(|line| {
            if line == "[limits]" {
                in_limits = true;
            }
            if in_limits && !replaced && line.starts_with(&prefix) {
                replaced = true;
                format!("{prefix}{value}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate(profile: &str) -> Result<super::ValidatedProfile, ProfileError> {
    parse_profile(profile)?.validate()
}

#[test]
fn accepts_a_valid_profile_and_exposes_only_validated_values() {
    let directory = TemporaryDirectory::new();
    let profile = validate(&valid_profile(directory.path())).expect("profile is valid");

    assert_eq!(profile.schema_version(), 1);
    assert_eq!(profile.name(), "test server");
    assert_eq!(profile.args().len(), 2);
    assert_eq!(profile.args()[0], "--stdio");
    assert_eq!(profile.args()[1], "profile fixture");
    assert_eq!(profile.protocol_mode(), ProtocolMode::Modern);
    assert_eq!(profile.limits().execution_timeout_ms(), 1000);
    assert_eq!(profile.limits().max_lifecycle_cycles(), 1);
}

#[test]
fn accepts_the_profile_name_and_executable_path_size_ceilings() {
    let directory = TemporaryDirectory::new();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "name",
        &toml::Value::String("N".repeat(128)).to_string(),
    );
    assert!(validate(&profile).is_ok());

    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "name",
        &toml::Value::String("N".repeat(129)).to_string(),
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField { field: "name", .. })
    ));

    #[cfg(windows)]
    let path_prefix = "C:\\";
    #[cfg(not(windows))]
    let path_prefix = "/";

    let maximum_path = format!("{path_prefix}{}", "a".repeat(4096 - path_prefix.len()));
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "executable",
        &toml::Value::String(maximum_path).to_string(),
    );
    assert!(validate(&profile).is_ok());

    let oversized_path = format!("{path_prefix}{}", "a".repeat(4097 - path_prefix.len()));
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "executable",
        &toml::Value::String(oversized_path).to_string(),
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField {
            field: "executable",
            ..
        })
    ));
}

#[test]
fn rejects_missing_required_fields() {
    let directory = TemporaryDirectory::new();
    let profile = valid_profile(directory.path())
        .lines()
        .filter(|line| !line.starts_with("name = "))
        .collect::<Vec<_>>()
        .join("\n");

    assert_eq!(
        parse_profile(&profile).err(),
        Some(ProfileError::MissingField { field: "name" })
    );

    let profile = valid_profile(directory.path())
        .lines()
        .filter(|line| !line.starts_with("max_lifecycle_cycles = "))
        .collect::<Vec<_>>()
        .join("\n");

    assert_eq!(
        parse_profile(&profile).err(),
        Some(ProfileError::MissingField {
            field: "max_lifecycle_cycles"
        })
    );
}

#[test]
fn rejects_unsupported_schema_versions() {
    let directory = TemporaryDirectory::new();
    let profile = replace_root_field(&valid_profile(directory.path()), "schema_version", "2");

    assert_eq!(
        parse_profile(&profile).err(),
        Some(ProfileError::UnsupportedSchemaVersion { version: 2 })
    );
}

#[test]
fn rejects_unknown_root_and_limits_fields() {
    let directory = TemporaryDirectory::new();
    let profile =
        valid_profile(directory.path()).replace("\n[limits]\n", "\nunknown_root = 1\n\n[limits]\n");
    assert!(matches!(
        parse_profile(&profile),
        Err(ProfileError::UnknownField {
            section: "profile",
            ..
        })
    ));

    let profile = format!("{}\nunknown_limit = 1\n", valid_profile(directory.path()));
    assert!(matches!(
        parse_profile(&profile),
        Err(ProfileError::UnknownField {
            section: "limits",
            ..
        })
    ));
}

#[test]
fn rejects_wrong_field_types_without_echoing_supplied_values() {
    let directory = TemporaryDirectory::new();
    let secret_sentinel = "ENVIRONMENT_SECRET_VALUE_SENTINEL";
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "environment_allowlist",
        &toml::Value::String(secret_sentinel.to_owned()).to_string(),
    );

    let error = parse_profile(&profile)
        .err()
        .expect("allowlist must be an array");
    assert_eq!(
        error,
        ProfileError::WrongType {
            field: "environment_allowlist",
            expected: "an array of strings"
        }
    );
    assert!(!error.to_string().contains(secret_sentinel));
}

#[test]
fn rejects_relative_executable_and_working_directory_paths() {
    let directory = TemporaryDirectory::new();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "executable",
        "\"relative/server.exe\"",
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField {
            field: "executable",
            ..
        })
    ));

    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "working_directory",
        "\"relative\"",
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField {
            field: "working_directory",
            ..
        })
    ));
}

#[test]
fn rejects_a_missing_or_non_directory_working_directory() {
    let directory = TemporaryDirectory::new();
    let file_path = directory.path().join("not-a-directory");
    fs::write(&file_path, b"fixture").expect("create regular file");

    for path in [directory.path().join("missing"), file_path] {
        let profile = replace_root_field(
            &valid_profile(directory.path()),
            "working_directory",
            &toml::Value::String(path.to_string_lossy().into_owned()).to_string(),
        );
        assert!(matches!(
            validate(&profile),
            Err(ProfileError::InvalidField {
                field: "working_directory",
                ..
            })
        ));
    }
}

#[test]
fn accepts_only_declared_protocol_modes() {
    let directory = TemporaryDirectory::new();
    for (value, expected) in [
        ("modern", ProtocolMode::Modern),
        ("legacy", ProtocolMode::Legacy),
        ("auto", ProtocolMode::Auto),
    ] {
        let profile = replace_root_field(
            &valid_profile(directory.path()),
            "protocol_mode",
            &toml::Value::String(value.to_owned()).to_string(),
        );
        assert_eq!(
            validate(&profile)
                .expect("declared protocol mode")
                .protocol_mode(),
            expected
        );
    }

    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "protocol_mode",
        "\"unknown\"",
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField {
            field: "protocol_mode",
            ..
        })
    ));
}

#[test]
fn checks_lower_upper_and_outside_values_for_every_numeric_limit() {
    let directory = TemporaryDirectory::new();
    let base = valid_profile(directory.path());
    let cases = [
        ("execution_timeout_ms", 1_000, 1_800_000),
        ("graceful_shutdown_timeout_ms", 100, 30_000),
        ("stdout_capture_bytes", 1_024, 267_386_880),
        ("stderr_capture_bytes", 1_024, 1_048_576),
        ("max_protocol_frame_bytes", 1_024, 8_388_608),
        ("max_lifecycle_cycles", 1, 100),
    ];

    for (field, minimum, maximum) in cases {
        for boundary in [minimum, maximum] {
            let profile = replace_limit(&base, field, boundary);
            assert!(
                validate(&profile).is_ok(),
                "{field}={boundary} should be accepted"
            );
        }
        for outside in [minimum - 1, maximum + 1] {
            let profile = replace_limit(&base, field, outside);
            assert!(
                matches!(validate(&profile), Err(ProfileError::InvalidField { .. })),
                "{field}={outside} should be rejected"
            );
        }
    }
}

#[test]
fn accepts_the_combined_output_capture_ceiling() {
    let directory = TemporaryDirectory::new();
    let profile = replace_limit(
        &replace_limit(
            &valid_profile(directory.path()),
            "stdout_capture_bytes",
            267_386_880,
        ),
        "stderr_capture_bytes",
        1_048_576,
    );
    let validated = validate(&profile).expect("combined capture ceiling is inclusive");
    assert_eq!(
        validated.limits().stdout_capture_bytes() + validated.limits().stderr_capture_bytes(),
        268_435_456
    );
}

#[test]
fn rejects_invalid_environment_names_and_duplicate_names() {
    let directory = TemporaryDirectory::new();
    for invalid in ["", "9STARTS_WITH_DIGIT", "HAS-DASH", "HAS SPACE"] {
        let profile = replace_root_field(
            &valid_profile(directory.path()),
            "environment_allowlist",
            &toml::Value::Array(vec![toml::Value::String(invalid.to_owned())]).to_string(),
        );
        assert!(matches!(
            validate(&profile),
            Err(ProfileError::InvalidField {
                field: "environment_allowlist",
                ..
            })
        ));
    }

    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "environment_allowlist",
        "[\"DUPLICATE\", \"DUPLICATE\"]",
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField {
            field: "environment_allowlist",
            ..
        })
    ));
}

#[cfg(windows)]
#[test]
fn detects_environment_duplicates_case_insensitively_on_windows() {
    let directory = TemporaryDirectory::new();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "environment_allowlist",
        "[\"Path\", \"PATH\"]",
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField {
            field: "environment_allowlist",
            ..
        })
    ));
}

#[cfg(not(windows))]
#[test]
fn accepts_environment_duplicates_with_case_sensitivity_on_non_windows() {
    let directory = TemporaryDirectory::new();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "environment_allowlist",
        "[\"Path\", \"PATH\"]",
    );
    assert!(validate(&profile).is_ok());
}

#[test]
fn rejects_excessive_argument_and_environment_counts_and_sizes() {
    let directory = TemporaryDirectory::new();
    let maximum_args = toml::Value::Array(
        (0..128)
            .map(|_| toml::Value::String("a".to_owned()))
            .collect(),
    )
    .to_string();
    let profile = replace_root_field(&valid_profile(directory.path()), "args", &maximum_args);
    assert!(validate(&profile).is_ok());

    let too_many_args = toml::Value::Array(
        (0..129)
            .map(|_| toml::Value::String("a".to_owned()))
            .collect(),
    )
    .to_string();
    let profile = replace_root_field(&valid_profile(directory.path()), "args", &too_many_args);
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField { field: "args", .. })
    ));

    let maximum_environment_count = toml::Value::Array(
        (0..64)
            .map(|index| toml::Value::String(format!("ENV_{index}")))
            .collect(),
    )
    .to_string();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "environment_allowlist",
        &maximum_environment_count,
    );
    assert!(validate(&profile).is_ok());

    let too_many_env = toml::Value::Array(
        (0..65)
            .map(|index| toml::Value::String(format!("ENV_{index}")))
            .collect(),
    )
    .to_string();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "environment_allowlist",
        &too_many_env,
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField {
            field: "environment_allowlist",
            ..
        })
    ));

    let oversized_argument =
        toml::Value::Array(vec![toml::Value::String("a".repeat(8193))]).to_string();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "args",
        &oversized_argument,
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField { field: "args", .. })
    ));

    let maximum_argument =
        toml::Value::Array(vec![toml::Value::String("a".repeat(8192))]).to_string();
    let profile = replace_root_field(&valid_profile(directory.path()), "args", &maximum_argument);
    assert!(validate(&profile).is_ok());

    let combined_argument_size = toml::Value::Array(vec![
        toml::Value::String("a".repeat(8192)),
        toml::Value::String("b".repeat(8192)),
    ])
    .to_string();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "args",
        &combined_argument_size,
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField { field: "args", .. })
    ));

    let maximum_combined_argument_size = toml::Value::Array(vec![
        toml::Value::String("a".repeat(8191)),
        toml::Value::String("b".repeat(8191)),
    ])
    .to_string();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "args",
        &maximum_combined_argument_size,
    );
    assert!(validate(&profile).is_ok());

    let long_environment_name =
        toml::Value::Array(vec![toml::Value::String(format!("A{}", "B".repeat(128)))]).to_string();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "environment_allowlist",
        &long_environment_name,
    );
    assert!(matches!(
        validate(&profile),
        Err(ProfileError::InvalidField {
            field: "environment_allowlist",
            ..
        })
    ));

    let maximum_environment_name =
        toml::Value::Array(vec![toml::Value::String(format!("A{}", "B".repeat(127)))]).to_string();
    let profile = replace_root_field(
        &valid_profile(directory.path()),
        "environment_allowlist",
        &maximum_environment_name,
    );
    assert!(validate(&profile).is_ok());
}

#[test]
fn rejects_nul_bytes_in_name_paths_arguments_and_environment_names() {
    let directory = TemporaryDirectory::new();
    let base = valid_profile(directory.path());
    let nul = toml::Value::String("before\0after".to_owned()).to_string();

    for (field, value) in [
        ("name", nul.clone()),
        ("executable", nul.clone()),
        ("working_directory", nul.clone()),
        (
            "args",
            toml::Value::Array(vec![toml::Value::String("before\0after".to_owned())]).to_string(),
        ),
        (
            "environment_allowlist",
            toml::Value::Array(vec![toml::Value::String("before\0after".to_owned())]).to_string(),
        ),
    ] {
        let profile = replace_root_field(&base, field, &value);
        assert!(
            matches!(validate(&profile), Err(ProfileError::InvalidField { .. })),
            "{field} containing NUL should be invalid"
        );
    }
}

#[test]
fn rejects_profiles_larger_than_the_documented_limit() {
    let too_large = format!("{}#{}", " ".repeat(MAX_PROFILE_FILE_BYTES), "x");
    assert_eq!(
        parse_profile(&too_large).err(),
        Some(ProfileError::ProfileTooLarge {
            maximum_bytes: MAX_PROFILE_FILE_BYTES
        })
    );
}

#[test]
fn bounded_file_reader_rejects_a_profile_over_the_documented_limit() {
    let directory = TemporaryDirectory::new();
    let path = directory.path().join("oversized-profile.toml");
    fs::write(&path, vec![b' '; MAX_PROFILE_FILE_BYTES + 1]).expect("write oversized profile");

    assert_eq!(
        read_profile(&path).err(),
        Some(ProfileError::ProfileTooLarge {
            maximum_bytes: MAX_PROFILE_FILE_BYTES
        })
    );
}

#[test]
fn reports_toml_syntax_location_without_echoing_source() {
    let secret_sentinel = "DO_NOT_PRINT_THIS_VALUE";
    let error = parse_profile(&format!("secret = \"{secret_sentinel}\"\n[broken"))
        .err()
        .expect("invalid TOML");
    assert!(matches!(error, ProfileError::InvalidToml { .. }));
    assert!(!error.to_string().contains(secret_sentinel));
}

#[test]
fn reads_and_validates_a_profile_file_without_checking_or_launching_executable() {
    let directory = TemporaryDirectory::new();
    let executable = directory.path().join("sentinel-not-an-executable");
    let sentinel_contents = b"MCP_LIFECYCLE_VALIDATION_SENTINEL: do not modify or execute";
    fs::write(&executable, sentinel_contents).expect("create inert sentinel file");

    let profile_text = replace_root_field(
        &valid_profile(directory.path()),
        "executable",
        &toml::Value::String(executable.to_string_lossy().into_owned()).to_string(),
    );
    let profile_path = directory.path().join("profile.toml");
    fs::write(&profile_path, profile_text).expect("write profile fixture");

    let validated = load_and_validate_profile(&profile_path).expect("profile validates");
    assert_eq!(validated.executable(), executable.as_path());
    let remaining_contents = fs::read(executable).expect("sentinel remains present");
    assert_eq!(remaining_contents.as_slice(), sentinel_contents.as_slice());
}

#[test]
fn reports_profile_file_read_failures_without_disclosing_the_path() {
    let directory = TemporaryDirectory::new();
    let missing = directory.path().join("missing-secret-path-profile.toml");
    let error = load_and_validate_profile(&missing)
        .err()
        .expect("profile path does not exist");

    assert!(matches!(error, ProfileError::ReadFailure { .. }));
    assert!(!error.to_string().contains("missing-secret-path-profile"));
}
