mod engine;
pub mod webstats;

use std::time::Instant;

use engine::crawler::crawl;
use engine::fetcher::build_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = std::env::args().nth(1).ok_or("usage: cargo run -- <url>")?;
    let start = url::Url::parse(&arg)?;

    let client = build_client()?;
    let started = Instant::now();
    let stats = crawl(&client, &start).await;

    println!("{stats}");
    println!("took {:.2?}", started.elapsed());
    Ok(())
}
