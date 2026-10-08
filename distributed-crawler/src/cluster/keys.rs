pub const QUEUE: &str = "crawl:queue"; // work queue (frontier)
pub const URL_JOB: &str = "crawl:url_job"; // maps url -> unique job id
pub const NEXT_ID: &str = "crawl:next_id"; // unique job ids 

/// Fields of job:<id>
pub const URL: &str = "url"; // start & base url
pub const STATE: &str = "state";
pub const RUNNING: &str = "running";
pub const DONE: &str = "done";
pub const SUBMITTED_AT: &str = "submitted_at";
pub const FINISHED_AT: &str = "finished_at";

/// Fields of job:<id>:stats
pub const FILES: &str = "files";
pub const WORDS: &str = "words";
pub const CRAWLED: &str = "crawled";
pub const IN_FLIGHT: &str = "in_flight";

/// Urls that need to be processed
pub const PENDING: &str = "pending";

pub fn job(id: &str) -> String {
    format!("job:{id}")
}
pub fn seen(id: &str) -> String {
    format!("job:{id}:seen")
}
pub fn stats(id: &str) -> String {
    format!("job:{id}:stats")
}
pub fn exts(id: &str) -> String {
    format!("job:{id}:exts")
}

/// Combine job id + url for tracking
pub fn queue_item(job: &str, url: &str) -> String {
    format!("{job}\t{url}")
}

pub fn parse_queue_item(item: &str) -> Option<(&str, &str)> {
    item.split_once('\t')
}
