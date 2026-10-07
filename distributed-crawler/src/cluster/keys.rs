// main work queue (frontier)
pub const QUEUE: &str = "crawl:queue";
pub const JOB_IDS: &str = "crawl:job_ids";
pub const NEXT_ID: &str = "crawl:next_id";

// fields of job:<id>
pub const URL: &str = "url";
pub const STATE: &str = "state";
pub const RUNNING: &str = "running";
pub const DONE: &str = "done";

// fields of job:<id>:stats
pub const FILES: &str = "files";
pub const WORDS: &str = "words";
pub const CRAWLED: &str = "crawled";
pub const IN_FLIGHT: &str = "in_flight";

// urls that need to be processed
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

// combine job id + url for tracking
pub fn queue_item(job: &str, url: &str) -> String {
    format!("{job}\t{url}")
}

pub fn parse_queue_item(item: &str) -> Option<(&str, &str)> {
    item.split_once('\t')
}
