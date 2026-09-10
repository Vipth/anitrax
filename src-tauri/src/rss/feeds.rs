//! Fetch and parse a torrent RSS feed into a flat list of items. RSS 2.0 only
//! (Nyaa, AniDex, TokyoTosho, subsplease.org — all RSS 2.0); Atom feeds aren't
//! supported. The download link is taken from `<enclosure>` when present (its
//! `.torrent` / magnet URL), otherwise `<link>` — which for Nyaa is the direct
//! `.torrent` download.

use reqwest::Client;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct FeedItem {
    /// Stable id for dedupe — the item's `<guid>`, or `<link>` when absent.
    pub guid: String,
    pub title: String,
    /// Magnet URI or `.torrent` URL to hand to the download client.
    pub link: String,
}

/// A polite cap — torrent feeds are small, but a misconfigured URL shouldn't
/// let us pull a huge body.
const MAX_BODY: usize = 4 * 1024 * 1024;

pub async fn fetch(http: &Client, url: &str) -> AppResult<Vec<FeedItem>> {
    let resp = http
        .get(url)
        .header("User-Agent", "AniTrax/0.1 (+https://github.com/Vipth/anitrax)")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("couldn't fetch the feed: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::other(format!(
            "the feed returned HTTP {}",
            resp.status()
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::Network(format!("couldn't read the feed body: {e}")))?;
    if bytes.len() > MAX_BODY {
        return Err(AppError::other("that feed is unexpectedly large — wrong URL?"));
    }

    parse(&bytes)
}

fn parse(bytes: &[u8]) -> AppResult<Vec<FeedItem>> {
    let channel = rss::Channel::read_from(bytes)
        .map_err(|e| AppError::other(format!("that doesn't look like an RSS feed: {e}")))?;

    let items = channel
        .items()
        .iter()
        .filter_map(|it| {
            let title = it.title()?.trim().to_string();
            if title.is_empty() {
                return None;
            }

            // Prefer an enclosure (the actual torrent/magnet); fall back to the
            // item link (Nyaa's link *is* the .torrent download).
            let link = it
                .enclosure()
                .map(|e| e.url().to_string())
                .filter(|u| !u.is_empty())
                .or_else(|| it.link().map(str::to_string))
                .filter(|u| !u.is_empty())?;

            // Dedupe id: <guid>, else the item's page <link>, else the torrent
            // link itself. The page link is a steadier id than a magnet URI.
            let guid = it
                .guid()
                .map(|g| g.value().to_string())
                .filter(|g| !g.is_empty())
                .or_else(|| it.link().map(str::to_string).filter(|l| !l.is_empty()))
                .unwrap_or_else(|| link.clone());

            Some(FeedItem { guid, title, link })
        })
        .collect();

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NYAA_SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:nyaa="https://nyaa.si/xmlns/nyaa">
  <channel>
    <title>Nyaa</title>
    <item>
      <title>[SubsPlease] Frieren - 05 (1080p) [ABCD1234].mkv</title>
      <link>https://nyaa.si/download/1700000.torrent</link>
      <guid isPermaLink="true">https://nyaa.si/view/1700000</guid>
      <pubDate>Fri, 05 Sep 2026 14:00:00 -0000</pubDate>
    </item>
    <item>
      <title>[Erai-raws] Some Show - 12 [720p].mkv</title>
      <enclosure url="magnet:?xt=urn:btih:DEADBEEF" type="application/x-bittorrent" />
      <link>https://nyaa.si/view/1700001</link>
      <pubDate>Fri, 05 Sep 2026 15:00:00 -0000</pubDate>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn parses_link_and_enclosure_variants() {
        let items = parse(NYAA_SAMPLE.as_bytes()).unwrap();
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].guid, "https://nyaa.si/view/1700000");
        assert_eq!(items[0].link, "https://nyaa.si/download/1700000.torrent");
        assert!(items[0].title.contains("Frieren"));

        // enclosure wins over <link>; guid falls back to the link.
        assert_eq!(items[1].link, "magnet:?xt=urn:btih:DEADBEEF");
        assert_eq!(items[1].guid, "https://nyaa.si/view/1700001");
    }

    #[test]
    fn rejects_non_rss() {
        assert!(parse(b"<html><body>nope</body></html>").is_err());
    }
}
