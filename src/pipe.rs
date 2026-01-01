//! Pipe transformation module

use crate::spec::PipeCommand;
use regex::{Regex, RegexBuilder};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

static REGEX_CACHE: LazyLock<RwLock<HashMap<String, Regex>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// ReDoS protection limits
const REGEX_SIZE_LIMIT: usize = 1_000_000;
const REGEX_DFA_SIZE_LIMIT: usize = 1_000_000;

fn get_cached_regex(pattern: &str) -> Result<Regex, anyhow::Error> {
    {
        let cache = REGEX_CACHE
            .read()
            .map_err(|_| anyhow::anyhow!("Regex cache lock poisoned"))?;
        if let Some(re) = cache.get(pattern) {
            return Ok(re.clone());
        }
    }

    let re = RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_DFA_SIZE_LIMIT)
        .build()
        .map_err(|e| anyhow::anyhow!("Invalid or unsafe regex '{}': {}", pattern, e))?;

    let mut cache = REGEX_CACHE
        .write()
        .map_err(|_| anyhow::anyhow!("Regex cache lock poisoned"))?;
    cache.insert(pattern.to_string(), re.clone());
    Ok(re)
}

pub fn apply_pipes(value: &str, pipes: &[PipeCommand]) -> Result<Value, anyhow::Error> {
    pipes
        .iter()
        .try_fold(Value::String(value.to_string()), |current, pipe| {
            apply_pipe(current, pipe)
        })
}

pub fn apply_pipe(value: Value, pipe: &PipeCommand) -> Result<Value, anyhow::Error> {
    match pipe {
        PipeCommand::Trim => string_transform(value, |s| s.trim().to_string()),
        PipeCommand::Lower => string_transform(value, |s| s.to_lowercase()),
        PipeCommand::Upper => string_transform(value, |s| s.to_uppercase()),
        PipeCommand::Substr(start, end) => apply_substring(value, *start, *end),
        PipeCommand::ParseAsNumber | PipeCommand::ParseAsFloat => apply_parse_number(value),
        PipeCommand::ParseAsInt => apply_parse_int(value),
        PipeCommand::Regex(pattern) => apply_regex(value, pattern),
        PipeCommand::Attr(_) => Ok(value),
    }
}

/// Helper to apply a string-to-string transformation
fn string_transform<F>(value: Value, f: F) -> Result<Value, anyhow::Error>
where
    F: FnOnce(&str) -> String,
{
    let s = as_string(&value)?;
    Ok(Value::String(f(s)))
}

/// Apply substring transformation
fn apply_substring(value: Value, start: usize, end: Option<usize>) -> Result<Value, anyhow::Error> {
    let s = as_string(&value)?;
    let result = match end {
        Some(e) => s.chars().take(e).skip(start).collect(),
        None => s.chars().skip(start).collect(),
    };
    Ok(Value::String(result))
}

/// Parse string as floating-point number
fn apply_parse_number(value: Value) -> Result<Value, anyhow::Error> {
    let s = as_string(&value)?;
    let n: f64 = s
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Cannot parse '{}' as number", s))?;
    Ok(Value::from(n))
}

/// Parse string as integer
fn apply_parse_int(value: Value) -> Result<Value, anyhow::Error> {
    let s = as_string(&value)?;
    let n: i64 = s
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Cannot parse '{}' as int", s))?;
    Ok(Value::from(n))
}

/// Apply regex extraction with optional capture group
fn apply_regex(value: Value, pattern: &str) -> Result<Value, anyhow::Error> {
    let s = as_string(&value)?;
    let re = get_cached_regex(pattern)?;

    match re.captures(s) {
        Some(caps) => Ok(caps
            .get(1)
            .or_else(|| caps.get(0))
            .map(|m| Value::String(m.as_str().to_string()))
            .unwrap_or(Value::Null)),
        None => Ok(Value::Null),
    }
}

/// Extract string from JSON value with consistent error messaging
fn as_string(value: &Value) -> Result<&str, anyhow::Error> {
    value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Expected string value"))
}
