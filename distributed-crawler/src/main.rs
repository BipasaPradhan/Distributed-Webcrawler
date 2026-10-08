mod cluster;
mod engine;
pub mod webstats;

use std::error::Error;
use std::time::Instant;

use crate::cluster::cli::{Cli, Command};
use crate::cluster::jobs::submit;
use crate::cluster::node::run_node;
use crate::engine::{crawler::crawl, fetcher::build_client};

use clap::Parser;
use redis::aio::MultiplexedConnection;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Node => {
            let connection = connect(&cli.redis).await?;
            run_node(connection).await?
        }

        Command::Submit { urls } => {
            let mut connection = connect(&cli.redis).await?;
            for raw_url in urls {
                let url = match Url::parse(&raw_url) {
                    Ok(url) => url,
                    Err(e) => {
                        eprintln!("skipping {raw_url}: {e}");
                        continue;
                    }
                };

                let (id, url_job_hash) = submit(&mut connection, &url).await?;
                let note = if url_job_hash {
                    ""
                } else {
                    "  (already submitted)"
                };
                println!("job {id:<4} {url}{note}");
            }
        }

        Command::Status { follow, job } => todo!("step 7: status {job} (follow: {follow})"),

        Command::Stats { job } => todo!("step 8: stats {job}"),

        Command::Local { url } => {
            let start = Url::parse(&url)?;

            let client = build_client()?;

            let started = Instant::now();

            let stats = crawl(&client, &start).await;

            println!("{stats}");
            println!("took {:.2?}", started.elapsed());
        }
    }
    Ok(())
}

async fn connect(redis_url: &str) -> Result<MultiplexedConnection, Box<dyn Error>> {
    let client = redis::Client::open(redis_url)?;
    Ok(client.get_multiplexed_async_connection().await?)
}
