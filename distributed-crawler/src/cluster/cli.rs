use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "crawl")]

pub struct Cli {
    // redis address
    #[arg(
        long,
        global = true,
        env = "REDIS_URL",
        default_value = "redis://127.0.0.1:6379/"
    )]
    pub redis: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Starts a crawler node that joins the cluster
    Node,

    /// Submit one or more URLs (each URL is its own crawl job)
    Submit {
        #[arg(required = true)]
        urls: Vec<String>,
    },

    /// Show a job's progress
    Status {
        #[arg(short = 'f', long)]
        follow: bool,
        job: String,
    },

    /// Show a finished job's WebStats
    Stats { job: String },

    /// Local single node crawler
    Local { url: String },
}
