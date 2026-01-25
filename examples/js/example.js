// Example: Using @qretaio/html2json in a browser or Node.js environment

import { extract } from '@qretaio/html2json';

// Sample HTML - in a real scenario, this might come from fetch() or a file
const html = `
  <!DOCTYPE html>
  <html>
  <head>
    <title>My Blog</title>
  </head>
  <body>
    <article class="post">
      <h2>Introduction to WebAssembly</h2>
      <p class="author">Jane Developer</p>
      <p class="date">2024-01-15</p>
      <div class="content">
        <p>WebAssembly allows you to run high-performance code in the browser.</p>
      </div>
      <div class="tags">
        <span>wasm</span>
        <span>rust</span>
        <span>javascript</span>
      </div>
      <a href="/posts/intro-wasm" class="read-more">Read more</a>
    </article>
    <article class="post">
      <h2>Advanced CSS Selectors</h2>
      <p class="author">John Coder</p>
      <div class="tags">
        <span>css</span>
        <span>html</span>
      </div>
      <a href="/posts/css-selectors" class="read-more">Read more</a>
    </article>
  </body>
  </html>
`;

// Define the extraction spec
const spec = JSON.stringify({
  // Extract page title
  pageTitle: "title",

  // Extract all articles as an array
  articles: [{
    "$": ".post",
    title: "h2",
    author: ".author",
    // Use the optional operator - won't be included if missing
    "date?": ".date",
    // Extract link attribute
    url: ".read-more | attr:href",
    // Extract tags as nested array
    tags: [{
      "$": ".tags span",
      name: "$"
    }]
  }]
});

// Perform the extraction
try {
  const resultJson = extract(html, spec);
  const result = JSON.parse(resultJson);

  console.log('Extracted data:');
  console.log(JSON.stringify(result, null, 2));

  // Example output:
  // {
  //   "pageTitle": "My Blog",
  //   "articles": [
  //     {
  //       "title": "Introduction to WebAssembly",
  //       "author": "Jane Developer",
  //       "date": "2024-01-15",
  //       "url": "/posts/intro-wasm",
  //       "tags": [
  //         { "name": "wasm" },
  //         { "name": "rust" },
  //         { "name": "javascript" }
  //       ]
  //     },
  //     {
  //       "title": "Advanced CSS Selectors",
  //       "author": "John Coder",
  //       "url": "/posts/css-selectors",
  //       "tags": [
  //         { "name": "css" },
  //         { "name": "html" }
  //       ]
  //     }
  //   ]
  // }
} catch (error) {
  console.error('Extraction failed:', error);
}

// Example: Using fallback operators
const html2 = `
  <div>
    <span class="price-regular">$25.00</span>
  </div>
`;

const spec2 = JSON.stringify({
  // Try multiple selectors, use first match
  price: ".price-sale || .price-regular || .price"
});

const result2 = JSON.parse(extract(html2, spec2));
console.log('Price:', result2.price); // "Price: $25.00"

// Example: Using pipes for transformations
const html3 = `
  <div>
    <span class="item">  Item Name  </span>
    <span class="cost">$19.99</span>
  </div>
`;

const spec3 = JSON.stringify({
  // Trim whitespace and convert to lowercase
  slug: ".item | trim | lower | regex:\\s+-",
  // Extract price using regex and parse as number
  price: ".cost | regex:\\$(\\d+\\.\\d+) | parseAs:float"
});

const result3 = JSON.parse(extract(html3, spec3));
console.log('Slug:', result3.slug);  // "item-name"
console.log('Price:', result3.price); // 19.99
