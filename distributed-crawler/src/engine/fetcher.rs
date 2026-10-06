use std::time::Duration;

use reqwest::{Client, StatusCode, header::CONTENT_TYPE, redirect::Policy};
use url::Url;

use crate::engine::parser::parse_html;
use crate::webstats::PageResult;

const MAX_ATTEMPTS: u32 = 3; // 1 normal + 2 retries
const WAIT_TIME: Duration = Duration::from_millis(250);

// 1 attempt
enum Attempt {
    Done(PageResult),
    Retry(PageResult),
}

pub fn build_client() -> reqwest::Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .redirect(Policy::limited(10))
        .user_agent("goms-mini1-crawler/0.1")
        .build()
}

async fn try_fetch(client: &Client, url: &Url, base: &str) -> Attempt {
    let extension = get_extension(url);

    // send req
    let resp = match client.get(url.clone()).send().await {
        Ok(resp) => resp,

        // invalid req
        Err(e) if e.is_redirect() || e.is_builder() => {
            return Attempt::Done(PageResult::missing(extension));
        }

        // retries
        Err(_) => return Attempt::Retry(PageResult::missing(extension)),
    };

    let status = resp.status();
    if !status.is_success() {
        let result = PageResult::missing(extension);
        return if is_retryable_status(status) {
            Attempt::Retry(result)
        } else {
            Attempt::Done(result)
        };
    }

    // check if html
    let is_html = match resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        Some(ct) => is_html_content_type(ct),
        None => extension == "html",
    };

    // not html -> dont parse
    if !is_html {
        return Attempt::Done(PageResult::file(extension, false));
    }

    let final_url = resp.url().clone();
    if !final_url.as_str().starts_with(base) {
        return Attempt::Done(PageResult::file(extension, true));
    }

    let body = match resp.text().await {
        Ok(body) => body,
        Err(_) => return Attempt::Retry(PageResult::file(extension, true)),
    };

    let stats = parse_html(&body, &final_url, base);
    Attempt::Done(PageResult {
        exists: true,
        extension,
        is_html: true,
        word_count: stats.word_count,
        links: stats.links,
    })
}

pub async fn fetch_page(client: &Client, url: &Url, base: &str) -> PageResult {
    let mut attempt = 1;
    loop {
        match try_fetch(client, url, base).await {
            Attempt::Done(result) => return result,
            Attempt::Retry(result) if attempt >= MAX_ATTEMPTS => return result,
            Attempt::Retry(_) => {
                let wait = WAIT_TIME * 2u32.pow(attempt - 1);
                tokio::time::sleep(wait).await;
                attempt += 1;
            }
        }
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

pub fn get_extension(url: &Url) -> String {
    let ext = url
        .path_segments()
        .and_then(|mut segs| segs.next_back())
        .unwrap_or("");
    match ext.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() => ext.to_ascii_lowercase(),
        _ => "html".to_string(),
    }
}

fn is_html_content_type(ct: &str) -> bool {
    let ct = ct.to_ascii_lowercase();
    ct.starts_with("text/html") || ct.starts_with("application/xhtml+xml")
}
