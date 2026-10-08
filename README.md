# Mini 1: Distributed-Webcrawler

The distributed web crawler is a web crawler that spreads its work across any `N` number of machines (or nodes). The node processes can be run on multiple terminals on one's local machine or using other machines (has not been tested). 

**The typical life cycle is:** You submit a (or multiple) URL $\rightarrow$ the crawler nodes crawl through the URL $\rightarrow$ reports `WebStats`

There are 3 main parts:
1. **Redis:** The shared component that has the "global" work queue, the job's state, and their statistics. 

2. **Nodes:** These are the heavy workers that do the crawling. Each node runs 10 worker tasks that take URLs from the queue, fetch and parse them, and record the results. 

3. **The CLI:** The interface in which the user can use to create jobs, see their progress, and their statistics. 

The nodes coordinate through Redis, so a new node can join at anytime. 

## Running the Crawler
### Required:
* Rust with Cargo 
* Docker and Docker Compose 

### Step 1: Starting Redis
Once you have cloned the repository, from the project root, you can start redis by:

```bash
docker compose up -d
```
It is written to listen on port `6379`. Writing `docker compose exec redis redis-cli ping` should return `PONG`.

### Step 2: Building and Installing the CLI
Go inside the `distributed-crawler` directory:

```bash
 cd distributed-crawler
```
There are 2 ways to make `crawl` work. 

**The first way:**
```bash
cargo install --path .
```
Then you can use the `crawl` command from the  `distributed-crawler` directory. 

**The second way:**

Run the command (to build the crawler) on the  `distributed-crawler` directory (where the `Cargo.toml` file is). 

```bash
cargo build --release
```
And use `./target/release/crawl` instead of just `crawl`

### Step 3: Launching the Nodes
Open 1 terminal per node and run:
```bash
crawl node
```
or the `./target` version

Each node prints `node started with: 10 workers` and keeps running. The node can be stopped with a `Ctrl+C` but only when the jobs are done. 

### Step 4: The CLI
```bash
crawl submit <url> [<url>...]    # one job per URL -> it will print the job IDs
crawl status <id>    # for progress, printed once
crawl status -f <id>    # for progress, updated every second until done
crawl stats <id>    # final WebStats and how long it took (once done)
```

An example would be:
```bash
$ crawl submit http://books.toscrape.com/
job 1     http://books.toscrape.com/

$ crawl status -f 1
crawled: 812  | frontier: 140 | in flight: 20 | running
...
crawled: 3203 | frontier: 0   | in flight: 0  | done

$ crawl stats 1
files: 3203 | extensions: 5 | words: 428028
jpg 2000 | html 1195 | js 4 | css 3 | ico 1
Took ...
```
 
### Notes
* The submitted URL is both the start page and the base path: only URLs that begin with it are crawled (as per the spec).

* Submitting a URL that was already submitted returns the existing job with a message that it was already submitted. 

* `crawl stats` on an unfinished job shows its progress instead with a message that the job isn't done yet.

* `crawl local <url>` crawls a site in a single process without Redis, as a reference. 

* To delete all jobs (but still keep redis running), you can do:
```bash
docker compose exec redis redis-cli FLUSHALL
```
In the case you want to submit the same URLs and try again.

### Running on Another Machine
> This method has not ben tested (safer to stick with the local version). 

Only 1 machine should be running Redis. For the other machines or nodes, build the crawler (same as the **step 2** above) and use your machine's IP address instead:

```bash
export REDIS_URL=redis://<machine-ip>:6379/
crawl node
crawl status -f 1      
```

## How the Nodes Coordinate 
The nodes do not talk to each other. The nodes and the CLI coordinates through Redis. Redis holds:

Key | Contents/Explanation|Type|
----|---------|----------------|
`crawl:queue` | The main work queue | list (dequeue)|
`crawl:url_job` | URL $\rightarrow$ job ID mapping | hash|
`crawl:next_id` | The job ID counter | string |
`job:<id>` | Holds the URL, the state of the job, and the times (for Duration)|hash|
`job:<id>:seen` | A set of URLs encountered| set |
`job:<id>:stats` | The Webstats | hash|
`job:<id>:ext` | Extensions $\rightarrow$ count mapping| hash|

* Submitting a URL assigns it a new job ID and checks that the URL isn't taken yet (using Redis instructions)

* The URL is then pushed to the main queue and the base URL is added to the seen set (a `pending` counter is incremented to track unfinished work).

* The workers continuously pull URLs from the queue to process them. 

    * The workers then fetch the page and parses them for links. The URL is then "active" (an `in_flight` counter is incremented).

    * For every new link it finds, it is pushed into the queue (after beeing checked against the seen list, and the `pending` counter is incremented accordingly).

    * The statistics are then recorded (piped with `.atomic` with Redis commands that are atomic). The `pending` counter is decremented. 

    * The job is done when `pending` is 0.

* Since the nodes are stateless and everything is updated into Redis, nodes can be added at any time. 

* This means that to get the stats and statuses, the structures set in Redis are queried. 

## How Progress is Tracked
* The progress lives in `job:<id>:stats` in Redis, which is updated by the workers in the node. The key statistics are:

    * **crawled:** Total URLs processed (including broken ones).

    * **in_flight:** URLs actively being downloaded.

    * **frontier:** Queued URLs waiting to be picked (`pending − in_flight`).

        * The `pending` counter is used for unfinished work. Minusing active tasks gives the URLs that are waiting in the queue. 
 
    * **state:** Tracks if the job is `running` or `done`.

* `crawl status` fetches these statistics and adding `-f` polls Redis every second. 

* The duration uses Redis's server time instead of the machine's. 

* `crawl stats` reads the file counts, word counts, and extension hash for the result summary.

These are all updated using Redis commands from the Redis crate (which are mainly atomic in itself with atomic pipelines where necessary).

## How the Cluster Knows a Job is Done
The nodes do not rely on an empty queue to know that it is done since we don't know if there is more coming. It uses a counter instead. 
* `pending`: Tracks all the unfinished work (ones queued and being processed $\rightarrow$ `in_flight`)

    * When you submit a URL, the counter is initialized (`pending = 1`)

    * When new links are discovered, the counter is incremented by the number of new links (e.g. adding *k* links increases `pending` by *k*)

    * When a page/file finishes, the counter is decremented.

* With the way it is written, newly discovered links are added before the counter is decremented when the page is done being processed (`+k` before `-1`). So `pending` reaches 0 when there are no new items and the queue is empty.

* Since the updates are pipelined atomically, the worker that brings `pending` down to 0 is the only one that marks the job as done. There will be a message printed on the console of that running node (logic can be seen in `node.rs`). 

## Design Choices
1. **Shared Work Queue (FIFO)**: Instead of waiting for the slowest page of a level to finish (in BFS), workers can pull new URLs the moment they finish a task, keeping the workers busy. 

    * The items in the queue have the specific job ID attached, so the workers can pull from the central queue without worrying about the jobs intefering with each other. 

    * Workers check the seen set before queueing a link (`SADD`). Only URLs that return a 1 (meaning it hasn't been discovered) are pushed, so duplicates are removed when discovered. 

2. **Stateless Nodes:** All states live in Redis. Nodes get the information they need from Redis so that we can add/remove as many nodes as we need. 

    * It also makes coordinating statistics and information easier (as long as the execution is atomic). 

3. **Polling Pops:** When the workers are idle (or the queue is empty), the workers check the queue every 100 ms instead of blocking the whole node's Redis connection (`RPOP` vs. `BRPOP`). 

4. For crawling:

    * The links come from the `href` and `src` attributes since these attributes capture essentially all the web contents/files. 

    * Word counts come from bodies of text and does not include things like tags, rules, and scripts. 

    * Requests time out after 15 seconds with up to 3 retries (for network, server errors)


**The design mainly relies on the fact that Redis is single-threaded for command execution.** 

## Limitations 
The design is not fault tolerant. It follows that assumption that nodes and Redis does not crash. If a node crashes mid crawl, then the progress is lost, and their status stays running. Redis is a single point of failure. 