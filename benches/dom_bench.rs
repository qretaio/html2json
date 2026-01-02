use criterion::{Criterion, criterion_group, criterion_main};
use html2json::{extract, spec::Spec};

fn bench_basic_extraction(c: &mut Criterion) {
    let html = include_str!("../examples/hn.html");
    let spec_json = r#"{"title": "title"}"#;
    let spec: Spec = serde_json::from_str(spec_json).unwrap();

    c.bench_function("basic_extraction", |b| {
        b.iter(|| extract(html, &spec).unwrap())
    });
}

fn bench_array_extraction(c: &mut Criterion) {
    let html = include_str!("../examples/hn.html");
    let spec_json = r#"{"items": [{"$": "tr.athing", "id": "$ | attr:id"}]}"#;
    let spec: Spec = serde_json::from_str(spec_json).unwrap();

    c.bench_function("array_extraction", |b| {
        b.iter(|| extract(html, &spec).unwrap())
    });
}

fn bench_nested_extraction(c: &mut Criterion) {
    let html = include_str!("../examples/hn.html");
    let spec_json =
        r#"{"items": [{"$": "tr.athing", "title": ".titleline a", "score": "+ .subtext .score"}]}"#;
    let spec: Spec = serde_json::from_str(spec_json).unwrap();

    c.bench_function("nested_extraction", |b| {
        b.iter(|| extract(html, &spec).unwrap())
    });
}

fn bench_full_hackernews(c: &mut Criterion) {
    let html = include_str!("../examples/hn.html");
    let spec_json = include_str!("../examples/hn.json");
    let spec: Spec = serde_json::from_str(spec_json).unwrap();

    c.bench_function("full_hackernews", |b| {
        b.iter(|| extract(html, &spec).unwrap())
    });
}

fn bench_dom_parse(c: &mut Criterion) {
    let html = include_str!("../examples/hn.html");

    c.bench_function("dom_parse", |b| {
        b.iter(|| html2json::Dom::parse(html).unwrap())
    });
}

fn bench_query_selector(c: &mut Criterion) {
    let html = include_str!("../examples/hn.html");
    let dom = html2json::Dom::parse(html).unwrap();

    c.bench_function("query_selector", |b| {
        b.iter(|| dom.query_selector("title").unwrap())
    });
}

fn bench_query_selector_all(c: &mut Criterion) {
    let html = include_str!("../examples/hn.html");
    let dom = html2json::Dom::parse(html).unwrap();

    c.bench_function("query_selector_all", |b| {
        b.iter(|| dom.query_selector_all("tr.athing").unwrap())
    });
}

fn bench_repeated_selector_parse(c: &mut Criterion) {
    let html = include_str!("../examples/hn.html");
    let dom = html2json::Dom::parse(html).unwrap();

    c.bench_function("repeated_selector_parse", |b| {
        b.iter(|| {
            // Simulate repeated queries with the same selector
            for _ in 0..10 {
                std::hint::black_box(dom.query_selector_all("tr.athing").unwrap());
            }
        })
    });
}

// Benchmark: Text extraction cost
fn bench_text_extraction(c: &mut Criterion) {
    let html = include_str!("../examples/hn.html");
    let dom = html2json::Dom::parse(html).unwrap();

    c.bench_function("text_extraction", |b| {
        b.iter(|| {
            let nodes = dom.query_selector_all("tr.athing").unwrap();
            // Extract text from all nodes
            for node in &nodes {
                std::hint::black_box(node.text());
            }
        })
    });
}

criterion_group!(
    dom_benches,
    bench_basic_extraction,
    bench_array_extraction,
    bench_nested_extraction,
    bench_full_hackernews,
    bench_dom_parse,
    bench_query_selector,
    bench_query_selector_all,
    bench_repeated_selector_parse,
    bench_text_extraction
);
criterion_main!(dom_benches);
