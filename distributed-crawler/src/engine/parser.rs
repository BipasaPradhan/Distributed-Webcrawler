use crate::webstats::BasicStats;

use scraper::{Html, Node, Selector};
use std::collections::HashSet;
use url::Url;

// attributes that can have urls -> most are caught with these
const URL_ATTRS: &[&str] = &["href", "src"];

pub fn parse_html(body: &str, page_url: &Url, base: &str) -> BasicStats {
    let doc = Html::parse_document(body);
    BasicStats {
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

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://site.com/docs/";

    fn page() -> Url {
        Url::parse("https://site.com/docs/guide/a.html").unwrap()
    }

    fn links_of(html: &str) -> Vec<String> {
        parse_html(html, &page(), BASE)
            .links
            .iter()
            .map(|u| u.to_string())
            .collect()
    }

    fn full(path: &str) -> String {
        format!("https://site.com/docs/{path}")
    }

    // ---------- words ----------

    #[test]
    fn words_basic() {
        assert_eq!(count_words_in("Hello World"), 2);
        assert_eq!(count_words_in("Ant ant ANT"), 3);
    }

    #[test]
    fn words_must_start_with_letter() {
        assert_eq!(count_words_in("42abc 2024 abc"), 1);
        // '_' is a separator under our rule, so "_x" yields the word "x"
        assert_eq!(count_words_in("_x"), 1);
    }

    #[test]
    fn words_punctuation() {
        assert_eq!(count_words_in("(hello), world! don't well-known"), 5);
    }

    #[test]
    fn words_skip_comments_script_style() {
        let html = r#"<html><head>
            <style>body { color: red }</style>
            <script>var hidden = "words";</script>
          </head><body>
            <!-- secret comment words -->
            <p class="these are attrs">one <b>two</b> three</p>
          </body></html>"#;
        assert_eq!(parse_html(html, &page(), BASE).word_count, 3);
    }

    #[test]
    fn words_entities_split() {
        assert_eq!(parse_html("<p>a&nbsp;b</p>", &page(), BASE).word_count, 2);
    }

    // ---------- links: resolution & filtering ----------

    #[test]
    fn links_relative_resolution() {
        let html = r#"<a href="b.html">x</a> <a href="../top.html">y</a>
                      <a href="/docs/abs.html">z</a>"#;
        assert_eq!(
            links_of(html),
            vec![full("guide/b.html"), full("top.html"), full("abs.html")]
        );
    }

    #[test]
    fn links_fragment_dropped_and_deduped() {
        let html = r##"<a href="b.html#intro">1</a> <a href="b.html#usage">2</a>
                       <a href="#top">self</a>"##;
        assert_eq!(
            links_of(html),
            vec![full("guide/b.html"), full("guide/a.html")]
        );
    }

    #[test]
    fn links_outside_base_rejected() {
        let html = r#"<a href="https://other.com/docs/">x</a>
                      <a href="/blog/post.html">y</a>
                      <a href="mailto:me@site.com">z</a>
                      <a href="javascript:void(0)">w</a>
                      <link rel="preconnect" href="https://fonts.example.com">"#;
        assert!(links_of(html).is_empty());
    }

    #[test]
    fn links_base_tag_respected() {
        let html = r#"<head><base href="/docs/other/"></head><a href="x.html">x</a>"#;
        // the <base> itself must NOT appear as a link
        assert_eq!(links_of(html), vec![full("other/x.html")]);
    }

    // ---------- links: sources ----------

    #[test]
    fn links_href_and_src_on_any_tag() {
        let html = r#"
            <link rel="stylesheet" href="s.css">
            <map><area href="region.html"></map>
            <img src="pic.png"> <script src="app.js"></script>
            <iframe src="frame.html"></iframe>
            <video><source src="clip.mp4"></video>"#;
        let mut got = links_of(html);
        got.sort();
        let mut want: Vec<String> = [
            "s.css",
            "region.html",
            "pic.png",
            "app.js",
            "frame.html",
            "clip.mp4",
        ]
        .iter()
        .map(|p| full(&format!("guide/{p}")))
        .collect();
        want.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn same_url_in_href_and_src_counted_once() {
        let html = r#"<a href="pic.png">big</a> <img src="pic.png">"#;
        assert_eq!(links_of(html), vec![full("guide/pic.png")]);
    }

    #[test]
    fn non_url_attributes_are_not_links() {
        // Only href/src are read, so none of these become fake URLs,
        // and the attributes we chose not to support are ignored too.
        let html = r#"<div class="button" id="intro" title="index" data-tab="api">t</div>
                      <img srcset="big.png 2x"> <video poster="p.jpg"></video>"#;
        assert!(links_of(html).is_empty());
    }
}
