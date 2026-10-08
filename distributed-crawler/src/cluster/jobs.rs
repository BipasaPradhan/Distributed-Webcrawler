use std::collections::HashMap;

use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, RedisResult};
use url::Url;

use crate::cluster::keys;
use crate::webstats::{JobStatus, WebStats};

pub async fn submit(
    connection: &mut MultiplexedConnection,
    url: &Url,
) -> RedisResult<(String, bool)> {
    let mut url = url.clone();
    url.set_fragment(None);
    let url = url.to_string();

    // Generating Job IDs
    let n: u64 = connection.incr(keys::NEXT_ID, 1).await?; // increase crawl::next_id by 1
    let id = n.to_string();

    // map url to job id (does not add same url twice)
    let url_job_hash: bool = connection.hset_nx(keys::URL_JOB, &url, &id).await?;
    if !url_job_hash {
        let existing_url_job_map: String = connection.hget(keys::URL_JOB, &url).await?;
        return Ok((existing_url_job_map, false));
    }

    let _: () = redis::pipe()
        .atomic()
        .hset_multiple(
            keys::job(&id),
            &[(keys::URL, url.as_str()), (keys::STATE, keys::RUNNING)],
        )
        .ignore()
        .sadd(keys::seen(&id), &url)
        .ignore()
        .hincr(keys::stats(&id), keys::PENDING, 1)
        .ignore()
        .lpush(keys::QUEUE, keys::queue_item(&id, &url))
        .ignore()
        .query_async(connection)
        .await?;

    Ok((id, true))
}

pub async fn status(
    connection: &mut MultiplexedConnection,
    id: &str,
) -> RedisResult<Option<JobStatus>> {
    let info: HashMap<String, String> = connection.hgetall(keys::job(id)).await?;

    let Some(url) = info.get(keys::URL).cloned() else {
        return Ok(None);
    };

    let counters: HashMap<String, i64> = connection.hgetall(keys::stats(id)).await?;
    let get = |field: &str| counters.get(field).copied().unwrap_or(0);

    Ok(Some(JobStatus {
        url,
        done: info.get(keys::STATE).map(String::as_str) == Some(keys::DONE),
        crawled: get(keys::CRAWLED),
        in_flight: get(keys::IN_FLIGHT),
        pending: get(keys::PENDING),
    }))
}
