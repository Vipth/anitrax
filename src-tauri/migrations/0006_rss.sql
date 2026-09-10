-- RSS auto-download (M5). Watch torrent feeds, match new releases to tracked
-- shows, hand the magnet / .torrent URL to a download client (qBittorrent).
-- All timestamps are RFC3339 UTC strings.

CREATE TABLE rss_feed (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    url             TEXT NOT NULL UNIQUE,
    enabled         INTEGER NOT NULL DEFAULT 1,
    added_at        TEXT NOT NULL,
    last_fetched_at TEXT,
    last_error      TEXT                         -- last fetch/parse failure, cleared on success
);

CREATE TABLE rss_rule (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    name           TEXT NOT NULL,
    enabled        INTEGER NOT NULL DEFAULT 1,

    -- Which feed this rule watches. NULL = every enabled feed.
    feed_id        INTEGER REFERENCES rss_feed (id) ON DELETE CASCADE,

    -- The tracked show this rule feeds. NULL is allowed (a title-only rule) but
    -- the UI always sets it — episode-range logic needs it.
    service        TEXT,
    media_id       INTEGER,

    -- Match constraints. All optional; an unset field doesn't filter.
    title_contains TEXT,                         -- case-insensitive substring, all words must appear
    release_group  TEXT,                         -- exact (case-insensitive) release group
    min_resolution INTEGER,                      -- vertical pixels, e.g. 1080
    episode_from   INTEGER,
    episode_to     INTEGER,

    -- Where matched torrents go.
    dest_path      TEXT,
    category       TEXT,
    paused         INTEGER NOT NULL DEFAULT 0,   -- add to the client paused

    created_at     TEXT NOT NULL
);

CREATE INDEX idx_rss_rule_feed ON rss_rule (feed_id);

-- One row per downloaded item. `guid` is the feed item's <guid> (or <link> when
-- absent); the dedupe check treats a guid as spent regardless of which rule
-- grabbed it, so an item is never added twice.
CREATE TABLE rss_history (
    guid         TEXT PRIMARY KEY,
    rule_id      INTEGER REFERENCES rss_rule (id) ON DELETE SET NULL,
    feed_id      INTEGER,
    title        TEXT NOT NULL,
    link         TEXT NOT NULL,                  -- magnet or .torrent URL handed to the client
    episode      INTEGER,
    downloaded_at TEXT NOT NULL
);

CREATE INDEX idx_rss_history_time ON rss_history (downloaded_at DESC);
