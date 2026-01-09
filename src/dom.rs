//! DOM module wrapping scraper (html5ever) for HTML parsing and CSS selector matching
//!
//! Parses HTML once and reuses the parsed document for all selections.

use ego_tree::NodeId;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;

/// A DOM node/element
///
/// Lightweight reference to a node in the DOM tree.
/// Only stores NodeId + DOM reference; everything else is lazy-computed and cached.
#[derive(Debug, Clone)]
pub struct Node {
    /// Node ID in the DOM tree (O(1) lookup via tree.get())
    node_id: NodeId,
    /// Cached text content
    text: OnceLock<String>,
    /// Cached HTML content
    html: OnceLock<String>,
    /// Reference to the DOM tree
    dom_html: Rc<Html>,
}

// Implement PartialEq for easier testing
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
    }
}
impl Eq for Node {}

impl Node {
    /// Returns the text content of this element
    pub fn text(&self) -> &str {
        self.text.get_or_init(|| {
            // Fast path: get ElementRef directly without going through Result
            let Some(el) = self
                .dom_html
                .tree
                .get(self.node_id)
                .and_then(ElementRef::wrap)
            else {
                return String::new();
            };

            let text_content = el.text().collect::<String>();
            // For void elements, check if next sibling is a text node
            if text_content.is_empty() && is_void_element(el.value().name()) {
                get_void_text_from_tree(el, &self.dom_html).unwrap_or(text_content)
            } else {
                text_content
            }
        })
    }

    /// Returns the value of the specified attribute
    pub fn attr(&self, name: &str) -> Option<&str> {
        // Fast path: get ElementRef directly
        let el = self
            .dom_html
            .tree
            .get(self.node_id)
            .and_then(ElementRef::wrap)?;
        el.value().attrs().find(|(k, _)| *k == name).map(|(_, v)| v)
    }

    /// Returns the HTML string of this element (cached)
    pub fn html(&self) -> &str {
        self.html.get_or_init(|| {
            // Fast path: get ElementRef directly
            self.dom_html
                .tree
                .get(self.node_id)
                .and_then(ElementRef::wrap)
                .map(|el| el.html().to_string())
                .unwrap_or_default()
        })
    }

    /// Get the ElementRef for this node (O(1) lookup by NodeId)
    pub(crate) fn element_ref(&self) -> Result<ElementRef<'_>, anyhow::Error> {
        self.dom_html
            .tree
            .get(self.node_id)
            .and_then(ElementRef::wrap)
            .ok_or_else(|| anyhow::anyhow!("Element not found in DOM tree"))
    }
}

/// DOM parser - parses HTML once and reuses for all queries
#[derive(Debug, Clone)]
pub struct Dom {
    /// Parsed HTML document
    html: Rc<Html>,
}

impl Dom {
    /// Parse HTML string into a DOM
    pub fn parse(source: &str) -> Result<Self, anyhow::Error> {
        Ok(Self {
            html: Rc::new(Html::parse_fragment(source)),
        })
    }

    /// Query selector - returns first matching element
    pub fn query_selector(&self, selector_str: &str) -> Result<Option<Node>, anyhow::Error> {
        let selector = Selector::parse(selector_str)
            .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", selector_str, e))?;
        Ok(self
            .html
            .select(&selector)
            .next()
            .map(|el| node_from_element(el, self.html.clone())))
    }

    /// Query selector all - returns all matching elements
    pub fn query_selector_all(&self, selector_str: &str) -> Result<Vec<Node>, anyhow::Error> {
        let selector = Selector::parse(selector_str)
            .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", selector_str, e))?;
        Ok(self
            .html
            .select(&selector)
            .map(|el| node_from_element(el, self.html.clone()))
            .collect())
    }

    /// Query selector relative to a base element
    pub fn query_selector_relative(
        &self,
        base: &Node,
        selector_str: &str,
    ) -> Result<Option<Node>, anyhow::Error> {
        let selector = Selector::parse(selector_str)
            .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", selector_str, e))?;
        let base_el = base.element_ref()?;
        Ok(base_el
            .select(&selector)
            .next()
            .map(|el| node_from_element(el, self.html.clone())))
    }

    /// Query selector all relative to a base element
    pub fn query_selector_all_relative(
        &self,
        base: &Node,
        selector_str: &str,
    ) -> Result<Vec<Node>, anyhow::Error> {
        let selector = Selector::parse(selector_str)
            .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", selector_str, e))?;
        let base_el = base.element_ref()?;
        Ok(base_el
            .select(&selector)
            .map(|el| node_from_element(el, self.html.clone()))
            .collect())
    }

    /// Extract JSON data from this DOM using a spec
    ///
    /// This is the main extraction method that applies the spec to the parsed HTML.
    pub fn extract(&self, spec: &crate::spec::Spec) -> Result<serde_json::Value, anyhow::Error> {
        match spec {
            crate::spec::Spec::Object(obj_spec) => self.extract_object(obj_spec, None),
            crate::spec::Spec::Array(arr_spec) => self.extract_array(arr_spec, None),
            crate::spec::Spec::Literal(lit) => Ok(self.literal_to_json(lit)),
        }
    }

    /// Extract an object from the DOM
    fn extract_object(
        &self,
        spec: &crate::spec::ObjectSpec,
        scope_node: Option<&Node>,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let scope = self.resolve_scope(&spec.scope_selector, scope_node)?;
        let result = spec
            .fields
            .iter()
            .map(|(key, field): (&String, &crate::spec::Field)| {
                self.extract_field(&field.spec, scope.as_ref())
                    .map(|value| (key.clone(), value, field.optional))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Filter out null optional fields and recursively clean nested objects
        let cleaned = Self::filter_optional_fields(result);

        Ok(serde_json::Value::Object(cleaned))
    }

    /// Extract an object from fields (helper to avoid cloning)
    fn extract_object_from_fields(
        &self,
        fields: &HashMap<String, crate::spec::Field>,
        scope: Option<&Node>,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let result = fields
            .iter()
            .map(|(key, field): (&String, &crate::spec::Field)| {
                self.extract_field(&field.spec, scope)
                    .map(|value| (key.clone(), value, field.optional))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Filter out null optional fields and recursively clean nested objects
        let cleaned = Self::filter_optional_fields(result);
        Ok(serde_json::Value::Object(cleaned))
    }

    /// Filter out null optional fields and recursively clean nested objects
    ///
    /// Returns a map with null optional fields removed.
    /// Nested objects with all null fields are also removed.
    fn filter_optional_fields(
        fields: Vec<(String, serde_json::Value, bool)>,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut result = serde_json::Map::new();

        for (key, value, optional) in fields {
            match value {
                // Null values: include only if not optional
                serde_json::Value::Null if optional => continue,
                serde_json::Value::Null => {
                    result.insert(key, value);
                }
                // Objects: recursively clean and insert if non-empty
                serde_json::Value::Object(_) => {
                    let cleaned = Self::recursively_clean_object(value);
                    if !cleaned.is_null() {
                        result.insert(key, cleaned);
                    }
                }
                // Arrays: recursively clean each item
                serde_json::Value::Array(arr) => {
                    let cleaned: Vec<_> = arr
                        .into_iter()
                        .map(Self::recursively_clean_object)
                        .collect();
                    // Only insert if array has items or is not optional
                    if !cleaned.is_empty() || !optional {
                        result.insert(key, serde_json::Value::Array(cleaned));
                    }
                }
                // All other values: always include
                _ => {
                    result.insert(key, value);
                }
            }
        }

        result
    }

    /// Recursively clean an object by removing null values
    ///
    /// Returns null if the object becomes empty after cleaning.
    fn recursively_clean_object(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(obj) => {
                let mut cleaned = serde_json::Map::new();
                for (k, v) in obj {
                    let cleaned_v = Self::recursively_clean_object(v);
                    // Keep non-null values
                    if !cleaned_v.is_null() {
                        cleaned.insert(k, cleaned_v);
                    }
                }
                if cleaned.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::Object(cleaned)
                }
            }
            serde_json::Value::Array(arr) => {
                let cleaned: Vec<_> = arr
                    .into_iter()
                    .map(Self::recursively_clean_object)
                    .filter(|v| !v.is_null())
                    .collect();
                serde_json::Value::Array(cleaned)
            }
            v => v,
        }
    }

    /// Extract an array from the DOM
    fn extract_array(
        &self,
        spec: &crate::spec::ArraySpec,
        scope: Option<&Node>,
    ) -> Result<serde_json::Value, anyhow::Error> {
        const DIRECT_CHILD_PREFIX: char = '>';

        // Special case: self-selector in array context
        let is_self_ref = spec
            .item_spec
            .scope_selector
            .as_ref()
            .map(|s: &crate::spec::SelectorRef| s.as_str() == "$")
            .unwrap_or(false);

        if is_self_ref && let Some(base) = scope {
            let obj = self.extract_object(&spec.item_spec, Some(base))?;
            return Ok(serde_json::Value::Array(vec![obj]));
        }

        // Get the effective selector
        let selector_str = spec
            .item_spec
            .scope_selector
            .as_ref()
            .map(|s: &crate::spec::SelectorRef| s.as_str())
            .unwrap_or("*");

        let effective_selector = selector_str
            .trim()
            .strip_prefix(DIRECT_CHILD_PREFIX)
            .map(|s: &str| s.trim())
            .unwrap_or(selector_str);

        let nodes = match scope {
            Some(base) => self.query_selector_all_relative(base, effective_selector)?,
            None => self.query_selector_all(effective_selector)?,
        };

        if nodes.is_empty() {
            return Ok(serde_json::Value::Array(Vec::new()));
        }

        let results = nodes
            .iter()
            .map(|node| self.extract_object_from_fields(&spec.item_spec.fields, Some(node)))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(serde_json::Value::Array(results))
    }

    /// Extract a single field value
    fn extract_field(
        &self,
        spec: &crate::spec::FieldSpec,
        scope: Option<&Node>,
    ) -> Result<serde_json::Value, anyhow::Error> {
        match spec {
            crate::spec::FieldSpec::Literal(lit) => Ok(self.literal_to_json(lit)),
            crate::spec::FieldSpec::Nested(obj_spec) => self.extract_object(obj_spec, scope),
            crate::spec::FieldSpec::NestedArray(arr_spec) => self.extract_array(arr_spec, scope),
            crate::spec::FieldSpec::Selector(selector_ref, pipes) => {
                let node = self.select_node(selector_ref, scope)?;
                Self::apply_pipes_to_node(node, pipes)
            }
            crate::spec::FieldSpec::FallbackSelector(selectors) => {
                self.extract_fallback_selector(selectors, scope)
            }
        }
    }

    /// Select a node based on a selector and optional scope
    fn select_node(
        &self,
        selector: &crate::spec::SelectorRef,
        scope: Option<&Node>,
    ) -> Result<Option<Node>, anyhow::Error> {
        const NEXT_SIBLING_PREFIX: &str = "+ ";
        const DIRECT_CHILD_PREFIX: char = '>';

        if selector.as_str() == "$" {
            return Ok(scope.cloned());
        }

        // Handle next sibling selector
        if let Some(inner) = selector.as_str().strip_prefix(NEXT_SIBLING_PREFIX) {
            let Some(base) = scope else {
                return Err(anyhow::anyhow!("Next sibling selector requires a scope"));
            };
            let inner_sel = Selector::parse(inner)
                .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", inner, e))?;
            let base_el = base.element_ref()?;
            for sibling in base_el.next_siblings() {
                if let Some(sib_el) = ElementRef::wrap(sibling)
                    && let Some(first_match) = sib_el.select(&inner_sel).next()
                {
                    return Ok(Some(node_from_element(first_match, self.html.clone())));
                }
            }
            return Ok(None);
        }

        // Handle direct child selector
        if selector.as_str().starts_with(DIRECT_CHILD_PREFIX) {
            let effective = selector.as_str()[1..].trim();
            return match scope {
                Some(base) => self.query_selector_relative(base, effective),
                None => self.query_selector(effective),
            };
        }

        // Regular selector
        let selector_str = selector.as_str();
        match scope {
            Some(base) => self.query_selector_relative(base, selector_str),
            None => self.query_selector(selector_str),
        }
    }

    /// Resolve a scope selector to a Node
    fn resolve_scope(
        &self,
        selector: &Option<crate::spec::SelectorRef>,
        base: Option<&Node>,
    ) -> Result<Option<Node>, anyhow::Error> {
        let Some(selector) = selector else {
            return Ok(base.cloned());
        };

        if selector.as_str() == "$" {
            Ok(base.cloned())
        } else if selector.as_str().starts_with('>') {
            let effective = selector.as_str()[1..].trim();
            match base {
                Some(b) => self.query_selector_relative(b, effective),
                None => self.query_selector(effective),
            }
        } else {
            let selector_str = selector.as_str();
            match base {
                Some(b) => self.query_selector_relative(b, selector_str),
                None => self.query_selector(selector_str),
            }
        }
    }

    /// Convert a literal value to JSON
    fn literal_to_json(&self, lit: &crate::spec::LiteralValue) -> serde_json::Value {
        match lit {
            crate::spec::LiteralValue::String(s) => serde_json::Value::String(s.clone()),
            crate::spec::LiteralValue::Number(n) => serde_json::Value::from(*n),
            crate::spec::LiteralValue::Boolean(b) => serde_json::Value::from(*b),
            crate::spec::LiteralValue::Null => serde_json::Value::Null,
        }
    }

    /// Apply pipe transformations to a node
    fn apply_pipes_to_node(
        node: Option<Node>,
        pipes: &[crate::spec::PipeCommand],
    ) -> Result<serde_json::Value, anyhow::Error> {
        use crate::pipe::apply_pipe;
        use crate::spec::PipeCommand;

        let Some(n) = node else {
            return Ok(serde_json::Value::Null);
        };

        let (source_pipe, transform_pipes) = crate::pipe::split_source_and_transforms(pipes);

        let initial_value = match source_pipe {
            Some(PipeCommand::Attr(attr_name)) => n
                .attr(attr_name)
                .map(|s| serde_json::Value::String(s.to_string()))
                .unwrap_or(serde_json::Value::Null),
            Some(PipeCommand::Void) => {
                let text_content = n.text();
                if text_content.is_empty() && is_void_element_from_html(n.html()) {
                    get_void_text_from_html(n.html())
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::String(text_content.to_string()))
                } else {
                    serde_json::Value::String(text_content.to_string())
                }
            }
            None => serde_json::Value::String(n.text().to_string()),
            Some(_) => return Err(anyhow::anyhow!("Non-source pipe in source_pipe position")),
        };

        transform_pipes
            .into_iter()
            .try_fold(initial_value, apply_pipe)
    }

    /// Extract from fallback selectors - tries each in order until one produces a non-null result
    fn extract_fallback_selector(
        &self,
        selectors: &[(crate::spec::SelectorRef, Vec<crate::spec::PipeCommand>)],
        scope: Option<&Node>,
    ) -> Result<serde_json::Value, anyhow::Error> {
        for (selector_ref, pipes) in selectors {
            let node = self.select_node(selector_ref, scope)?;
            let result = Self::apply_pipes_to_node(node, pipes)?;

            // Check if we got a meaningful result (not null, not empty string)
            match &result {
                serde_json::Value::Null => continue,
                serde_json::Value::String(s) if s.trim().is_empty() => continue,
                _ => return Ok(result),
            }
        }

        // All selectors failed, return null
        Ok(serde_json::Value::Null)
    }
}

fn node_from_element(el: ElementRef, tree: Rc<Html>) -> Node {
    let node_id = el.id();

    Node {
        node_id,
        text: OnceLock::new(),
        html: OnceLock::new(),
        dom_html: tree,
    }
}

/// Get text content from void element's next sibling (for RSS/XML patterns)
fn get_void_text_from_tree(el: ElementRef, _tree: &Rc<Html>) -> Option<String> {
    // For void elements in RSS/XML, text often appears as next sibling
    let node_id = el.id();
    let tree = el.tree();

    // Find parent and look for next sibling after current element
    tree.get(node_id).and_then(|node_ref| {
        node_ref.parent().and_then(|parent_ref| {
            tree.get(parent_ref.id()).and_then(|parent_ref| {
                let mut found_current = false;

                for child_ref in parent_ref.children() {
                    if found_current {
                        // Found next sibling - check if it's a text node
                        if let Some(text) = child_ref.value().as_text() {
                            return Some(text.trim().to_string());
                        }
                        break;
                    }
                    if child_ref.id() == node_id {
                        found_current = true;
                    }
                }
                None
            })
        })
    })
}

/// Check if element name is an HTML void element
pub fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Check if HTML represents a void element
fn is_void_element_from_html(html: &str) -> bool {
    if let Some(tag_end) = html.find('>') {
        let tag = &html[1..tag_end.min(html.len())];
        let tag_name = tag.split_whitespace().next().unwrap_or("");
        is_void_element(tag_name)
    } else {
        false
    }
}

/// Extract text content from sibling node's HTML representation
fn get_void_text_from_html(html: &str) -> Option<String> {
    // Find the closing of the void element
    if let Some(close_pos) = html.find('>') {
        let after_open = &html[close_pos + 1..];

        // Look for text up to the next '<'
        if let Some(next_tag) = after_open.find('<') {
            let text = &after_open[..next_tag];
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}
