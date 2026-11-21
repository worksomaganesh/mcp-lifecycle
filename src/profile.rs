//! Versioned TOML profiles and the validation gate used by the CLI.

mod schema;
mod validation;

#[cfg(test)]
mod tests;

use std::fmt;
use std::io;
use std::path::Path;

pub use schema::UnvalidatedProfile;
pub use validation::{ProfileLimits, ValidatedProfile};

/// The only profile schema version accepted by this release.
pub const CURRENT_SCHEMA_VERSION: i64 = 1;

/// Maximum size of a profile file or profile string, in bytes.
pub const MAX_PROFILE_FILE_BYTES: usize = 1024 * 1024;

/// Protocol lifecycle selected for a future MCP client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolMode {
    /// Stateless modern lifecycle, pinned by a later protocol implementation.
    Modern,
    /// Initialization-handshake lifecycle, pinned by a later protocol implementation.
    Legacy,
    /// Defer compatibility selection to a later protocol implementation.
    Auto,
}

impl ProtocolMode {
    pub(super) fn parse(value: &str) -> Result<Self, ProfileError> {
        match value {
            "modern" => Ok(Self::Modern),
            "legacy" => Ok(Self::Legacy),
            "auto" => Ok(Self::Auto),
            _ => Err(ProfileError::InvalidField {
                field: "protocol_mode",
                requirement: "must be one of: modern, legacy, auto",
            }),
        }
    }
}

/// A safe, structured profile error. Error messages never include profile values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileError {
    ReadFailure {
        kind: io::ErrorKind,
    },
    ProfileTooLarge {
        maximum_bytes: usize,
    },
    InvalidUtf8,
    InvalidToml {
        line: Option<usize>,
        column: Option<usize>,
    },
    InvalidDocumentShape,
    MissingField {
        field: &'static str,
    },
    UnknownField {
        section: &'static str,
        field: String,
    },
    WrongType {
        field: &'static str,
        expected: &'static str,
    },
    UnsupportedSchemaVersion {
        version: i64,
    },
    InvalidField {
        field: &'static str,
        requirement: &'static str,
    },
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailure { kind } => write!(
                f,
                "could not read profile file ({kind:?}); check that the path exists and is readable"
            ),
            Self::ProfileTooLarge { maximum_bytes } => write!(
                f,
                "profile exceeds the maximum size of {maximum_bytes} bytes"
            ),
            Self::InvalidUtf8 => f.write_str("profile must be valid UTF-8 TOML"),
            Self::InvalidToml {
                line: Some(line),
                column: Some(column),
            } => write!(f, "invalid TOML syntax at line {line}, column {column}"),
            Self::InvalidToml { .. } => f.write_str("invalid TOML syntax"),
            Self::InvalidDocumentShape => {
                f.write_str("profile document does not match the required schema shape")
            }
            Self::MissingField { field } => write!(f, "missing required field `{field}`"),
            Self::UnknownField { section, field } => {
                write!(f, "unknown field {} in {section}", escaped_key(field))
            }
            Self::WrongType { field, expected } => {
                write!(f, "`{field}` must be {expected}")
            }
            Self::UnsupportedSchemaVersion { version } => write!(
                f,
                "unsupported `schema_version` {version}; this release supports \
                 version {CURRENT_SCHEMA_VERSION}"
            ),
            Self::InvalidField { field, requirement } => {
                write!(f, "`{field}` {requirement}")
            }
        }
    }
}

impl std::error::Error for ProfileError {}

fn escaped_key(key: &str) -> String {
    let mut result = String::new();
    for character in key.chars().take(80) {
        result.extend(character.escape_default());
    }
    if key.chars().count() > 80 {
        result.push('…');
    }
    format!("`{result}`")
}

/// Parse the TOML representation into an opaque, unvalidated profile.
///
/// This performs bounded parsing and strict schema deserialization only. Call
/// [`UnvalidatedProfile::validate`] to obtain a value accepted by future run code.
pub fn parse_profile(input: &str) -> Result<UnvalidatedProfile, ProfileError> {
    schema::parse(input)
}

/// Read a bounded TOML profile file and deserialize its schema.
pub fn read_profile(path: &Path) -> Result<UnvalidatedProfile, ProfileError> {
    schema::read(path)
}

/// Read, deserialize, and semantically validate a profile without executing it.
pub fn load_and_validate_profile(path: &Path) -> Result<ValidatedProfile, ProfileError> {
    read_profile(path)?.validate()
}

impl UnvalidatedProfile {
    /// Apply semantic checks and consume this unvalidated value.
    pub fn validate(self) -> Result<ValidatedProfile, ProfileError> {
        validation::validate(self.raw)
    }
}
