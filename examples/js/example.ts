// TypeScript Example: Using @qretaio/html2json with full type safety

import { extract } from "@qretaio/html2json";

// Define your result types for type safety
interface Tag {
  name: string;
}

interface Article {
  title: string;
  author: string;
  date?: string; // Optional field
  url: string;
  tags: Tag[];
}

interface ExtractedData {
  pageTitle: string;
  articles: Article[];
}

// Sample HTML
const html: string = `
  <html>
  <head><title>Tech Blog</title></head>
  <body>
    <article class="post">
      <h2>Understanding TypeScript</h2>
      <p class="author">Jane Developer</p>
      <p class="date">2024-01-15</p>
      <div class="tags">
        <span>typescript</span>
        <span>javascript</span>
      </div>
      <a href="/posts/typescript">Read more</a>
    </article>
  </body>
  </html>
`;

// Define the extraction spec with TypeScript
const spec = {
  pageTitle: "title",
  articles: [
    {
      $: ".post",
      title: "h2",
      author: ".author",
      "date?": ".date", // Optional - won't be in result if missing
      url: "a | attr:href",
      tags: [
        {
          $: ".tags span",
          name: "$",
        },
      ],
    },
  ],
};

// Extract and type the result
try {
  const resultJson: string = await extract(html, spec);
  const result: ExtractedData = resultJson as unknown as ExtractedData;

  console.log(`Page: ${result.pageTitle}`);
  console.log(`Found ${result.articles.length} article(s)`);

  result.articles.forEach((article: Article) => {
    console.log(`- ${article.title} by ${article.author}`);
    if (article.date) {
      console.log(`  Published: ${article.date}`);
    }
    console.log(`  Tags: ${article.tags.map((t) => t.name).join(", ")}`);
  });
} catch (error) {
  console.error("Extraction error:", error);
}
