use std::collections::{HashSet, VecDeque};
use std::time::Duration;
use tokio::time::{MissedTickBehavior, interval};

use reqwest::Client;
use tokio::task::JoinSet;
use url::Url;

use crate::engine::fetcher::fetch_page;
use crate::webstats::{PageResult, WebStats};

// At most 10 reqs
pub const MAX_IN_FLIGHT: usize = 10;

// How often to print
const PROGRESS_TIME: Duration = Duration::from_secs(3);

pub async fn crawl(client: &Client, start: &Url) -> WebStats {
    let mut start = start.clone();
    start.set_fragment(None);
    let base = start.to_string();

    // urls we've discovered
    let mut seen: HashSet<Url> = HashSet::from([start.clone()]);

    // urls waiting to be fetched
    let mut frontier: VecDeque<Url> = VecDeque::from([start]);

    // urls being crawled
    let mut in_flight: JoinSet<PageResult> = JoinSet::new();

    let mut stats = WebStats::default();

    // progress reporting
    let mut crawled = 0usize;
    let mut ticker = interval(PROGRESS_TIME);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker.tick().await;

    loop {
        // fill jobs until max number
        while in_flight.len() < MAX_IN_FLIGHT {
            let Some(url) = frontier.pop_front() else {
                break;
            };
            let client = client.clone();
            let base = base.clone();

            in_flight.spawn(async move { fetch_page(&client, &url, &base).await });
        }

        let joined = tokio::select! {
            joined = in_flight.join_next() => joined,
            _ = ticker.tick() => {
                print_progress(crawled, frontier.len(), in_flight.len(), "running");
                continue; // nothing finished yet; go back and wait again
            }
        };

        let Some(joined) = joined else { break };
        let page = joined.expect("crawler panicked");
        crawled += 1;

        stats.record(&page);
        for link in page.links {
            if seen.insert(link.clone()) {
                frontier.push_back(link);
            }
        }
    }
    print_progress(crawled, frontier.len(), in_flight.len(), "done");
    stats
}

fn print_progress(crawled: usize, frontier: usize, in_flight: usize, state: &str) {
    eprintln!("crawled {crawled} | frontier {frontier} | in flight {in_flight} {state}");
}
