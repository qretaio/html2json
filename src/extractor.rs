//! Main extractor module
//!
//! This module performs the actual HTML extraction based on parsed specs.
//!
//! # Scoping System
//!
//! The scoping system allows nested extractions where inner selectors are
//! evaluated relative to an outer scope element:
//!
//! - `$` property defines the scope selector for an object
//! - Selectors starting with `>` are relative to the scope (direct children)
//! - Other selectors are also searched within the scope
//! - The `$` value as a selector refers to the scope element itself

use crate::dom::{Dom, Node};
use crate::pipe::apply_pipes;
use crate::spec::{ArraySpec, FieldSpec, LiteralValue, ObjectSpec, SelectorRef, Spec};
use serde_json::Value;

/// Special selector prefixes
const NEXT_SIBLING_PREFIX: &str = "+ ";
const DIRECT_CHILD_PREFIX: char = '>';

pub struct Extractor {
    dom: Dom,
}

impl Extractor {
    pub fn new(html: &str) -> Result<Self, anyhow::Error> {
        Ok(Self {
            dom: Dom::parse(html)?,
        })
    }

    pub fn extract(&self, spec: &Spec) -> Result<Value, anyhow::Error> {
        match spec {
            Spec::Object(obj_spec) => self.extract_object(obj_spec, None),
            Spec::Array(arr_spec) => self.extract_array(arr_spec, None),
            Spec::Literal(lit) => Ok(self.literal_to_json(lit)),
        }
    }

    fn literal_to_json(&self, lit: &LiteralValue) -> Value {
        match lit {
            LiteralValue::String(s) => Value::String(s.clone()),
            LiteralValue::Number(n) => Value::from(*n),
            LiteralValue::Boolean(b) => Value::from(*b),
            LiteralValue::Null => Value::Null,
        }
    }

    /// Extract an object, optionally within a scope
    ///
    /// The scope_node is the context element for relative selectors.
    /// If the spec has a scope_selector, it's first resolved to get the
    /// actual scope element, then all fields are extracted from it.
    fn extract_object(
        &self,
        spec: &ObjectSpec,
        scope_node: Option<&crate::dom::Node>,
    ) -> Result<Value, anyhow::Error> {
        let scope = self.resolve_scope(&spec.scope_selector, scope_node)?;
        let result = spec
            .fields
            .iter()
            .map(|(key, field_spec)| {
                self.extract_field(field_spec, scope.as_ref())
                    .map(|value| (key.clone(), value))
            })
            .collect::<Result<serde_json::Map<_, _>, _>>()?;

        Ok(Value::Object(result))
    }

    /// Resolve a scope selector to a Node
    fn resolve_scope(
        &self,
        selector: &Option<crate::spec::SelectorRef>,
        base: Option<&crate::dom::Node>,
    ) -> Result<Option<crate::dom::Node>, anyhow::Error> {
        let Some(selector) = selector else {
            return Ok(base.cloned());
        };

        if selector.is_self_ref() {
            Ok(base.cloned())
        } else if selector.as_str().starts_with(DIRECT_CHILD_PREFIX) {
            let effective = selector.as_str()[1..].trim();
            match base {
                Some(b) => self.dom.select_one_relative(b, effective),
                None => self.dom.select_one(effective),
            }
        } else {
            let parsed = selector.get()?;
            match base {
                Some(b) => self.dom.select_one_relative_with_selector(b, parsed),
                None => self.dom.select_one_with_selector(parsed),
            }
        }
    }

    /// Extract a single field value
    ///
    /// Handles selectors, nested objects, nested arrays, and literals.
    /// For attribute extraction, the first pipe must be `attr:name` which
    /// operates on the element itself rather than its text content.
    fn extract_field(
        &self,
        spec: &FieldSpec,
        scope: Option<&crate::dom::Node>,
    ) -> Result<Value, anyhow::Error> {
        match spec {
            FieldSpec::Literal(lit) => Ok(self.literal_to_json(lit)),
            FieldSpec::Nested(obj_spec) => self.extract_object(obj_spec, scope),
            FieldSpec::NestedArray(arr_spec) => self.extract_array(arr_spec, scope),
            FieldSpec::Selector(selector_ref, pipes) => {
                let has_attr_pipe = pipes
                    .iter()
                    .any(|p| matches!(p, crate::spec::PipeCommand::Attr(_)));

                let node = self.select_node(selector_ref, scope)?;
                Self::apply_pipes_opt(node, pipes, has_attr_pipe)
            }
        }
    }

    /// Select a node based on a selector and optional scope
    fn select_node(
        &self,
        selector: &SelectorRef,
        scope: Option<&Node>,
    ) -> Result<Option<Node>, anyhow::Error> {
        if selector.is_self_ref() {
            return Ok(scope.cloned());
        }

        // Handle next sibling selector: "+ .selector"
        if let Some(inner) = selector.as_str().strip_prefix(NEXT_SIBLING_PREFIX) {
            let Some(base) = scope else {
                return Err(anyhow::anyhow!(
                    "Next sibling selector '{NEXT_SIBLING_PREFIX}' requires a scope"
                ));
            };
            let sibling = self.dom.select_next_sibling(base, inner)?;
            // Apply the inner selector to the sibling
            return Ok(match sibling {
                Some(s) => self.dom.select_one_relative(&s, inner)?,
                None => None,
            });
        }

        // Handle direct child selector: "> selector"
        if selector.as_str().starts_with(DIRECT_CHILD_PREFIX) {
            let effective = selector.as_str()[1..].trim();
            return match scope {
                Some(base) => self.dom.select_one_relative(base, effective),
                None => self.dom.select_one(effective),
            };
        }

        // Regular selector - use parsed path
        let selector = selector.get()?;
        match scope {
            Some(base) => self.dom.select_one_relative_with_selector(base, selector),
            None => self.dom.select_one_with_selector(selector),
        }
    }

    /// Apply pipes to an optional node, returning Null if node is None
    fn apply_pipes_opt(
        node: Option<crate::dom::Node>,
        pipes: &[crate::spec::PipeCommand],
        has_attr_pipe: bool,
    ) -> Result<Value, anyhow::Error> {
        match node {
            Some(n) => Self::apply_pipes_to_node(n, pipes, has_attr_pipe),
            None => Ok(Value::Null),
        }
    }

    fn apply_pipes_to_node(
        node: crate::dom::Node,
        pipes: &[crate::spec::PipeCommand],
        has_attr_pipe: bool,
    ) -> Result<Value, anyhow::Error> {
        if has_attr_pipe {
            apply_pipes_with_attr(node, pipes)
        } else {
            apply_pipes(node.text(), pipes)
        }
    }

    /// Extract an array of elements matching the scope selector
    ///
    /// For each matched element, the item_spec is applied to extract fields.
    /// The item_spec's scope_selector is removed so each matched element
    /// becomes the scope for its own extraction.
    fn extract_array(
        &self,
        spec: &ArraySpec,
        scope: Option<&crate::dom::Node>,
    ) -> Result<Value, anyhow::Error> {
        // Special case: self-selector in array context creates single-element array
        let is_self_ref = spec
            .item_spec
            .scope_selector
            .as_ref()
            .map(|s| s.is_self_ref())
            .unwrap_or(false);

        if is_self_ref && let Some(base) = scope {
            let obj = self.extract_object(&spec.item_spec, Some(base))?;
            return Ok(Value::Array(vec![obj]));
        }

        // Get the effective selector (strip `>` prefix if present)
        let selector_str = spec
            .item_spec
            .scope_selector
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("*");

        let effective_selector = selector_str
            .trim()
            .strip_prefix(DIRECT_CHILD_PREFIX)
            .map(|s| s.trim())
            .unwrap_or(selector_str);

        // Select nodes relative to scope or from root
        let nodes = match scope {
            Some(base) => self.dom.select_relative(base, effective_selector)?,
            None => self.dom.select(effective_selector)?,
        };

        if nodes.is_empty() {
            return Ok(Value::Array(Vec::new()));
        }

        // Remove scope selector from item spec and extract each node
        let item_spec_without_scope = ObjectSpec {
            scope_selector: None,
            fields: spec.item_spec.fields.clone(),
        };

        let results = nodes
            .iter()
            .map(|node| self.extract_object(&item_spec_without_scope, Some(node)))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Value::Array(results))
    }
}

/// Apply pipes starting with attribute extraction
///
/// The first pipe must be `attr:name` which extracts from the element.
/// Subsequent pipes transform the attribute value.
fn apply_pipes_with_attr(
    node: crate::dom::Node,
    pipes: &[crate::spec::PipeCommand],
) -> Result<Value, anyhow::Error> {
    // Find the attr pipe index and split the pipes
    let attr_idx = pipes
        .iter()
        .position(|p| matches!(p, crate::spec::PipeCommand::Attr(_)))
        .ok_or_else(|| anyhow::anyhow!("attr pipe required"))?;

    let attr_name = match &pipes[attr_idx] {
        crate::spec::PipeCommand::Attr(name) => name,
        _ => unreachable!(),
    };

    // Extract attribute value
    let mut result = node
        .attr(attr_name)
        .map(|s| Value::String(s.to_string()))
        .unwrap_or(Value::Null);

    // Apply remaining pipes
    for pipe in &pipes[attr_idx + 1..] {
        result = crate::pipe::apply_pipe(result, pipe)?;
    }

    Ok(result)
}
