use anyhow::Result;
use clap::Parser;
use html2json::Spec;
use similar::{ChangeTag, TextDiff};
use std::io::Read;

// ANSI color codes
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

/// html2json - Extract JSON from HTML using CSS selectors
#[derive(Parser, Debug)]
#[command(name = "html2json")]
#[command(author = "html2json")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Args {
    /// Input: path to HTML file (reads from stdin if not provided)
    #[arg(value_name = "FILE")]
    input: Option<String>,

    /// Path to JSON extractor spec file
    #[arg(short, long, value_name = "SPEC")]
    spec: String,

    /// Check output matches expected JSON file (shows diff if different)
    #[arg(short, long, value_name = "FILE")]
    check: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let html = read_html(args.input.as_deref())?;
    let spec_value = load_spec(&args.spec)?;
    let spec = Spec::from_json(&spec_value)?;
    let dom = html2json::Dom::parse(&html)?;
    let result = dom.extract(&spec)?;

    if let Some(check_path) = args.check {
        // Compare against expected output
        let expected_value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&check_path)?)?;
        let actual_json = serde_json::to_string_pretty(&result)?;
        let expected_json = serde_json::to_string_pretty(&expected_value)?;

        if result == expected_value {
            eprintln!("✓ Output matches {}", check_path);
            std::process::exit(0);
        } else {
            eprintln!("✗ Output differs from {}\n", check_path);
            print_diff(&expected_json, &actual_json);
            std::process::exit(1);
        }
    } else {
        // Print output to stdout
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}

fn print_diff(expected: &str, actual: &str) {
    let diff = TextDiff::from_lines(expected, actual);

    for op in diff.ops().iter().take(50) {
        for change in diff.iter_changes(op) {
            let sign = match change.tag() {
                ChangeTag::Delete => format!("{}-{}", RED, BOLD),
                ChangeTag::Insert => format!("{}+{}", GREEN, BOLD),
                ChangeTag::Equal => continue,
            };
            print!("{}{} {}", sign, RESET, change.value());
        }
    }

    // Show truncation message if diff was too long
    if diff.ops().len() > 50 {
        eprintln!("... (diff truncated, showing first 50 changes)");
    }
}

// Maximum sizes for security
const MAX_HTML_SIZE: usize = 100_000_000; // 100MB
const MAX_SPEC_SIZE: usize = 1_048_576; // 1MB

/// Read HTML from a file path or stdin
fn read_html(path: Option<&str>) -> Result<String> {
    let content = match path {
        Some(file_path) => {
            std::fs::read_to_string(file_path)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", file_path, e))?
        }
        None => {
            // Read from stdin
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| anyhow::anyhow!("Failed to read from stdin: {}", e))?;
            buffer
        }
    };

    if content.len() > MAX_HTML_SIZE {
        return Err(anyhow::anyhow!(
            "HTML input exceeds maximum size of {} bytes",
            MAX_HTML_SIZE
        ));
    }

    Ok(content)
}

/// Load spec from a JSON file
fn load_spec(path: &str) -> Result<serde_json::Value> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read spec file '{}': {}", path, e))?;

    if content.len() > MAX_SPEC_SIZE {
        return Err(anyhow::anyhow!(
            "Spec file exceeds maximum size of {} bytes",
            MAX_SPEC_SIZE
        ));
    }

    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse spec JSON: {}", e))?;

    Ok(value)
}
