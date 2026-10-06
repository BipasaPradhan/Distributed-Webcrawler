use std::collections::HashMap;
use url::Url;

#[derive(Debug)]
pub struct WebStats {
    // the total number of (unique) files found
    pub num_files: usize,
    // the total number of (unique) file extensions (.jpg is different from .jpeg)
    pub num_exts: usize,
    // the total number of files for each extension
    pub ext_counts: HashMap<String, usize>,
    // the total number of words in all HTML files combined, excluding
    // all HTML tags, attributes, and HTML comments
    pub total_word_count: u64,
}

// Everything the crawler needs to know about one URL
#[derive(Debug)]
pub struct PageResult {
    // 200 response
    pub exists: bool,
    pub extension: String,
    pub is_html: bool,
    pub word_count: u64,
    // links to crawl next
    pub links: Vec<Url>,
}

impl PageResult {
    pub fn missing(extension: String) -> Self {
        Self {
            exists: false,
            extension,
            is_html: false,
            word_count: 0,
            links: Vec::new(),
        }
    }

    pub fn file(extension: String, is_html: bool) -> Self {
        Self {
            exists: true,
            extension,
            is_html,
            word_count: 0,
            links: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct BasicStats {
    pub links: Vec<Url>,
    pub word_count: u64,
}
