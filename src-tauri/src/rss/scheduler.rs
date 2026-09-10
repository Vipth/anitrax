//! Orchestration for RSS auto-download: poll enabled feeds, evaluate each new
//! item against the rules bound to that feed, dedupe against `rss_history`, and
//! hand matches to the download client. Runs on a timer (`lib.rs`) and on the
//! "Check feeds now" command.

use std::time::Duration;

use tauri::AppHandle;

use crate::db::repo;
use crate::download::{qbittorrent::QbClient, AddTorrent, DownloadClient};
use crate::error::AppResult;
use crate::library::scanner;
use crate::rss::rules::{self, ParsedItem};
use crate::rss::{feeds, CheckReport};
use crate::state::AppState;

/// How often the background poller runs. One request per feed per tick — torrent
/// feeds expect this cadence, and it's nowhere near AniList (which this never
/// touches anyway).
pub const POLL_EVERY: Duration = Duration::from_secs(15 * 60);
/// Setting key: master switch for the background poller. Default on.
pub const RSS_POLL_KEY: &str = "rss_poll_enabled";

/// Run the anitomy parser over a release title for the fields rules filter on.
/// `title` stays the raw feed title so `title_contains` matches the whole thing.
fn parse_item(raw_title: &str) -> ParsedItem {
    let p = scanner::parse_name(raw_title);
    ParsedItem {
        title: raw_title.to_string(),
        episode: p.episode,
        resolution_height: p.resolution.as_deref().and_then(rules::resolution_height),
        release_group: p.release_group,
    }
}

/// Check every enabled feed once. Serialised by `state.rss_lock` so the timer
/// and a manual trigger can't double-add.
pub async fn check_all_feeds(state: &AppState, app: &AppHandle) -> AppResult<CheckReport> {
    let _guard = state.rss_lock.lock().await;
    let mut report = CheckReport::default();

    let feeds = repo::enabled_feeds(&state.db).await?;
    report.feeds_checked = feeds.len();

    // Build the download client once per run so its session is reused.
    let qb_cfg = repo::get_qb_config(&state.db).await?;
    let client: Option<QbClient> = if qb_cfg.is_configured() {
        QbClient::new(qb_cfg).ok()
    } else {
        None
    };

    for feed in feeds {
        let items = match feeds::fetch(&state.http, &feed.url).await {
            Ok(items) => {
                repo::mark_feed_fetched(&state.db, feed.id, None).await.ok();
                items
            }
            Err(e) => {
                let msg = e.to_string();
                repo::mark_feed_fetched(&state.db, feed.id, Some(&msg))
                    .await
                    .ok();
                report.errors.push(format!("{}: {msg}", feed.name));
                continue;
            }
        };

        let active: Vec<_> = repo::rules_for_feed(&state.db, feed.id)
            .await?
            .into_iter()
            .filter(|r| r.enabled)
            .collect();

        for item in items {
            report.items_seen += 1;
            if active.is_empty() {
                continue;
            }
            if repo::history_has(&state.db, &item.guid).await? {
                continue;
            }

            let parsed = parse_item(&item.title);
            let Some(rule) = active
                .iter()
                .find(|r| rules::evaluate(r, &parsed).is_download())
            else {
                continue;
            };

            let Some(client) = client.as_ref() else {
                report
                    .errors
                    .push(format!("{} matched but qBittorrent isn't set up", item.title));
                continue;
            };

            let add = AddTorrent {
                link: item.link.clone(),
                save_path: rule.dest_path.clone().filter(|p| !p.trim().is_empty()),
                category: rule.category.clone().filter(|c| !c.trim().is_empty()),
                paused: rule.paused,
            };

            match client.add(&add).await {
                Ok(()) => {
                    repo::record_history(
                        &state.db,
                        &item.guid,
                        rule.id,
                        feed.id,
                        &item.title,
                        &item.link,
                        parsed.episode,
                    )
                    .await?;
                    report.added += 1;
                    notify_added(app, &rule.name, &item.title);
                }
                Err(e) => report.errors.push(format!("{}: {e}", item.title)),
            }
        }
    }

    report.finished_at = chrono::Utc::now().to_rfc3339();
    Ok(report)
}

fn notify_added(app: &AppHandle, rule_name: &str, title: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title(format!("Downloading — {rule_name}"))
        .body(title)
        .show();
}
