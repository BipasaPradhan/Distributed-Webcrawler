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

#[derive(Debug)]
pub struct BasicStats {
    pub links: Vec<Url>,
    pub word_count: u64,
}
