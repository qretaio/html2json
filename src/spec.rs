//! Spec parsing module
//!
//! This module handles parsing JSON extraction specs into a structured format.
//! The spec format supports:
//! - Object specs with optional scope selector (`$`)
//! - Array specs for extracting collections
//! - Literal values (strings, numbers, booleans)
//! - Pipe transformations for data manipulation

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// A CSS selector string
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorRef(String);

impl SelectorRef {
    /// Get the selector string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this is a self-reference selector ($)
    pub fn is_self_ref(&self) -> bool {
        self.0 == "$"
    }
}

/// Represents an extraction specification
#[derive(Debug, Clone)]
pub enum Spec {
    /// Extract a single value (object with key-value pairs)
    Object(ObjectSpec),
    /// Extract multiple values (array of objects)
    Array(ArraySpec),
    /// A literal value
    Literal(LiteralValue),
}

impl<'de> Deserialize<'de> for Spec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Spec::from_json(&value).map_err(serde::de::Error::custom)
    }
}

/// Object spec - map of keys to extractors
///
/// The scope_selector defines the base element(s) for all field extractions.
/// All selectors in fields are evaluated relative to this scope.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectSpec {
    pub scope_selector: Option<SelectorRef>,
    pub fields: HashMap<String, Field>,
}

/// A field specification with optional flag
///
/// Fields marked as optional (with `?` suffix) will be removed from output
/// if their value is null.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub spec: FieldSpec,
    pub optional: bool,
}

/// Array spec - extract all matching elements
///
/// The item_spec is applied to each matched element to produce an array of results.
#[derive(Debug, Clone, PartialEq)]
pub struct ArraySpec {
    pub item_spec: ObjectSpec,
}

/// Field specification
///
/// Defines how to extract a single field value from HTML.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldSpec {
    /// CSS selector with optional pipes
    Selector(SelectorRef, Vec<PipeCommand>),
    /// Fallback selectors - tries each in order until one produces a result
    FallbackSelector(Vec<(SelectorRef, Vec<PipeCommand>)>),
    /// Nested object
    Nested(ObjectSpec),
    /// Nested array
    NestedArray(ArraySpec),
    /// Literal value
    Literal(LiteralValue),
}

/// Pipe transformation command
///
/// Pipes are applied sequentially to transform extracted values.
#[derive(Debug, Clone, PartialEq)]
pub enum PipeCommand {
    Attr(String),
    Void,
    Trim,
    Lower,
    Upper,
    Substr(usize, Option<usize>),
    ParseAsNumber,
    ParseAsInt,
    ParseAsFloat,
    Regex(String),
}

/// Literal values
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

impl Spec {
    pub fn from_json(value: &Value) -> Result<Self, anyhow::Error> {
        match value {
            Value::Array(arr) if !arr.is_empty() => {
                let item_spec = Self::parse_object_spec(&arr[0])?;
                Ok(Spec::Array(ArraySpec { item_spec }))
            }
            Value::Object(_) => {
                let spec = Self::parse_object_spec(value)?;
                Ok(Spec::Object(spec))
            }
            _ => Ok(Spec::Object(ObjectSpec {
                scope_selector: None,
                fields: HashMap::new(),
            })),
        }
    }

    fn parse_object_spec(value: &Value) -> Result<ObjectSpec, anyhow::Error> {
        let obj = value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Expected object"))?;

        let mut scope_selector = None;
        let mut fields = HashMap::new();

        for (key, val) in obj {
            if key == "$" {
                if let Some(s) = val.as_str() {
                    scope_selector = Some(SelectorRef(s.to_string()));
                }
            } else {
                // Check if field is optional (ends with ?)
                let (field_name, optional) = if key.ends_with('?') {
                    (&key[..key.len() - 1], true)
                } else {
                    (key.as_str(), false)
                };

                fields.insert(
                    field_name.to_string(),
                    Field {
                        spec: FieldSpec::from_json(val)?,
                        optional,
                    },
                );
            }
        }

        Ok(ObjectSpec {
            scope_selector,
            fields,
        })
    }
}

impl FieldSpec {
    fn from_json(value: &Value) -> Result<Self, anyhow::Error> {
        match value {
            Value::String(s) => {
                if let Some(literal) = Self::parse_literal_string(s) {
                    return Ok(FieldSpec::Literal(literal));
                }
                Self::parse_selector_or_fallback(s)
            }
            Value::Number(n) => {
                let literal = LiteralValue::Number(n.as_f64().unwrap_or(0.0));
                Ok(FieldSpec::Literal(literal))
            }
            Value::Bool(b) => Ok(FieldSpec::Literal(LiteralValue::Boolean(*b))),
            Value::Null => Ok(FieldSpec::Literal(LiteralValue::Null)),
            Value::Array(arr) if !arr.is_empty() => {
                let item_spec = Spec::parse_object_spec(&arr[0])?;
                Ok(FieldSpec::NestedArray(ArraySpec { item_spec }))
            }
            Value::Object(_) => {
                let spec = Spec::parse_object_spec(value)?;
                Ok(FieldSpec::Nested(spec))
            }
            Value::Array(_) => Ok(FieldSpec::Literal(LiteralValue::Null)),
        }
    }

    /// Check if a string is a literal (single or double quoted)
    fn parse_literal_string(s: &str) -> Option<LiteralValue> {
        let trimmed = s.trim();

        if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
            || (trimmed.starts_with('"') && trimmed.ends_with('"'))
        {
            let inner = &trimmed[1..trimmed.len() - 1];
            return Some(LiteralValue::String(inner.to_string()));
        }

        None
    }

    /// Parse a selector string into base selector and pipe commands
    ///
    /// Formats supported:
    /// - "selector" -> ("selector", [])
    /// - "selector | pipe1 | pipe2" -> ("selector", [pipe1, pipe2])
    /// - "$ | pipe1" -> ("$", [pipe1])
    /// - "attr:name" -> ("$", [Attr("name")])  (implicit self-selector)
    fn parse_selector_string(s: &str) -> Result<(String, Vec<PipeCommand>), anyhow::Error> {
        let trimmed = s.trim();
        if trimmed == "$" {
            return Ok(("$".to_string(), Vec::new()));
        }

        let parts: Vec<&str> = trimmed.split('|').map(|p| p.trim()).collect();

        let (selector, pipe_start) = if parts[0].starts_with("attr:") {
            ("$".to_string(), 0)
        } else if parts[0] == "$" {
            ("$".to_string(), 1)
        } else {
            (parts[0].to_string(), 1)
        };

        let mut pipes = Vec::new();

        for part in &parts[pipe_start..] {
            if !part.is_empty() {
                pipes.push(Self::parse_pipe_command(part)?);
            }
        }

        Ok((selector, pipes))
    }

    fn parse_pipe_command(s: &str) -> Result<PipeCommand, anyhow::Error> {
        // Simple commands without arguments
        match s {
            "trim" | "text" => return Ok(PipeCommand::Trim),
            "lower" => return Ok(PipeCommand::Lower),
            "upper" => return Ok(PipeCommand::Upper),
            "void" => return Ok(PipeCommand::Void),
            "parseAs:number" => return Ok(PipeCommand::ParseAsNumber),
            "parseAs:int" => return Ok(PipeCommand::ParseAsInt),
            "parseAs:float" => return Ok(PipeCommand::ParseAsFloat),
            _ => {}
        }

        // Commands with arguments (using prefix-based dispatch)
        if let Some(rest) = s.strip_prefix("attr:") {
            return Ok(PipeCommand::Attr(rest.to_string()));
        }

        if let Some(rest) = s.strip_prefix("substr:") {
            return Self::parse_substr_command(rest);
        }

        if let Some(pattern) = s.strip_prefix("regex:") {
            return Ok(PipeCommand::Regex(pattern.to_string()));
        }

        Err(anyhow::anyhow!("Unknown pipe command: {}", s))
    }

    fn parse_substr_command(rest: &str) -> Result<PipeCommand, anyhow::Error> {
        let parts: Vec<&str> = rest.split(':').collect();
        let start: usize = parts[0]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid substr start: {}", parts[0]))?;

        let end = if parts.len() > 1 {
            Some(
                parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid substr end: {}", parts[1]))?,
            )
        } else {
            None
        };

        Ok(PipeCommand::Substr(start, end))
    }

    /// Parse a selector string, handling fallback selectors with ||
    ///
    /// - "selector" -> Selector
    /// - "selector || fallback" -> FallbackSelector with two options
    /// - "sel1 || sel2 || sel3" -> FallbackSelector with three options
    fn parse_selector_or_fallback(s: &str) -> Result<Self, anyhow::Error> {
        let trimmed = s.trim();

        // Check for || operator (fallback)
        if trimmed.contains("||") {
            let parts: Vec<&str> = trimmed.split("||").map(|p| p.trim()).collect();
            if parts.len() < 2 {
                return Err(anyhow::anyhow!("Invalid fallback selector"));
            }

            let mut selectors = Vec::new();
            for part in parts {
                let (selector, pipes) = Self::parse_selector_string(part)?;
                selectors.push((SelectorRef(selector), pipes));
            }

            return Ok(FieldSpec::FallbackSelector(selectors));
        }

        // Single selector
        let (selector, pipes) = Self::parse_selector_string(trimmed)?;
        Ok(FieldSpec::Selector(SelectorRef(selector), pipes))
    }
}
