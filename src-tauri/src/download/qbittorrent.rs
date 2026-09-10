//! qBittorrent Web API client (v2). Docs:
//! <https://github.com/qbittorrent/qBittorrent/wiki/WebUI-API-(qBittorrent-5.0)>
//!
//! Auth is a cookie (`SID`) set by `POST /api/v2/auth/login`; the shared
//! `reqwest::Client` here carries a cookie jar, so we log in lazily and reuse
//! the session until it expires (a 403 triggers one re-login + retry).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::Client;

use crate::error::{AppError, AppResult};

use super::{AddTorrent, DownloadClient, QbConfig};

pub struct QbClient {
    cfg: QbConfig,
    http: Client,
    /// When we last completed a login. `None` = never / expired.
    logged_in_at: Mutex<Option<Instant>>,
}

/// Re-auth if the session is older than this, rather than waiting for a 403.
const SESSION_TTL: Duration = Duration::from_secs(30 * 60);

impl QbClient {
    pub fn new(cfg: QbConfig) -> AppResult<Self> {
        let http = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| AppError::other(format!("HTTP client init failed: {e}")))?;
        Ok(Self {
            cfg,
            http,
            logged_in_at: Mutex::new(None),
        })
    }

    fn base(&self) -> &str {
        self.cfg.base_url.trim().trim_end_matches('/')
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base(), path)
    }

    async fn login(&self) -> AppResult<()> {
        let resp = self
            .http
            .post(self.url("/api/v2/auth/login"))
            // qBittorrent rejects cross-origin API calls unless Referer matches
            // its own host; send it so non-localhost setups work too.
            .header("Referer", self.base())
            .form(&[
                ("username", self.cfg.username.as_str()),
                ("password", self.cfg.password.as_str()),
            ])
            .send()
            .await
            .map_err(qb_net_err)?;

        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(AppError::other(
                "qBittorrent refused the login (too many failed attempts — wait a minute, or check its host allowlist).",
            ));
        }
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() || body.trim() != "Ok." {
            return Err(AppError::other(
                "qBittorrent rejected those credentials.".to_string(),
            ));
        }
        *self.logged_in_at.lock().unwrap() = Some(Instant::now());
        Ok(())
    }

    /// Log in unless we have a session that's still within [`SESSION_TTL`].
    async fn ensure_session(&self) -> AppResult<()> {
        let fresh = self
            .logged_in_at
            .lock()
            .unwrap()
            .map(|t| t.elapsed() < SESSION_TTL)
            .unwrap_or(false);
        if fresh {
            return Ok(());
        }
        self.login().await
    }

    fn invalidate_session(&self) {
        *self.logged_in_at.lock().unwrap() = None;
    }
}

#[async_trait::async_trait]
impl DownloadClient for QbClient {
    fn name(&self) -> &'static str {
        "qBittorrent"
    }

    async fn test_connection(&self) -> AppResult<String> {
        if !self.cfg.is_configured() {
            return Err(AppError::other("Set the qBittorrent Web UI address first."));
        }
        self.login().await?;
        let version = self
            .http
            .get(self.url("/api/v2/app/version"))
            .header("Referer", self.base())
            .send()
            .await
            .map_err(qb_net_err)?
            .text()
            .await
            .map_err(qb_net_err)?;
        Ok(version.trim().to_string())
    }

    async fn add(&self, torrent: &AddTorrent) -> AppResult<()> {
        if !self.cfg.is_configured() {
            return Err(AppError::other("qBittorrent isn't set up."));
        }
        self.ensure_session().await?;

        let paused = torrent.paused.to_string();
        let mut form: Vec<(&str, String)> = vec![
            ("urls", torrent.link.clone()),
            // Both spellings — `paused` (<=4.6) and `stopped` (>=5.0).
            ("paused", paused.clone()),
            ("stopped", paused),
        ];
        if let Some(p) = &torrent.save_path {
            if !p.trim().is_empty() {
                form.push(("savepath", p.clone()));
                form.push(("autoTMM", "false".into()));
            }
        }
        if let Some(c) = &torrent.category {
            if !c.trim().is_empty() {
                form.push(("category", c.clone()));
            }
        }

        let send = |f: &Vec<(&str, String)>| {
            self.http
                .post(self.url("/api/v2/torrents/add"))
                .header("Referer", self.base())
                .form(f)
                .send()
        };

        let mut resp = send(&form).await.map_err(qb_net_err)?;
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            // Session expired mid-flight — re-auth once and retry.
            self.invalidate_session();
            self.login().await?;
            resp = send(&form).await.map_err(qb_net_err)?;
        }

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::other(format!(
                "qBittorrent returned {status} adding the torrent."
            )));
        }
        // qB replies "Fails." (HTTP 200) when it can't fetch/parse the torrent.
        if body.trim().eq_ignore_ascii_case("fails.") {
            return Err(AppError::other(
                "qBittorrent couldn't add that torrent (bad or unreachable link).",
            ));
        }
        Ok(())
    }
}

fn qb_net_err(e: reqwest::Error) -> AppError {
    AppError::other(format!(
        "Couldn't reach qBittorrent — is the Web UI running and the address right? ({e})"
    ))
}
