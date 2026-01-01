//! Comprehensive test on a large HTML document (several KBs)
//!
//! This test simulates scraping a real e-commerce product listing page
//! with multiple products, nested structures, and various selectors.

use html2json::{Spec, extract};

const LARGE_HTML: &str = include_str!("fixtures/ecommerce.html");

#[test]
fn comprehensive_extraction() {
    let spec_json = r#"{
        "$": "body",
        "siteName": ".logo",
        "cart": {
            "$": ".cart-info",
            "itemCount": ".cart-count",
            "total": ".cart-total"
        },
        "hero": {
            "$": ".hero",
            "title": "h1",
            "subtitle": ".hero-subtitle",
            "stats": [{ "$": ".stat", "text": "$" }]
        },
        "categories": [{ "$": ".category-list a", "name": "$", "url": "$ | attr:href", "id": "$ | attr:data-cat-id | parseAs:int" }],
        "products": [{
            "$": ".product-card",
            "id": "$ | attr:data-product-id | parseAs:int",
            "inStock": "$ | attr:data-in-stock",
            "title": "> .product-info .product-title",
            "description": "> .product-info .product-description",
            "badges": [{ "$": "> .product-image .badge", "text": "$" }],
            "specs": [{ "$": "> .product-info .spec", "name": "$" }],
            "rating": {
                "$": "> .product-info .product-rating",
                "stars": ".stars",
                "reviewCount": ".review-count | regex:\\((\\d+)\\s*reviews\\)"
            },
            "pricing": {
                "$": "> .product-info .product-pricing",
                "current": ".price-current | regex:\\$(\\d+\\.\\d+)",
                "original": ".price-original | regex:\\$(\\d+\\.\\d+)",
                "discount": ".discount"
            }
        }],
        "newsletter": {
            "$": ".newsletter",
            "title": "h2",
            "description": "p"
        },
        "features": [{ "$": ".feature", "title": "h3", "text": "p" }],
        "footer": {
            "$": ".site-footer",
            "about": "> .footer-section:nth-of-type(1) p",
            "email": "> .footer-section:nth-of-type(2) li",
            "socialLinks": [{ "$": "> .footer-section:nth-of-type(3) .social", "platform": "$" }]
        }
    }"#;

    let spec: Spec = serde_json::from_str(spec_json).unwrap();
    let result = extract(LARGE_HTML, &spec).unwrap();

    // Verify top-level page info
    assert_eq!(result["siteName"], "TechStore");

    // Verify cart info
    eprintln!(
        "cart result: {}",
        serde_json::to_string_pretty(&result["cart"]).unwrap()
    );
    assert_eq!(result["cart"]["itemCount"], "3");
    assert_eq!(result["cart"]["total"], "$1,249.97");

    // Verify hero section
    assert_eq!(result["hero"]["title"], "Summer Sale - Up to 50% Off!");
    assert_eq!(
        result["hero"]["subtitle"],
        "Limited time offer on premium electronics"
    );
    let hero_stats = result["hero"]["stats"].as_array().unwrap();
    assert_eq!(hero_stats.len(), 3);
    assert_eq!(hero_stats[0]["text"], "500+ Products");

    // Verify categories
    let categories = result["categories"].as_array().unwrap();
    assert_eq!(categories.len(), 4);
    assert_eq!(categories[0]["name"], "Laptops");
    assert_eq!(categories[0]["url"], "/category/laptops");
    assert_eq!(categories[0]["id"], 1);

    // Verify products
    let products = result["products"].as_array().unwrap();
    assert_eq!(products.len(), 6);

    // First product details
    assert_eq!(products[0]["id"], 101);
    assert_eq!(products[0]["inStock"], "true");
    assert_eq!(products[0]["title"], "TechPro Laptop 15\"");
    assert!(
        products[0]["description"]
            .as_str()
            .unwrap()
            .contains("16GB RAM")
    );

    // First product badges
    let badges = products[0]["badges"].as_array().unwrap();
    assert_eq!(badges.len(), 1);
    assert_eq!(badges[0]["text"], "SALE");

    // First product specs
    let specs = products[0]["specs"].as_array().unwrap();
    assert_eq!(specs.len(), 3);
    assert_eq!(specs[0]["name"], "Intel Core i7");

    // First product rating
    assert_eq!(products[0]["rating"]["stars"], "★★★★☆");
    assert_eq!(products[0]["rating"]["reviewCount"], "245");

    // First product pricing
    assert_eq!(products[0]["pricing"]["current"], "899.99");
    // Note: .price-original has value "$1,199.99" with comma, which doesn't match regex \$(\d+\.\d+)
    assert_eq!(products[0]["pricing"]["original"], serde_json::Value::Null);
    assert_eq!(products[0]["pricing"]["discount"], "25% OFF");

    // Third product (out of stock)
    assert_eq!(products[2]["id"], 103);
    assert_eq!(products[2]["inStock"], "false");
    let out_badges = products[2]["badges"].as_array().unwrap();
    assert_eq!(out_badges[0]["text"], "OUT OF STOCK");

    // Newsletter
    assert_eq!(result["newsletter"]["title"], "Subscribe to Our Newsletter");
    assert_eq!(
        result["newsletter"]["description"],
        "Get exclusive deals and updates delivered to your inbox"
    );

    // Features
    let features = result["features"].as_array().unwrap();
    assert_eq!(features.len(), 4);
    assert_eq!(features[0]["title"], "Free Shipping");
    assert_eq!(features[0]["text"], "On orders over $50");

    // Footer
    assert_eq!(
        result["footer"]["about"],
        "Your trusted source for premium electronics since 2010."
    );

    // Total extracted fields count (comprehensive coverage)
    assert_eq!(result.as_object().unwrap().len(), 8);
}

#[test]
fn nested_array_extraction_with_pipes() {
    let spec_json = r#"{
        "products": [{
            "$": ".product-card",
            "id": "attr:data-product-id | parseAs:int",
            "title": "> .product-info .product-title",
            "price": "> .product-info .price-current | regex:\\$(\\d+\\.\\d+) | parseAs:number",
            "reviews": "> .product-info .review-count | regex:(\\d+)"
        }]
    }"#;

    let spec: Spec = serde_json::from_str(spec_json).unwrap();
    let result = extract(LARGE_HTML, &spec).unwrap();

    let products = result["products"].as_array().unwrap();
    assert_eq!(products.len(), 6);

    // Verify parsing and pipes work correctly
    assert_eq!(products[0]["id"], 101);
    assert_eq!(products[0]["title"], "TechPro Laptop 15\"");
    assert_eq!(products[0]["price"], 899.99);
    assert_eq!(products[0]["reviews"], "245");

    assert_eq!(products[1]["id"], 102);
    assert_eq!(products[1]["reviews"], "512");
}

#[test]
fn attribute_extraction_with_selectors() {
    let spec_json = r#"{
        "logo": ".logo",
        "logoExists": ".logo | attr:class",
        "firstProductImage": ".product-card:nth-of-type(1) .product-image img | attr:src",
        "firstProductAlt": ".product-card:nth-of-type(1) .product-image img | attr:alt"
    }"#;

    let spec: Spec = serde_json::from_str(spec_json).unwrap();
    let result = extract(LARGE_HTML, &spec).unwrap();

    assert_eq!(result["logo"], "TechStore");
    assert_eq!(result["logoExists"], "logo");
    assert_eq!(result["firstProductImage"], "/images/laptop-pro.jpg");
    assert_eq!(result["firstProductAlt"], "TechPro Laptop");
}
