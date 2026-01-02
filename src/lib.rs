//! html2json - HTML to JSON extractor using html5ever
//!
//! A Rust port of cheerio-json-mapper using html5ever for HTML parsing.
//!
//! # Overview
//!
//! This library extracts structured JSON data from HTML using CSS selectors
//! defined in a JSON spec format.
//!
//! # Basic Example
//!
//! ```no_run
//! use html2json::{extract, Spec};
//!
//! let html = r#"<html><body><h1>Hello</h1><p class="desc">World</p></body></html>"#;
//! let spec_json = r#"{"title": "h1", "description": "p.desc"}"#;
//! let spec: Spec = serde_json::from_str(spec_json)?;
//! let result = extract(html, &spec)?;
//! assert_eq!(result["title"], "Hello");
//! assert_eq!(result["description"], "World");
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod dom;
pub mod extractor;
pub mod pipe;
pub mod spec;

pub use extractor::Extractor;
pub use spec::Spec;

use anyhow::Result;

/// Extract JSON from HTML using a spec
///
/// # Arguments
///
/// * `html` - The HTML source to parse
/// * `spec` - The extraction specification
///
/// # Example
///
/// ```
/// use html2json::{extract, Spec};
///
/// let html = r#"<div class="item"><span>Price: $25.00</span></div>"#;
/// let spec_json = r#"{"price": ".item span | regex:\\$(\\d+\\.\\d+)"}"#;
/// let spec: Spec = serde_json::from_str(spec_json)?;
/// let result = extract(html, &spec)?;
/// assert_eq!(result["price"], "25.00");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn extract(html: &str, spec: &Spec) -> Result<serde_json::Value> {
    let extractor = Extractor::new(html)?;
    extractor.extract(spec)
}

#[cfg(test)]
mod tests {
    use crate::extract;
    use crate::spec::Spec;

    const HTML: &str = include_str!("../tests/fixtures/readme_tests.html");

    #[test]
    fn basic_text_extraction() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-01",
                "title": "h1"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["title"], "Hello World");
    }

    #[test]
    fn attribute_extraction() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-02",
                "link": "a | attr:href"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["link"], "https://example.com");
    }

    // TEST-3: Scoping with $
    #[test]
    fn scoping_with_dollar() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-03 article",
                "headline": "> h1",
                "content": "> .content"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["headline"], "Title");
        assert_eq!(result["content"], "Body text");
    }

    #[test]
    fn nested_scoping() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-04 article",
                "title": "> h1",
                "author": {
                    "$": "> .author",
                    "name": "span.name",
                    "email": "a | attr:href"
                }
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["title"], "My Post");
        assert_eq!(result["author"]["name"], "Jane");
        assert_eq!(result["author"]["email"], "mailto:jane@example.com");
    }

    #[test]
    fn collection_extraction() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-05",
                "items": [{
                    "$": "ul li",
                    "text": "$"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let arr = result["items"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["text"], "First");
        assert_eq!(arr[1]["text"], "Second");
        assert_eq!(arr[2]["text"], "Third");
    }

    #[test]
    fn collection_with_nested_properties() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-06",
                "products": [{
                    "$": ".product",
                    "name": "> h2",
                    "price": "> .price"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let arr = result["products"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "Widget A");
        assert_eq!(arr[0]["price"], "$10");
        assert_eq!(arr[1]["name"], "Widget B");
        assert_eq!(arr[1]["price"], "$20");
    }

    #[test]
    fn literal_values() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-07",
                "extracted": "h1",
                "source": "'web-scrape'",
                "version": 1.5,
                "active": true
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["extracted"], "Test");
        assert_eq!(result["source"], "web-scrape");
        assert_eq!(result["version"], 1.5);
        assert_eq!(result["active"], true);
    }

    #[test]
    fn trim_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-08",
                "clean": "span | trim"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["clean"], "messy text");
    }

    // TEST-9: Lowercase Pipe
    #[test]
    fn lowercase_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-09",
                "lower": "h1 | lower"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["lower"], "hello world");
    }

    #[test]
    fn uppercase_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-10",
                "upper": "h1 | upper"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["upper"], "HELLO");
    }

    // TEST-11: Substring Pipe
    #[test]
    fn substring_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-11",
                "partial": "h1 | substr:0:4"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["partial"], "Hell");
    }

    #[test]
    fn parse_as_number_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-12",
                "price": ".price | parseAs:number"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["price"], 99.99);
    }

    #[test]
    fn parse_as_int_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-13",
                "count": ".count | parseAs:int"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["count"], 42);
    }

    #[test]
    fn chained_pipes() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-14",
                "email": "a | attr:href | substr:7"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["email"], "test@example.com");
    }

    // TEST-15: Regex Pipe with Capture Group
    #[test]
    fn regex_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-15",
                "amount": ".price | regex:\\$(\\d+\\.\\d+)"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["amount"], "19.99");
    }

    // TEST-16: No Match Returns Null
    #[test]
    fn no_match_returns_null() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-16",
                "missing": ".nonexistent",
                "present": "h1"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert!(result["missing"].is_null());
        assert_eq!(result["present"], "Exists");
    }

    // TEST-17: Empty Collection Returns Empty Array
    #[test]
    fn empty_collection_returns_empty_array() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-17",
                "items": [{
                    "$": ".nonexistent",
                    "value": "$"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let arr = result["items"].as_array().unwrap();
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn multiple_attributes() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-18",
                "href": "a | attr:href",
                "id": "a | attr:id",
                "class": "a | attr:class"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["href"], "/link");
        assert_eq!(result["id"], "link1");
        assert_eq!(result["class"], "btn");
    }

    #[test]
    fn complex_nested_structure() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-19 .container",
                "title": "> h1",
                "items": {
                    "$": "> ul",
                    "list": [{
                        "$": "> li",
                        "text": "$"
                    }]
                }
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["title"], "Shopping List");
        let list = result["items"]["list"].as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["text"], "Apples");
        assert_eq!(list[1]["text"], "Bananas");
    }

    #[test]
    fn self_selector_in_collection() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-20",
                "paragraphs": [{
                    "$": "p",
                    "content": "$"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let arr = result["paragraphs"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["content"], "First paragraph");
        assert_eq!(arr[1]["content"], "Second paragraph");
    }

    #[test]
    #[expect(clippy::approx_constant)]
    fn parse_as_float_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-21",
                "pi": ".float | parseAs:float"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        // Value from HTML fixture, not actually PI constant
        assert_eq!(result["pi"], 3.14159);
    }

    #[test]
    fn next_sibling_with_last_child() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#test-22",
                "items": [{
                    "$": "tr.athing",
                    "title": ".titlelink",
                    "points": "+ .subtext .score",
                    "user": "+ .subtext .hnuser",
                    "comments": "+ .subtext > a:last-child"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let items = result["items"].as_array().unwrap();

        assert_eq!(items.len(), 2);

        // First item
        assert_eq!(items[0]["title"], "Item 1 Title");
        assert_eq!(items[0]["points"], "100 points");
        assert_eq!(items[0]["user"], "user1");
        assert_eq!(items[0]["comments"], "100 comments");

        // Second item
        assert_eq!(items[1]["title"], "Item 2 Title");
        assert_eq!(items[1]["points"], "50 points");
        assert_eq!(items[1]["user"], "user2");
        assert_eq!(items[1]["comments"], "50 comments");
    }
}
