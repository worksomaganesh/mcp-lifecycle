use super::{CURRENT_SCHEMA_VERSION, MAX_PROFILE_FILE_BYTES, ProfileError};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use toml::Table;

const PROFILE_FIELDS: &[&str] = &[
    "schema_version",
    "name",
    "executable",
    "args",
    "working_directory",
    "environment_allowlist",
    "protocol_mode",
    "limits",
];

const LIMIT_FIELDS: &[&str] = &[
    "execution_timeout_ms",
    "graceful_shutdown_timeout_ms",
    "stdout_capture_bytes",
    "stderr_capture_bytes",
    "max_protocol_frame_bytes",
    "max_lifecycle_cycles",
];

/// Opaque TOML-deserialized data. Its private field prevents callers from
/// constructing an unvalidated profile directly.
pub struct UnvalidatedProfile {
    pub(super) raw: RawProfile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawProfile {
    pub(super) schema_version: i64,
    pub(super) name: String,
    pub(super) executable: PathBuf,
    pub(super) args: Vec<String>,
    pub(super) working_directory: PathBuf,
    pub(super) environment_allowlist: Vec<String>,
    pub(super) protocol_mode: String,
    pub(super) limits: RawLimits,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawLimits {
    pub(super) execution_timeout_ms: i64,
    pub(super) graceful_shutdown_timeout_ms: i64,
    pub(super) stdout_capture_bytes: i64,
    pub(super) stderr_capture_bytes: i64,
    pub(super) max_protocol_frame_bytes: i64,
    pub(super) max_lifecycle_cycles: i64,
}

pub(super) fn parse(input: &str) -> Result<UnvalidatedProfile, ProfileError> {
    if input.len() > MAX_PROFILE_FILE_BYTES {
        return Err(ProfileError::ProfileTooLarge {
            maximum_bytes: MAX_PROFILE_FILE_BYTES,
        });
    }

    // Inspect a TOML value tree first so errors can identify the field and
    // expected type without printing a potentially sensitive supplied value.
    let value: toml::Value = toml::from_str(input).map_err(|error| {
        let position = error.span().map(|span| span.start);
        invalid_toml(input, position)
    })?;
    let profile = value.as_table().ok_or(ProfileError::InvalidDocumentShape)?;

    check_table_fields(profile, "profile", PROFILE_FIELDS)?;
    check_field_types(profile)?;

    let version = profile
        .get("schema_version")
        .and_then(toml::Value::as_integer)
        .ok_or(ProfileError::WrongType {
            field: "schema_version",
            expected: "an integer",
        })?;
    if version != CURRENT_SCHEMA_VERSION {
        return Err(ProfileError::UnsupportedSchemaVersion { version });
    }

    let raw = value
        .try_into::<RawProfile>()
        .map_err(|_| ProfileError::InvalidDocumentShape)?;
    Ok(UnvalidatedProfile { raw })
}

pub(super) fn read(path: &Path) -> Result<UnvalidatedProfile, ProfileError> {
    let file =
        File::open(path).map_err(|error| ProfileError::ReadFailure { kind: error.kind() })?;
    let mut bytes = Vec::with_capacity(MAX_PROFILE_FILE_BYTES + 1);
    file.take((MAX_PROFILE_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| ProfileError::ReadFailure { kind: error.kind() })?;

    if bytes.len() > MAX_PROFILE_FILE_BYTES {
        return Err(ProfileError::ProfileTooLarge {
            maximum_bytes: MAX_PROFILE_FILE_BYTES,
        });
    }

    let input = String::from_utf8(bytes).map_err(|_| ProfileError::InvalidUtf8)?;
    parse(&input)
}

fn check_table_fields(
    table: &Table,
    section: &'static str,
    allowed: &[&'static str],
) -> Result<(), ProfileError> {
    if let Some(unknown) = table.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(ProfileError::UnknownField {
            section,
            field: unknown.clone(),
        });
    }

    for &field in allowed {
        if !table.contains_key(field) {
            return Err(ProfileError::MissingField { field });
        }
    }

    Ok(())
}

fn check_field_types(profile: &Table) -> Result<(), ProfileError> {
    expect_integer(profile, "schema_version")?;
    expect_string(profile, "name")?;
    expect_string(profile, "executable")?;
    expect_string_array(profile, "args")?;
    expect_string(profile, "working_directory")?;
    expect_string_array(profile, "environment_allowlist")?;
    expect_string(profile, "protocol_mode")?;

    let limits = profile
        .get("limits")
        .and_then(toml::Value::as_table)
        .ok_or(ProfileError::WrongType {
            field: "limits",
            expected: "a table",
        })?;
    check_table_fields(limits, "limits", LIMIT_FIELDS)?;
    for field in LIMIT_FIELDS {
        expect_integer(limits, field)?;
    }

    Ok(())
}

fn expect_integer(table: &Table, field: &'static str) -> Result<(), ProfileError> {
    if table.get(field).and_then(toml::Value::as_integer).is_some() {
        Ok(())
    } else {
        Err(ProfileError::WrongType {
            field,
            expected: "an integer",
        })
    }
}

fn expect_string(table: &Table, field: &'static str) -> Result<(), ProfileError> {
    if table.get(field).and_then(toml::Value::as_str).is_some() {
        Ok(())
    } else {
        Err(ProfileError::WrongType {
            field,
            expected: "a string",
        })
    }
}

fn expect_string_array(table: &Table, field: &'static str) -> Result<(), ProfileError> {
    let values =
        table
            .get(field)
            .and_then(toml::Value::as_array)
            .ok_or(ProfileError::WrongType {
                field,
                expected: "an array of strings",
            })?;

    if values.iter().all(toml::Value::is_str) {
        Ok(())
    } else {
        Err(ProfileError::WrongType {
            field,
            expected: "an array of strings",
        })
    }
}

fn invalid_toml(input: &str, byte_offset: Option<usize>) -> ProfileError {
    let Some(mut offset) = byte_offset else {
        return ProfileError::InvalidToml {
            line: None,
            column: None,
        };
    };
    offset = offset.min(input.len());
    while !input.is_char_boundary(offset) {
        offset -= 1;
    }
    let prefix = &input[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit_once('\n').map_or_else(
        || prefix.chars().count() + 1,
        |(_, tail)| tail.chars().count() + 1,
    );
    ProfileError::InvalidToml {
        line: Some(line),
        column: Some(column),
    }
}
