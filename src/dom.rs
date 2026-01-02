//! DOM module wrapping scraper (html5ever) for HTML parsing and CSS selector matching
//!
//! Parses HTML once and reuses the parsed document for all selections.

use scraper::{ElementRef, Html, Selector};
use std::sync::Arc;

/// A DOM node with extracted data
///
/// Stores element data for use in relative selections.
#[derive(Debug, Clone)]
pub struct Node {
    /// Element reference (stored as HTML for re-finding in relative selections)
    html: String,
    /// Cached text content
    text: String,
    /// Cached attributes
    attributes: Vec<(String, String)>,
}

impl Node {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn html(&self) -> &str {
        &self.html
    }
}

/// DOM parser - parses HTML once and reuses for all queries
#[derive(Debug)]
pub struct Dom {
    /// Parsed HTML document (owned, no lifetime issues in scraper 0.25+)
    html: Html,
}

impl Dom {
    pub fn parse(source: &str) -> Result<Self, anyhow::Error> {
        Ok(Self {
            html: Html::parse_fragment(source),
        })
    }

    pub fn select(&self, selector_str: &str) -> Result<Vec<Node>, anyhow::Error> {
        let selector = Selector::parse(selector_str)
            .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", selector_str, e))?;
        Ok(self
            .html
            .select(&selector)
            .map(|el| node_from_element(el, &self.html))
            .collect())
    }

    pub fn select_one(&self, selector_str: &str) -> Result<Option<Node>, anyhow::Error> {
        let selector = Selector::parse(selector_str)
            .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", selector_str, e))?;
        Ok(self
            .html
            .select(&selector)
            .next()
            .map(|el| node_from_element(el, &self.html)))
    }

    pub fn select_relative(
        &self,
        base: &Node,
        selector_str: &str,
    ) -> Result<Vec<Node>, anyhow::Error> {
        let base_el = self.find_element_by_html(base.html())?;

        // Handle direct child selector prefix
        let effective_selector = selector_str
            .trim()
            .strip_prefix('>')
            .map(|s| s.trim())
            .unwrap_or(selector_str);

        if effective_selector.is_empty() {
            return Ok(Vec::new());
        }

        let relative_selector = Selector::parse(effective_selector)
            .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", effective_selector, e))?;

        Ok(base_el
            .select(&relative_selector)
            .map(|el| node_from_element(el, &self.html))
            .collect())
    }

    pub fn select_one_relative(
        &self,
        base: &Node,
        selector_str: &str,
    ) -> Result<Option<Node>, anyhow::Error> {
        Ok(self.select_relative(base, selector_str)?.into_iter().next())
    }

    /// Find the next sibling element containing elements matching the selector
    /// Returns the sibling element itself, not the matched descendant
    pub fn select_next_sibling(
        &self,
        base: &Node,
        selector_str: &str,
    ) -> Result<Option<Node>, anyhow::Error> {
        let base_el = self.find_element_by_html(base.html())?;
        let selector = Selector::parse(selector_str)
            .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", selector_str, e))?;

        for sibling in base_el.next_siblings() {
            if let Some(sib_el) = ElementRef::wrap(sibling)
                && sib_el.select(&selector).next().is_some()
            {
                return Ok(Some(node_from_element(sib_el, &self.html)));
            }
        }

        Ok(None)
    }

    /// Select using a pre-parsed `Arc<Selector>` - optimized path
    pub fn select_one_with_selector(
        &self,
        selector: Arc<Selector>,
    ) -> Result<Option<Node>, anyhow::Error> {
        Ok(self
            .html
            .select(&selector)
            .next()
            .map(|el| node_from_element(el, &self.html)))
    }

    /// Select relative using a pre-parsed `Arc<Selector>` - optimized path
    pub fn select_one_relative_with_selector(
        &self,
        base: &Node,
        selector: Arc<Selector>,
    ) -> Result<Option<Node>, anyhow::Error> {
        let base_el = self.find_element_by_html(base.html())?;
        Ok(base_el
            .select(&selector)
            .next()
            .map(|el| node_from_element(el, &self.html)))
    }

    /// Find element by its HTML content
    fn find_element_by_html(&self, target_html: &str) -> Result<ElementRef<'_>, anyhow::Error> {
        // Iterate through tree to find matching element
        for node in self.html.tree.nodes() {
            if let Some(el) = ElementRef::wrap(node)
                && el.html() == target_html
            {
                return Ok(el);
            }
        }
        Err(anyhow::anyhow!("Element not found"))
    }
}

fn node_from_element(el: ElementRef, tree: &Html) -> Node {
    let attrs: Vec<(String, String)> = el
        .value()
        .attrs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    // Get text content normally
    let text_content = el.text().collect::<String>();

    // For void elements, check if next sibling is a text node (common in RSS/XML)
    let text = if text_content.is_empty() && is_void_element(el.value().name()) {
        // Look for text in next sibling (where RSS link text ends up)
        // The text node becomes a sibling after the void element
        let node_id = el.id();
        tree.tree
            .get(node_id)
            .and_then(|node_ref| node_ref.parent())
            .and_then(|parent| {
                let parent_ref = tree.tree.get(parent.id())?;
                let mut found_current = false;
                for child_ref in parent_ref.children() {
                    if found_current {
                        // This is the next sibling - check if it's a text node
                        if let Some(text) = child_ref.value().as_text() {
                            return Some(text.trim().to_string());
                        }
                        break;
                    }
                    if child_ref.id() == node_id {
                        found_current = true;
                    }
                }
                None::<String>
            })
            .unwrap_or_else(|| text_content.clone())
    } else {
        text_content
    };

    Node {
        html: el.html(),
        text,
        attributes: attrs,
    }
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
