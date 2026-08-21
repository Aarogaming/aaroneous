use anyhow::Result;
use serde_json::Value;
/// Config Validation Framework — validates configuration files against schemas.
///
/// Provides a lightweight schema definition system for validating JSON/TOML
/// config files at load time. Supports:
/// - Required field checks
/// - Type validation (string, number, bool, array, object)
/// - Range constraints (min/max for numbers, min_length for strings)
/// - Enum validation (allowed values)
/// - Nested object validation
/// - Custom validators via closures
use std::collections::HashMap;

/// Validation result for a single field.
#[derive(Debug, Clone)]
pub struct FieldResult {
    pub path: String,
    pub valid: bool,
    pub message: String,
}

/// Overall validation result.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<FieldResult>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn fail(errors: Vec<FieldResult>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, path: &str, message: &str) {
        self.errors.push(FieldResult {
            path: path.to_string(),
            valid: false,
            message: message.to_string(),
        });
        self.valid = false;
    }

    pub fn add_warning(&mut self, message: &str) {
        self.warnings.push(message.to_string());
    }
}

/// A schema rule for validating a field.
pub enum SchemaRule {
    /// Field must exist
    Required,
    /// Field must be a string
    IsString,
    /// Field must be a number
    IsNumber,
    /// Field must be a boolean
    IsBool,
    /// Field must be an array
    IsArray,
    /// Field must be an object
    IsObject,
    /// Number must be >= min
    Min(f64),
    /// Number must be <= max
    Max(f64),
    /// String must have length >= min
    MinLength(usize),
    /// String must have length <= max
    MaxLength(usize),
    /// Value must be one of these
    OneOf(Vec<Value>),
    /// Array items must all be strings
    ArrayOfStrings,
    /// Nested object must match sub-schema
    ObjectSchema(HashMap<String, Vec<SchemaRule>>),
    /// Custom validator function
    Custom(String, Box<dyn Fn(&Value) -> bool + Send + Sync>),
}

impl std::fmt::Debug for SchemaRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Required => write!(f, "Required"),
            Self::IsString => write!(f, "IsString"),
            Self::IsNumber => write!(f, "IsNumber"),
            Self::IsBool => write!(f, "IsBool"),
            Self::IsArray => write!(f, "IsArray"),
            Self::IsObject => write!(f, "IsObject"),
            Self::Min(v) => write!(f, "Min({})", v),
            Self::Max(v) => write!(f, "Max({})", v),
            Self::MinLength(n) => write!(f, "MinLength({})", n),
            Self::MaxLength(n) => write!(f, "MaxLength({})", n),
            Self::OneOf(vals) => write!(f, "OneOf({:?})", vals),
            Self::ArrayOfStrings => write!(f, "ArrayOfStrings"),
            Self::ObjectSchema(_) => write!(f, "ObjectSchema(...)"),
            Self::Custom(name, _) => write!(f, "Custom({})", name),
        }
    }
}

/// A schema definition for validating a JSON value.
#[derive(Debug)]
pub struct Schema {
    pub fields: HashMap<String, Vec<SchemaRule>>,
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Add a field with rules.
    pub fn field(mut self, name: &str, rules: Vec<SchemaRule>) -> Self {
        self.fields.insert(name.to_string(), rules);
        self
    }

    /// Validate a JSON value against this schema.
    pub fn validate(&self, value: &Value) -> ValidationResult {
        let mut result = ValidationResult::ok();

        for (field_name, rules) in &self.fields {
            let field_value = value.get(field_name);

            for rule in rules {
                match rule {
                    SchemaRule::Required => {
                        if field_value.is_none() {
                            result.add_error(field_name, "required field is missing");
                        }
                    }
                    _ => {
                        if let Some(v) = field_value {
                            self.validate_rule(field_name, v, rule, &mut result);
                        }
                    }
                }
            }
        }

        result
    }

    fn validate_rule(
        &self,
        field_name: &str,
        value: &Value,
        rule: &SchemaRule,
        result: &mut ValidationResult,
    ) {
        match rule {
            SchemaRule::Required => {} // Already handled
            SchemaRule::IsString => {
                if !value.is_string() {
                    result.add_error(field_name, "expected string");
                }
            }
            SchemaRule::IsNumber => {
                if !value.is_number() {
                    result.add_error(field_name, "expected number");
                }
            }
            SchemaRule::IsBool => {
                if !value.is_boolean() {
                    result.add_error(field_name, "expected boolean");
                }
            }
            SchemaRule::IsArray => {
                if !value.is_array() {
                    result.add_error(field_name, "expected array");
                }
            }
            SchemaRule::IsObject => {
                if !value.is_object() {
                    result.add_error(field_name, "expected object");
                }
            }
            SchemaRule::Min(min) => {
                if let Some(n) = value.as_f64()
                    && n < *min
                {
                    result.add_error(
                        &format!("{}.{}", "value", field_name),
                        &format!("{} is less than minimum {}", n, min),
                    );
                }
            }
            SchemaRule::Max(max) => {
                if let Some(n) = value.as_f64()
                    && n > *max
                {
                    result.add_error(
                        &format!("{}.{}", "value", field_name),
                        &format!("{} exceeds maximum {}", n, max),
                    );
                }
            }
            SchemaRule::MinLength(min) => {
                if let Some(s) = value.as_str()
                    && s.len() < *min
                {
                    result.add_error(
                        field_name,
                        &format!("length {} is less than minimum {}", s.len(), min),
                    );
                }
            }
            SchemaRule::MaxLength(max) => {
                if let Some(s) = value.as_str()
                    && s.len() > *max
                {
                    result.add_error(
                        field_name,
                        &format!("length {} exceeds maximum {}", s.len(), max),
                    );
                }
            }
            SchemaRule::OneOf(allowed) => {
                if !allowed.contains(value) {
                    result.add_error(field_name, "value not in allowed set");
                }
            }
            SchemaRule::ArrayOfStrings => {
                if let Some(arr) = value.as_array() {
                    for (i, item) in arr.iter().enumerate() {
                        if !item.is_string() {
                            result.add_error(
                                &format!("{}[{}]", field_name, i),
                                "expected string in array",
                            );
                        }
                    }
                }
            }
            SchemaRule::ObjectSchema(sub_schema) => {
                if let Some(obj) = value.as_object() {
                    for (sub_field, sub_rules) in sub_schema {
                        let sub_value = obj.get(sub_field.as_str());
                        for sub_rule in sub_rules {
                            match sub_rule {
                                SchemaRule::Required => {
                                    if sub_value.is_none() {
                                        result.add_error(
                                            &format!("{}.{}", field_name, sub_field),
                                            "required nested field is missing",
                                        );
                                    }
                                }
                                _ => {
                                    if let Some(v) = sub_value {
                                        self.validate_rule(
                                            &format!("{}.{}", field_name, sub_field),
                                            v,
                                            sub_rule,
                                            result,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            SchemaRule::Custom(name, validator) => {
                if !validator(value) {
                    result.add_error(field_name, &format!("custom validation '{}' failed", name));
                }
            }
        }
    }
}

/// Load and validate a JSON config file against a schema.
pub fn validate_config_file(path: &std::path::Path, schema: &Schema) -> Result<ValidationResult> {
    let json_str = std::fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&json_str)?;
    Ok(schema.validate(&value))
}

/// Validate a JSON string directly.
pub fn validate_json_str(json_str: &str, schema: &Schema) -> Result<ValidationResult> {
    let value: Value = serde_json::from_str(json_str)?;
    Ok(schema.validate(&value))
}

/// Pre-built schemas for common Aaroneous config files.
/// Schema for specialist_registry.json
pub fn specialist_registry_schema() -> Schema {
    Schema::new()
        .field("version", vec![SchemaRule::Required, SchemaRule::IsString])
        .field(
            "specialists",
            vec![SchemaRule::Required, SchemaRule::IsObject],
        )
}

/// Schema for genome manifest
pub fn genome_manifest_schema() -> Schema {
    Schema::new()
        .field(
            "schema_version",
            vec![SchemaRule::Required, SchemaRule::IsString],
        )
        .field(
            "identity_version",
            vec![SchemaRule::Required, SchemaRule::IsString],
        )
        .field(
            "designation",
            vec![
                SchemaRule::Required,
                SchemaRule::IsString,
                SchemaRule::MinLength(1),
            ],
        )
        .field(
            "primary_directive",
            vec![
                SchemaRule::Required,
                SchemaRule::IsString,
                SchemaRule::MinLength(1),
            ],
        )
        .field(
            "core_values",
            vec![
                SchemaRule::Required,
                SchemaRule::IsArray,
                SchemaRule::ArrayOfStrings,
            ],
        )
        .field(
            "invariants",
            vec![
                SchemaRule::Required,
                SchemaRule::IsArray,
                SchemaRule::ArrayOfStrings,
            ],
        )
}

/// Schema for epigenetic profile
pub fn epigenetic_profile_schema() -> Schema {
    Schema::new()
        .field(
            "schema_version",
            vec![SchemaRule::Required, SchemaRule::IsString],
        )
        .field(
            "profile_name",
            vec![SchemaRule::Required, SchemaRule::IsString],
        )
        .field("preset", vec![SchemaRule::Required, SchemaRule::IsString])
        .field(
            "spectrums",
            vec![SchemaRule::Required, SchemaRule::IsObject],
        )
        .field(
            "policy_clamps",
            vec![SchemaRule::Required, SchemaRule::IsObject],
        )
}

/// Schema for WASM registry index
pub fn wasm_registry_schema() -> Schema {
    Schema::new()
        .field(
            "schema_version",
            vec![SchemaRule::Required, SchemaRule::IsString],
        )
        .field(
            "components",
            vec![SchemaRule::Required, SchemaRule::IsObject],
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_field_missing() {
        let schema = Schema::new()
            .field("name", vec![SchemaRule::Required])
            .field("value", vec![SchemaRule::Required, SchemaRule::IsNumber]);

        let json = r#"{"name": "test"}"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let result = schema.validate(&value);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.path == "value"));
    }

    #[test]
    fn test_type_validation() {
        let schema = Schema::new()
            .field("count", vec![SchemaRule::Required, SchemaRule::IsNumber])
            .field("label", vec![SchemaRule::Required, SchemaRule::IsString]);

        let json = r#"{"count": "not_a_number", "label": 123}"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let result = schema.validate(&value);

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_range_validation() {
        let schema = Schema::new().field(
            "port",
            vec![
                SchemaRule::Required,
                SchemaRule::IsNumber,
                SchemaRule::Min(1.0),
                SchemaRule::Max(65535.0),
            ],
        );

        let valid_json = r#"{"port": 8080}"#;
        let result = schema.validate(&serde_json::from_str(valid_json).unwrap());
        assert!(result.valid);

        let invalid_json = r#"{"port": 0}"#;
        let result = schema.validate(&serde_json::from_str(invalid_json).unwrap());
        assert!(!result.valid);
    }

    #[test]
    fn test_enum_validation() {
        let schema = Schema::new().field(
            "level",
            vec![
                SchemaRule::Required,
                SchemaRule::OneOf(vec![
                    Value::String("debug".into()),
                    Value::String("info".into()),
                    Value::String("warn".into()),
                    Value::String("error".into()),
                ]),
            ],
        );

        let valid = schema.validate(&serde_json::from_str(r#"{"level": "info"}"#).unwrap());
        assert!(valid.valid);

        let invalid = schema.validate(&serde_json::from_str(r#"{"level": "verbose"}"#).unwrap());
        assert!(!invalid.valid);
    }

    #[test]
    fn test_nested_object_validation() {
        let schema = Schema::new()
            .field("server", vec![SchemaRule::Required, SchemaRule::IsObject])
            .field("name", vec![SchemaRule::Required, SchemaRule::IsString]);

        let valid = schema.validate(
            &serde_json::from_str(r#"{"server": {"host": "localhost"}, "name": "test"}"#).unwrap(),
        );
        assert!(valid.valid);
    }

    #[test]
    fn test_genome_manifest_schema() {
        let schema = genome_manifest_schema();

        let valid = r#"{
            "schema_version": "1.0",
            "identity_version": "v1.0.0",
            "designation": "TestAgent",
            "primary_directive": "Test directive",
            "core_values": ["stability"],
            "invariants": ["never_crash"]
        }"#;

        let result = schema.validate(&serde_json::from_str(valid).unwrap());
        assert!(result.valid);
    }
}
