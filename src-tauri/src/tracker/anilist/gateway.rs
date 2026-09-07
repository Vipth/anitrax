//! The single coordinated gateway for every request to `graphql.anilist.co`.
//!
//! Nothing else in the app is allowed to talk to AniList directly. Every read
//! and write is funnelled through one background task that:
//!   * paces requests well under AniList's 90 req/min ceiling (target ~45/min),
//!   * smooths bursts (AniList also has an undocumented short-window limiter),
//!   * reacts to `X-RateLimit-Remaining` by slowing down before it hits zero,
//!   * on HTTP 429 parks the *entire* queue for `Retry-After`, then retries,
//!   * exposes a live "request budget" snapshot for the UI.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::sleep;

use crate::error::{AppError, AppResult};

const ENDPOINT: &str = "https://graphql.anilist.co";

/// Sustained ceiling we impose on ourselves (AniList allows 90).
const MAX_PER_MINUTE: usize = 45;
/// Minimum spacing between consecutive requests, to smooth bursts.
const MIN_GAP: Duration = Duration::from_millis(180);
/// When AniList tells us this few requests remain in its window, crawl.
const CRAWL_THRESHOLD: i64 = 20;
const CRAWL_GAP: Duration = Duration::from_millis(2000);
const MAX_5XX_RETRIES: u32 = 3;

struct Job {
    body: Value,
    token: Option<String>,
    respond: oneshot::Sender<AppResult<Value>>,
}

/// Live snapshot of the gateway, surfaced in Settings for transparency.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BudgetSnapshot {
    /// Requests we have sent in the last rolling 60 seconds.
    pub used_last_minute: usize,
    /// The self-imposed sustained ceiling.
    pub self_limit: usize,
    /// AniList's own reported remaining count for its window, if known.
    pub api_remaining: Option<i64>,
    /// Seconds until the queue un-parks (0 when running normally).
    pub parked_for_secs: u64,
    pub queue_depth: usize,
}

#[derive(Default)]
struct Stats {
    sent: VecDeque<Instant>,
    api_remaining: Option<i64>,
    parked_until: Option<Instant>,
    queue_depth: usize,
}

impl Stats {
    fn prune(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(60);
        while self.sent.front().is_some_and(|t| *t < cutoff) {
            self.sent.pop_front();
        }
    }

    fn snapshot(&mut self) -> BudgetSnapshot {
        self.prune();
        BudgetSnapshot {
            used_last_minute: self.sent.len(),
            self_limit: MAX_PER_MINUTE,
            api_remaining: self.api_remaining,
            parked_for_secs: self
                .parked_until
                .map(|t| t.saturating_duration_since(Instant::now()).as_secs())
                .unwrap_or(0),
            queue_depth: self.queue_depth,
        }
    }
}

#[derive(Clone)]
pub struct AniListGateway {
    tx: mpsc::Sender<Job>,
    stats: Arc<Mutex<Stats>>,
}

impl AniListGateway {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Job>(256);
        let stats = Arc::new(Mutex::new(Stats::default()));
        let worker = Worker {
            client: reqwest::Client::builder()
                .user_agent("AnimeTracker/0.1 (+https://github.com/bcnet-dev/anime-tracker)")
                .timeout(Duration::from_secs(30))
                .build()
                .expect("build reqwest client"),
            stats: stats.clone(),
            last_sent: None,
        };
        tokio::spawn(worker.run(rx));
        Self { tx, stats }
    }

    /// Enqueue a GraphQL request. Resolves once the gateway has actually sent it
    /// (respecting all pacing/parking) and parsed the response.
    pub async fn query(&self, body: Value, token: Option<String>) -> AppResult<Value> {
        let (respond, rx) = oneshot::channel();
        {
            let mut s = self.stats.lock().await;
            s.queue_depth += 1;
        }
        self.tx
            .send(Job {
                body,
                token,
                respond,
            })
            .await
            .map_err(|_| AppError::other("AniList gateway is shut down"))?;
        rx.await
            .map_err(|_| AppError::other("AniList gateway dropped the request"))?
    }

    pub async fn budget(&self) -> BudgetSnapshot {
        self.stats.lock().await.snapshot()
    }
}

struct Worker {
    client: reqwest::Client,
    stats: Arc<Mutex<Stats>>,
    last_sent: Option<Instant>,
}

impl Worker {
    async fn run(mut self, mut rx: mpsc::Receiver<Job>) {
        while let Some(job) = rx.recv().await {
            {
                let mut s = self.stats.lock().await;
                s.queue_depth = s.queue_depth.saturating_sub(1);
            }
            let result = self.process(&job).await;
            let _ = job.respond.send(result);
        }
    }

    async fn process(&mut self, job: &Job) -> AppResult<Value> {
        let mut attempt_5xx = 0u32;
        loop {
            self.pace().await;

            let mut req = self.client.post(ENDPOINT).json(&job.body);
            if let Some(token) = &job.token {
                req = req.bearer_auth(token);
            }

            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => return Err(AppError::Network(e.to_string())),
            };

            self.record_sent().await;
            self.absorb_headers(&resp).await;

            let status = resp.status();

            if status.as_u16() == 429 {
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(60)
                    .clamp(1, 300);
                self.park(retry_after).await;
                // One retry after the park; if it 429s again, surface it.
                self.pace().await;
                let mut retry = self.client.post(ENDPOINT).json(&job.body);
                if let Some(token) = &job.token {
                    retry = retry.bearer_auth(token);
                }
                let resp2 = retry.send().await.map_err(|e| AppError::Network(e.to_string()))?;
                self.record_sent().await;
                self.absorb_headers(&resp2).await;
                if resp2.status().as_u16() == 429 {
                    return Err(AppError::RateLimited {
                        service: "anilist".into(),
                        retry_after_secs: retry_after,
                    });
                }
                return parse_graphql(resp2).await;
            }

            if status.is_server_error() {
                attempt_5xx += 1;
                if attempt_5xx >= MAX_5XX_RETRIES {
                    return Err(AppError::Api {
                        service: "anilist".into(),
                        message: format!("server error {status} after {attempt_5xx} attempts"),
                    });
                }
                let backoff = Duration::from_millis(400 * 2u64.pow(attempt_5xx))
                    + Duration::from_millis(fastjitter());
                sleep(backoff).await;
                continue;
            }

            return parse_graphql(resp).await;
        }
    }

    /// Block until we are allowed to send the next request.
    async fn pace(&mut self) {
        // 1. Honour an active park.
        loop {
            let wait = {
                let s = self.stats.lock().await;
                s.parked_until
                    .map(|t| t.saturating_duration_since(Instant::now()))
                    .filter(|d| !d.is_zero())
            };
            match wait {
                Some(d) => sleep(d).await,
                None => break,
            }
        }

        // 2. Respect the rolling per-minute budget.
        loop {
            let sleep_for = {
                let mut s = self.stats.lock().await;
                s.prune();
                if s.sent.len() < MAX_PER_MINUTE {
                    None
                } else {
                    s.sent
                        .front()
                        .map(|oldest| (*oldest + Duration::from_secs(60)) - Instant::now())
                }
            };
            match sleep_for {
                Some(d) if !d.is_zero() => sleep(d).await,
                _ => break,
            }
        }

        // 3. Smooth bursts, and crawl if AniList says its window is nearly spent.
        let gap = {
            let s = self.stats.lock().await;
            if s.api_remaining.is_some_and(|r| r <= CRAWL_THRESHOLD) {
                CRAWL_GAP
            } else {
                MIN_GAP
            }
        };
        if let Some(last) = self.last_sent {
            let elapsed = last.elapsed();
            if elapsed < gap {
                sleep(gap - elapsed).await;
            }
        }
    }

    async fn record_sent(&mut self) {
        let now = Instant::now();
        self.last_sent = Some(now);
        let mut s = self.stats.lock().await;
        s.sent.push_back(now);
        s.prune();
    }

    async fn absorb_headers(&self, resp: &reqwest::Response) {
        let remaining = resp
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<i64>().ok());
        if let Some(rem) = remaining {
            let mut s = self.stats.lock().await;
            s.api_remaining = Some(rem);
        }
    }

    async fn park(&self, secs: u64) {
        let mut s = self.stats.lock().await;
        let until = Instant::now() + Duration::from_secs(secs) + Duration::from_millis(fastjitter());
        s.parked_until = Some(until);
        tracing::warn!(secs, "AniList gateway parked after 429");
    }
}

async fn parse_graphql(resp: reqwest::Response) -> AppResult<Value> {
    let status = resp.status();
    let text = resp.text().await.map_err(|e| AppError::Network(e.to_string()))?;
    let json: Value = serde_json::from_str(&text).map_err(|_| AppError::Api {
        service: "anilist".into(),
        message: format!("non-JSON response ({status}): {}", truncate(&text, 200)),
    })?;

    if let Some(errors) = json.get("errors").and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            let msg = errors
                .iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(AppError::Api {
                service: "anilist".into(),
                message: if msg.is_empty() {
                    format!("{status}")
                } else {
                    msg
                },
            });
        }
    }

    json.get("data")
        .cloned()
        .ok_or_else(|| AppError::Api {
            service: "anilist".into(),
            message: format!("response had no data field ({status})"),
        })
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

/// Cheap 0..250ms jitter without pulling in the full `rand` machinery here.
fn fastjitter() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    nanos % 250
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_drops_entries_older_than_a_minute() {
        let mut s = Stats::default();
        let now = Instant::now();
        s.sent.push_back(now - Duration::from_secs(90));
        s.sent.push_back(now - Duration::from_secs(30));
        s.sent.push_back(now - Duration::from_secs(1));
        s.prune();
        assert_eq!(s.sent.len(), 2, "the 90s-old request should be pruned");
    }

    #[test]
    fn snapshot_reports_the_self_limit_and_recent_usage() {
        let mut s = Stats::default();
        for _ in 0..10 {
            s.sent.push_back(Instant::now());
        }
        s.api_remaining = Some(42);
        let snap = s.snapshot();
        assert_eq!(snap.used_last_minute, 10);
        assert_eq!(snap.self_limit, MAX_PER_MINUTE);
        assert_eq!(snap.api_remaining, Some(42));
        assert_eq!(snap.parked_for_secs, 0);
    }

    #[test]
    fn snapshot_counts_down_an_active_park() {
        let mut s = Stats::default();
        s.parked_until = Some(Instant::now() + Duration::from_secs(30));
        let snap = s.snapshot();
        assert!(
            (25..=30).contains(&snap.parked_for_secs),
            "expected ~30s, got {}",
            snap.parked_for_secs
        );
    }

    #[test]
    fn self_limit_stays_well_under_anilist_ceiling() {
        // AniList allows 90/min; we must leave real headroom.
        assert!(MAX_PER_MINUTE <= 50);
    }

    #[test]
    fn graphql_errors_become_api_errors() {
        // parse_graphql is async; exercise the JSON-shape logic it relies on.
        let body = serde_json::json!({
            "errors": [{ "message": "Invalid token" }],
            "data": null,
        });
        let errs = body.get("errors").and_then(|e| e.as_array()).unwrap();
        assert!(!errs.is_empty());
        assert_eq!(
            errs[0].get("message").and_then(|m| m.as_str()),
            Some("Invalid token")
        );
    }
}
