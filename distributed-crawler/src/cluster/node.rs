use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, RedisResult};
use reqwest::Client;
use tokio::task::JoinSet;

use crate::cluster::keys;
use crate::engine::fetcher::{build_client, fetch_page};

use crate::webstats::PageResult;
use url::Url;

// at most 10 in flight requests per node
pub const TASKS: usize = 10;

// how long a task waits before checking a (then) empty queue
const WAIT_TIME: Duration = Duration::from_millis(100);

pub async fn run_node(connection: MultiplexedConnection) -> Result<(), Box<dyn Error>> {
    let http_client = build_client()?;
    eprintln!("node started with: {TASKS} workers");

    let mut workers = JoinSet::new();
    for _ in 0..TASKS {
        workers.spawn(worker(connection.clone(), http_client.clone()));
    }

    while let Some(result) = workers.join_next().await {
        result?;
    }
    Ok(())
}

async fn worker(mut connection: MultiplexedConnection, http_client: Client) {
    // job id -> base path (cache)
    let mut bases: HashMap<String, String> = HashMap::new();
    loop {
        let item: Option<String> = match connection.rpop(keys::QUEUE, None).await {
            Ok(item) => item,
            Err(e) => {
                eprintln!("redis error while popping: {e}");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        // empty queue
        let Some(item) = item else {
            tokio::time::sleep(WAIT_TIME).await;
            continue;
        };

        // parse <job>\t<url>
        let Some((job, url)) = keys::parse_queue_item(&item) else {
            eprintln!("skipping malformed queue item: {item:?}");
            continue;
        };

        if let Err(e) = process(&mut connection, &http_client, &mut bases, job, url).await {
            eprintln!("redis error while processing {url}: {e}");
        }
    }
}

/// Processes 1 url of 1 job
async fn process(
    connection: &mut MultiplexedConnection,
    http_client: &Client,
    bases: &mut HashMap<String, String>,
    job: &str,
    url: &str,
) -> RedisResult<()> {
    let stats_key = keys::stats(job);
    let _: i64 = connection.hincr(&stats_key, keys::IN_FLIGHT, 1).await?; // increases in_flight counter 

    // look up base
    let base = match bases.get(job) {
        Some(base) => base.clone(),
        None => {
            let base: String = connection.hget(keys::job(job), keys::URL).await?;
            bases.insert(job.to_string(), base.clone());
            base
        }
    };

    let page = match Url::parse(url) {
        Ok(u) => fetch_page(http_client, &u, &base).await,
        Err(_) => PageResult::missing("html".to_string()),
    };

    // schedule links
    schedule_links(connection, job, &page.links).await?;

    let mut pipe = redis::pipe();
    pipe.atomic();
    pipe.hincr(&stats_key, keys::CRAWLED, 1).ignore();
    if page.exists {
        pipe.hincr(&stats_key, keys::FILES, 1).ignore();
        pipe.hincr(keys::exts(job), &page.extension, 1).ignore();
        pipe.hincr(&stats_key, keys::WORDS, page.word_count)
            .ignore();
    }
    pipe.hincr(&stats_key, keys::IN_FLIGHT, -1).ignore();
    pipe.hincr(&stats_key, keys::PENDING, -1);
    let (remaining,): (i64,) = pipe.query_async(connection).await?;

    if remaining == 0 {
        let _: () = connection
            .hset(keys::job(job), keys::STATE, keys::DONE)
            .await?;
        eprintln!("job {job} done");
    }
    Ok(())
}

async fn schedule_links(
    connection: &mut MultiplexedConnection,
    job: &str,
    links: &[Url],
) -> RedisResult<()> {
    if links.is_empty() {
        return Ok(());
    }

    // add new links to seen set
    let seen_key = keys::seen(job);
    let mut pipe = redis::pipe();

    for link in links {
        pipe.sadd(&seen_key, link.as_str());
    }

    let added: Vec<i64> = pipe.query_async(connection).await?;

    let new_links: Vec<&Url> = links
        .iter()
        .zip(added)
        .filter(|(_, added)| *added == 1)
        .map(|(link, _)| link)
        .collect();

    if new_links.is_empty() {
        return Ok(());
    }

    // add new links -> increase pending first then enqueue
    let mut pipe = redis::pipe();
    pipe.atomic();
    pipe.hincr(keys::stats(job), keys::PENDING, new_links.len() as i64)
        .ignore();

    for link in new_links {
        pipe.lpush(keys::QUEUE, keys::queue_item(job, link.as_str()))
            .ignore();
    }

    let _: () = pipe.query_async(connection).await?;
    Ok(())
}
