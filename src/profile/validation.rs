use super::schema::RawProfile;
use super::{ProfileError, ProtocolMode};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;

pub(super) const MIN_EXECUTION_TIMEOUT_MS: i64 = 1_000;
pub(super) const MAX_EXECUTION_TIMEOUT_MS: i64 = 30 * 60 * 1_000;
pub(super) const MIN_GRACEFUL_SHUTDOWN_TIMEOUT_MS: i64 = 100;
pub(super) const MAX_GRACEFUL_SHUTDOWN_TIMEOUT_MS: i64 = 30 * 1_000;
pub(super) const MIN_CAPTURE_BYTES: i64 = KIB as i64;
pub(super) const MAX_STDOUT_CAPTURE_BYTES: i64 = (255 * MIB) as i64;
pub(super) const MAX_STDERR_CAPTURE_BYTES: i64 = MIB as i64;
pub(super) const MAX_TOTAL_CAPTURE_BYTES: u64 = 256 * MIB;
pub(super) const MIN_PROTOCOL_FRAME_BYTES: i64 = KIB as i64;
pub(super) const MAX_PROTOCOL_FRAME_BYTES: i64 = (8 * MIB) as i64;
pub(super) const MIN_LIFECYCLE_CYCLES: i64 = 1;
pub(super) const MAX_LIFECYCLE_CYCLES: i64 = 100;

const MAX_PROFILE_NAME_BYTES: usize = 128;
const MAX_PATH_BYTES: usize = 4 * 1024;
const MAX_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_BYTES: usize = 8 * 1024;
const MAX_TOTAL_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_ENVIRONMENT_NAMES: usize = 64;
const MAX_ENVIRONMENT_NAME_BYTES: usize = 128;

/// Semantic resource bounds accepted by profile schema version 1.
pub struct ProfileLimits {
    execution_timeout_ms: u64,
    graceful_shutdown_timeout_ms: u64,
    stdout_capture_bytes: u64,
    stderr_capture_bytes: u64,
    max_protocol_frame_bytes: u64,
    max_lifecycle_cycles: u32,
}

impl ProfileLimits {
    pub fn execution_timeout_ms(&self) -> u64 {
        self.execution_timeout_ms
    }

    pub fn graceful_shutdown_timeout_ms(&self) -> u64 {
        self.graceful_shutdown_timeout_ms
    }

    pub fn stdout_capture_bytes(&self) -> u64 {
        self.stdout_capture_bytes
    }

    pub fn stderr_capture_bytes(&self) -> u64 {
        self.stderr_capture_bytes
    }

    pub fn max_protocol_frame_bytes(&self) -> u64 {
        self.max_protocol_frame_bytes
    }

    pub fn max_lifecycle_cycles(&self) -> u32 {
        self.max_lifecycle_cycles
    }
}

/// A profile that passed schema and semantic validation.
///
/// Its fields are private and it deliberately does not implement `Debug`, so
/// accidental diagnostics cannot print arguments or other profile data.
pub struct ValidatedProfile {
    schema_version: u32,
    name: String,
    executable: PathBuf,
    args: Vec<String>,
    working_directory: PathBuf,
    environment_allowlist: Vec<String>,
    protocol_mode: ProtocolMode,
    limits: ProfileLimits,
}

impl ValidatedProfile {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// Names only. Values are resolved from the parent environment by a later phase.
    pub fn environment_allowlist(&self) -> &[String] {
        &self.environment_allowlist
    }

    pub fn protocol_mode(&self) -> ProtocolMode {
        self.protocol_mode
    }

    pub fn limits(&self) -> &ProfileLimits {
        &self.limits
    }
}

pub(super) fn validate(raw: RawProfile) -> Result<ValidatedProfile, ProfileError> {
    if raw.schema_version != super::CURRENT_SCHEMA_VERSION {
        return Err(ProfileError::UnsupportedSchemaVersion {
            version: raw.schema_version,
        });
    }

    validate_profile_name(&raw.name)?;
    validate_path(&raw.executable, "executable")?;
    validate_path(&raw.working_directory, "working_directory")?;
    validate_working_directory(&raw.working_directory)?;
    validate_arguments(&raw.args)?;
    validate_environment_allowlist(&raw.environment_allowlist)?;

    let protocol_mode = ProtocolMode::parse(&raw.protocol_mode)?;
    let limits = validate_limits(raw.limits)?;

    Ok(ValidatedProfile {
        schema_version: raw.schema_version as u32,
        name: raw.name,
        executable: raw.executable,
        args: raw.args,
        working_directory: raw.working_directory,
        environment_allowlist: raw.environment_allowlist,
        protocol_mode,
        limits,
    })
}

fn validate_profile_name(name: &str) -> Result<(), ProfileError> {
    if name.is_empty() || name.trim().is_empty() {
        return Err(ProfileError::InvalidField {
            field: "name",
            requirement: "must not be empty or whitespace-only",
        });
    }
    if name.len() > MAX_PROFILE_NAME_BYTES {
        return Err(ProfileError::InvalidField {
            field: "name",
            requirement: "must be at most 128 UTF-8 bytes",
        });
    }
    if name.trim() != name || name.chars().any(char::is_control) {
        return Err(ProfileError::InvalidField {
            field: "name",
            requirement: "must not have surrounding whitespace or control characters",
        });
    }
    Ok(())
}

fn validate_path(path: &Path, field: &'static str) -> Result<(), ProfileError> {
    let value = path.to_str().ok_or(ProfileError::InvalidField {
        field,
        requirement: "must be valid Unicode",
    })?;
    if value.is_empty() || value.len() > MAX_PATH_BYTES {
        return Err(ProfileError::InvalidField {
            field,
            requirement: "must be between 1 and 4096 UTF-8 bytes",
        });
    }
    if value.as_bytes().contains(&0) {
        return Err(ProfileError::InvalidField {
            field,
            requirement: "must not contain NUL bytes",
        });
    }
    if value.chars().any(char::is_control) {
        return Err(ProfileError::InvalidField {
            field,
            requirement: "must not contain control characters",
        });
    }
    if !path.is_absolute() {
        return Err(ProfileError::InvalidField {
            field,
            requirement: "must be an absolute path for the current operating system",
        });
    }
    Ok(())
}

fn validate_working_directory(path: &Path) -> Result<(), ProfileError> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        _ => Err(ProfileError::InvalidField {
            field: "working_directory",
            requirement: "must name an existing directory",
        }),
    }
}

fn validate_arguments(args: &[String]) -> Result<(), ProfileError> {
    if args.len() > MAX_ARGUMENTS {
        return Err(ProfileError::InvalidField {
            field: "args",
            requirement: "must contain no more than 128 arguments",
        });
    }

    let mut total_bytes = 0usize;
    for argument in args {
        if argument.as_bytes().contains(&0) {
            return Err(ProfileError::InvalidField {
                field: "args",
                requirement: "entries must not contain NUL bytes",
            });
        }
        if argument.len() > MAX_ARGUMENT_BYTES {
            return Err(ProfileError::InvalidField {
                field: "args",
                requirement: "each argument must be at most 8192 UTF-8 bytes",
            });
        }
        total_bytes = total_bytes
            .checked_add(argument.len())
            .and_then(|size| size.checked_add(1))
            .ok_or(ProfileError::InvalidField {
                field: "args",
                requirement: "combined argument size must be at most 16384 UTF-8 bytes",
            })?;
    }
    if total_bytes > MAX_TOTAL_ARGUMENT_BYTES {
        return Err(ProfileError::InvalidField {
            field: "args",
            requirement: "combined argument size, including separators, must be at most \
                         16384 UTF-8 bytes",
        });
    }
    Ok(())
}

fn validate_environment_allowlist(names: &[String]) -> Result<(), ProfileError> {
    if names.len() > MAX_ENVIRONMENT_NAMES {
        return Err(ProfileError::InvalidField {
            field: "environment_allowlist",
            requirement: "must contain no more than 64 names",
        });
    }

    let mut seen = HashSet::with_capacity(names.len());
    for name in names {
        if name.is_empty()
            || name.len() > MAX_ENVIRONMENT_NAME_BYTES
            || !valid_environment_name(name)
        {
            return Err(ProfileError::InvalidField {
                field: "environment_allowlist",
                requirement: "entries must match [A-Za-z_][A-Za-z0-9_]* and be at most 128 bytes",
            });
        }

        #[cfg(windows)]
        let comparison_key = name.to_ascii_lowercase();
        #[cfg(not(windows))]
        let comparison_key = name.clone();

        if !seen.insert(comparison_key) {
            return Err(ProfileError::InvalidField {
                field: "environment_allowlist",
                requirement: "must not contain duplicate names (case-insensitive on Windows)",
            });
        }
    }
    Ok(())
}

fn valid_environment_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn validate_limits(raw: super::schema::RawLimits) -> Result<ProfileLimits, ProfileError> {
    let execution_timeout_ms = bounded_u64(
        "limits.execution_timeout_ms",
        raw.execution_timeout_ms,
        MIN_EXECUTION_TIMEOUT_MS,
        MAX_EXECUTION_TIMEOUT_MS,
    )?;
    let graceful_shutdown_timeout_ms = bounded_u64(
        "limits.graceful_shutdown_timeout_ms",
        raw.graceful_shutdown_timeout_ms,
        MIN_GRACEFUL_SHUTDOWN_TIMEOUT_MS,
        MAX_GRACEFUL_SHUTDOWN_TIMEOUT_MS,
    )?;
    let stdout_capture_bytes = bounded_u64(
        "limits.stdout_capture_bytes",
        raw.stdout_capture_bytes,
        MIN_CAPTURE_BYTES,
        MAX_STDOUT_CAPTURE_BYTES,
    )?;
    let stderr_capture_bytes = bounded_u64(
        "limits.stderr_capture_bytes",
        raw.stderr_capture_bytes,
        MIN_CAPTURE_BYTES,
        MAX_STDERR_CAPTURE_BYTES,
    )?;
    let max_protocol_frame_bytes = bounded_u64(
        "limits.max_protocol_frame_bytes",
        raw.max_protocol_frame_bytes,
        MIN_PROTOCOL_FRAME_BYTES,
        MAX_PROTOCOL_FRAME_BYTES,
    )?;
    let max_lifecycle_cycles = bounded_u64(
        "limits.max_lifecycle_cycles",
        raw.max_lifecycle_cycles,
        MIN_LIFECYCLE_CYCLES,
        MAX_LIFECYCLE_CYCLES,
    )? as u32;

    if stdout_capture_bytes
        .checked_add(stderr_capture_bytes)
        .is_none_or(|total| total > MAX_TOTAL_CAPTURE_BYTES)
    {
        return Err(ProfileError::InvalidField {
            field: "limits.stdout_capture_bytes + limits.stderr_capture_bytes",
            requirement: "must total no more than 268435456 bytes",
        });
    }

    Ok(ProfileLimits {
        execution_timeout_ms,
        graceful_shutdown_timeout_ms,
        stdout_capture_bytes,
        stderr_capture_bytes,
        max_protocol_frame_bytes,
        max_lifecycle_cycles,
    })
}

fn bounded_u64(
    field: &'static str,
    value: i64,
    minimum: i64,
    maximum: i64,
) -> Result<u64, ProfileError> {
    if !(minimum..=maximum).contains(&value) {
        return Err(ProfileError::InvalidField {
            field,
            requirement: match field {
                "limits.execution_timeout_ms" => "must be between 1000 and 1800000 milliseconds",
                "limits.graceful_shutdown_timeout_ms" => "must be between 100 and 30000 ms",
                "limits.stdout_capture_bytes" => "must be between 1024 and 267386880 bytes",
                "limits.stderr_capture_bytes" => "must be between 1024 and 1048576 bytes",
                "limits.max_protocol_frame_bytes" => "must be between 1024 and 8388608 bytes",
                "limits.max_lifecycle_cycles" => "must be between 1 and 100 cycles",
                _ => "is outside the supported range",
            },
        });
    }
    Ok(value as u64)
}
