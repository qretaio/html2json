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
        Ok(self.html.select(&selector).map(node_from_element).collect())
    }

    pub fn select_one(&self, selector_str: &str) -> Result<Option<Node>, anyhow::Error> {
        let selector = Selector::parse(selector_str)
            .map_err(|e| anyhow::anyhow!("Invalid selector '{}': {}", selector_str, e))?;
        Ok(self.html.select(&selector).next().map(node_from_element))
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
            .map(node_from_element)
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
                return Ok(Some(node_from_element(sib_el)));
            }
        }

        Ok(None)
    }

    /// Select using a pre-parsed `Arc<Selector>` - optimized path
    pub fn select_one_with_selector(
        &self,
        selector: Arc<Selector>,
    ) -> Result<Option<Node>, anyhow::Error> {
        Ok(self.html.select(&selector).next().map(node_from_element))
    }

    /// Select relative using a pre-parsed `Arc<Selector>` - optimized path
    pub fn select_one_relative_with_selector(
        &self,
        base: &Node,
        selector: Arc<Selector>,
    ) -> Result<Option<Node>, anyhow::Error> {
        let base_el = self.find_element_by_html(base.html())?;
        Ok(base_el.select(&selector).next().map(node_from_element))
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

fn node_from_element(el: ElementRef) -> Node {
    let attrs: Vec<(String, String)> = el
        .value()
        .attrs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    Node {
        html: el.html(),
        text: el.text().collect(),
        attributes: attrs,
    }
}
