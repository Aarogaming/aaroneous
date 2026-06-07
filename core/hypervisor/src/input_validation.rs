// Lightweight input validation helpers for HTTP request bodies and
// task inputs.
//
// The codebase already has a richer validator in `security_hardener`
// (regex-based, JSON-schema aware) and `config_validation` (TOML
// manifest validation). This module fills the small, hot-path gap:
// when a handler receives a `String`, a `Vec<u8>`, or a numeric
// field, it can call `validate_*` to reject obviously bad input
// before doing anything expensive. Each function returns a
// `Result<T, ValidationError>` with a stable error string so the
// caller can convert to a 400 response.

use std::fmt;

/// Single error type for all helpers in this module. The variants
/// are flat: callers usually just need to know "this is invalid"
/// and the message string. Splitting into a tree of subtypes
/// would be more precise but adds noise to handler code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ValidationError {}

impl ValidationError {
    fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
    }
}

/// Reject empty strings and strings above `max_len` bytes. Whitespace
/// at the edges is allowed (do not pre-trim); the handler can decide
/// whether to trim before passing to logic that compares values.
pub fn validate_string(field: &str, value: &str, max_len: usize) -> Result<String, ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::new(format!("{}: must not be empty", field)));
    }
    if value.len() > max_len {
        return Err(ValidationError::new(format!(
            "{}: length {} exceeds max {}",
            field,
            value.len(),
            max_len
        )));
    }
    // Reject ASCII control characters and NULs early. UTF-8 multi-byte
    // sequences are not collapsed; if the caller wants unicode-aware
    // length, they should count chars before calling.
    if value.chars().any(|c| c.is_control()) {
        return Err(ValidationError::new(format!(
            "{}: contains control characters",
            field
        )));
    }
    Ok(value.to_string())
}

/// Like `validate_string` but allows empty (for optional fields).
pub fn validate_optional_string(
    field: &str,
    value: Option<&str>,
    max_len: usize,
) -> Result<Option<String>, ValidationError> {
    match value {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => Ok(Some(validate_string(field, s, max_len)?)),
    }
}

/// Restrict a numeric input to `[min, max]`. Generic over any integer
/// or float type via `PartialOrd`.
pub fn validate_range<T: PartialOrd + fmt::Display + Copy>(
    field: &str,
    value: T,
    min: T,
    max: T,
) -> Result<T, ValidationError> {
    if value < min || value > max {
        return Err(ValidationError::new(format!(
            "{}: {} not in [{}, {}]",
            field, value, min, max
        )));
    }
    Ok(value)
}

/// Restrict a byte buffer to a maximum size. Use for binary payloads
/// where `Vec<u8>::len()` is the natural size measure.
pub fn validate_bytes<'a>(field: &str, value: &'a [u8], max_len: usize) -> Result<&'a [u8], ValidationError> {
    if value.len() > max_len {
        return Err(ValidationError::new(format!(
            "{}: length {} exceeds max {}",
            field,
            value.len(),
            max_len
        )));
    }
    Ok(value)
}

/// Validate a free-form identifier (specialist name, model name,
/// link name): must be non-empty, ASCII alphanumeric + `_-.:`,
/// length 1..=128. The character set is the minimum needed to
/// support the namespaces Aaroneous uses today; if a future feature
/// needs more (e.g. `@`), widen the set here.
pub fn validate_identifier(field: &str, value: &str) -> Result<String, ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::new(format!("{}: must not be empty", field)));
    }
    if value.len() > 128 {
        return Err(ValidationError::new(format!(
            "{}: length {} exceeds max 128",
            field,
            value.len()
        )));
    }
    let bad = value
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')));
    if let Some(c) = bad {
        return Err(ValidationError::new(format!(
            "{}: invalid character {:?}",
            field, c
        )));
    }
    Ok(value.to_string())
}

/// Reject `value` if it is not one of the allowed enum variants
/// (compared by string). `field` is the caller-supplied name for
/// error messages; `allowed` is the set of legal values.
pub fn validate_enum<'a>(field: &str, value: &'a str, allowed: &'a [&'a str]) -> Result<&'a str, ValidationError> {
    if allowed.iter().any(|a| *a == value) {
        Ok(value)
    } else {
        Err(ValidationError::new(format!(
            "{}: {} not in [{}]",
            field,
            value,
            allowed.join(", ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_accepts_normal() {
        assert_eq!(
            validate_string("name", "hello", 100).unwrap(),
            "hello".to_string()
        );
    }

    #[test]
    fn string_rejects_empty() {
        assert!(validate_string("name", "", 100).is_err());
    }

    #[test]
    fn string_rejects_too_long() {
        assert!(validate_string("name", &"x".repeat(101), 100).is_err());
    }

    #[test]
    fn string_rejects_control_chars() {
        assert!(validate_string("name", "a\nb", 100).is_err());
    }

    #[test]
    fn optional_string_treats_empty_as_none() {
        assert!(validate_optional_string("name", None, 100).unwrap().is_none());
        assert!(validate_optional_string("name", Some(""), 100).unwrap().is_none());
        assert_eq!(
            validate_optional_string("name", Some("x"), 100).unwrap(),
            Some("x".to_string())
        );
    }

    #[test]
    fn range_accepts_in_bounds() {
        assert_eq!(validate_range("p", 5, 0, 10).unwrap(), 5);
        assert_eq!(validate_range("p", 0.5f64, 0.0, 1.0).unwrap(), 0.5);
    }

    #[test]
    fn range_rejects_out_of_bounds() {
        assert!(validate_range("p", 11, 0, 10).is_err());
        assert!(validate_range("p", -1, 0, 10).is_err());
    }

    #[test]
    fn bytes_rejects_oversize() {
        assert!(validate_bytes("payload", &[0u8; 100], 50).is_err());
        assert!(validate_bytes("payload", &[0u8; 50], 50).is_ok());
    }

    #[test]
    fn identifier_accepts_typical_names() {
        assert!(validate_identifier("model", "merlin-v1").is_ok());
        assert!(validate_identifier("model", "genome.reg_v2").is_ok());
        assert!(validate_identifier("model", "link:webhook-1").is_ok());
    }

    #[test]
    fn identifier_rejects_bad_chars() {
        assert!(validate_identifier("model", "merlin/v1").is_err());
        assert!(validate_identifier("model", "merlin v1").is_err());
        assert!(validate_identifier("model", "merlin@v1").is_err());
    }

    #[test]
    fn identifier_rejects_too_long() {
        assert!(validate_identifier("model", &"x".repeat(129)).is_err());
    }

    #[test]
    fn enum_accepts_known() {
        assert_eq!(
            validate_enum("kind", "genome", &["genome", "tensor", "model"]).unwrap(),
            "genome"
        );
    }

    #[test]
    fn enum_rejects_unknown() {
        assert!(validate_enum("kind", "wat", &["genome", "tensor", "model"]).is_err());
    }
}
