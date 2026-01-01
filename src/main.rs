use anyhow::Result;
use clap::Parser;
use html2json::Spec;
use html2json::parser;

/// html2json - Extract JSON from HTML using CSS selectors
#[derive(Parser, Debug)]
#[command(name = "html2json")]
#[command(author = "html2json")]
#[command(version = "0.1.0")]
struct Args {
    /// Input: URL starting with http:// or https://, or path to HTML file
    input: String,

    /// Path to JSON extractor spec file
    spec: String,

    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let html = parser::fetch_html(&args.input).await?;
    let spec_value = parser::load_spec(&args.spec)?;
    let spec = Spec::from_json(&spec_value)?;
    let extractor = html2json::Extractor::new(&html)?;
    let result = extractor.extract(&spec)?;

    if args.verbose {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}
