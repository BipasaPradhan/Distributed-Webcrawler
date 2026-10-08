use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use url::Url;

use crate::cluster::keys;

#[derive(Debug)]
pub struct ParsedPage {
    pub links: Vec<Url>,
    pub word_count: u64,
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

#[derive(Debug, Default, Clone, PartialEq)]
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

impl WebStats {
    pub fn record(&mut self, page: &PageResult) {
        if !page.exists {
            return;
        }

        self.num_files += 1;
        *self.ext_counts.entry(page.extension.clone()).or_insert(0) += 1;
        self.num_exts = self.ext_counts.len();
        self.total_word_count += page.word_count;
    }
}

impl fmt::Display for WebStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "files: {} | extensions: {} | words: {}",
            self.num_files, self.num_exts, self.total_word_count
        )?;
        // sort by num then alphabetically
        let mut exts: Vec<_> = self.ext_counts.iter().collect();
        exts.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let line: Vec<String> = exts.iter().map(|(e, n)| format!("{e} {n}")).collect();
        write!(f, "{}", line.join(" | "))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JobStatus {
    pub url: String,
    pub done: bool,
    pub crawled: i64,
    pub in_flight: i64,
    pub pending: i64,
    pub time_elapsed: Duration,
}

impl JobStatus {
    // Get how many urls are in the queue
    pub fn frontier(&self) -> i64 {
        (self.pending - self.in_flight).max(0)
    }

    pub fn line(&self) -> String {
        format!(
            "crawled {:<6}| frontier {:<6}| in flight {:<3} {}",
            self.crawled,
            self.frontier(),
            self.in_flight,
            if self.done { keys::DONE } else { keys::RUNNING }
        )
    }
}
