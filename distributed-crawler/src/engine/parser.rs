use crate::webstats::ParsedPage;

use scraper::{Html, Node, Selector};
use std::collections::HashSet;
use url::Url;

// attributes that can have urls -> most are caught with these
const URL_ATTRS: &[&str] = &["href", "src"];

pub fn parse_html(body: &str, page_url: &Url, base: &str) -> ParsedPage {
    let doc = Html::parse_document(body);
    ParsedPage {
        links: extract_links(&doc, page_url, base),
        word_count: count_words(&doc),
    }
}

fn sel(css: &str) -> Selector {
    Selector::parse(css).expect("hardcoded selector must be valid")
}

// Function to extract links from file
fn extract_links(doc: &Html, page_url: &Url, base: &str) -> Vec<Url> {
    // if there is a base tag
    let from = doc
        .select(&sel("base[href]"))
        .next()
        .and_then(|el| el.value().attr("href"))
        .and_then(|h| page_url.join(h).ok())
        .unwrap_or_else(|| page_url.clone());

    let mut seen = HashSet::new(); // urls seen in this page 
    let mut output = Vec::new();

    for attr in URL_ATTRS {
        for el in doc.select(&sel(&format!("[{attr}]"))) {
            if el.value().name() == "base" {
                continue;
            }

            let Some(value) = el.value().attr(attr) else {
                continue;
            };

            if let Some(url) = normalize_link(&from, value, base) {
                if seen.insert(url.clone()) {
                    output.push(url);
                }
            }
        }
    }
    output
}

fn normalize_link(from: &Url, href: &str, base: &str) -> Option<Url> {
    let href = href.trim();

    if href.is_empty() {
        return None;
    }

    let mut url = from.join(href).ok()?;

    // filtering out non http - cannot send http request
    if url.scheme() != "http" && url.scheme() != "https" {
        return None;
    }

    url.set_fragment(None); // removes fragment (e.g. #)
    url.as_str().starts_with(base).then_some(url) // only accept if it starts with base
}

fn count_words(doc: &Html) -> u64 {
    let mut count = 0;
    for node in doc.root_element().descendants() {
        // only text nodes -> tags, attributes, comments are skipped automatically
        let Node::Text(text) = node.value() else {
            continue;
        };

        // skip code/CSS inside <script>, <style>, <noscript>
        let parent_tag = node
            .parent()
            .and_then(|p| p.value().as_element())
            .map(|e| e.name());
        if matches!(parent_tag, Some("script" | "style" | "noscript")) {
            continue;
        }
        count += count_words_in(text);
    }
    count
}

// Lowercased, begins with a-z, split at anything that is not alphanumeric or '
pub fn count_words_in(text: &str) -> u64 {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| w.starts_with(|c: char| c.is_ascii_lowercase()))
        .count() as u64
}
