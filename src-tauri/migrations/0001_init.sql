-- Anime Tracker — initial schema
-- All timestamps are stored as RFC3339 UTC strings.

CREATE TABLE account (
    service       TEXT PRIMARY KEY,          -- 'anilist' | 'kitsu'
    user_id       TEXT NOT NULL,
    user_name     TEXT NOT NULL,
    avatar_url    TEXT,
    score_format  TEXT NOT NULL DEFAULT 'POINT_10',
    is_primary    INTEGER NOT NULL DEFAULT 0,
    connected_at  TEXT NOT NULL
);

CREATE TABLE media_cache (
    service         TEXT NOT NULL,
    id              INTEGER NOT NULL,
    title_romaji    TEXT,
    title_english   TEXT,
    title_native    TEXT,
    format          TEXT,
    airing_status   TEXT,
    description     TEXT,
    episodes        INTEGER,
    duration        INTEGER,
    season          TEXT,
    season_year     INTEGER,
    cover_url       TEXT,
    cover_color     TEXT,
    banner_url      TEXT,
    average_score   INTEGER,
    genres_json     TEXT NOT NULL DEFAULT '[]',
    synonyms_json   TEXT NOT NULL DEFAULT '[]',
    start_date      TEXT,
    site_url        TEXT,
    next_episode    INTEGER,                 -- next airing episode number
    next_airing_at  TEXT,                    -- RFC3339 of next airing
    meta_fetched_at TEXT NOT NULL,           -- when full metadata was last refreshed
    airing_fetched_at TEXT,                  -- when nextAiring was last refreshed
    PRIMARY KEY (service, id)
);

CREATE TABLE list_entry (
    service      TEXT NOT NULL,
    media_id     INTEGER NOT NULL,
    remote_id    INTEGER,                    -- the service's MediaList entry id
    status       TEXT NOT NULL,              -- ListStatus
    progress     INTEGER NOT NULL DEFAULT 0,
    score_raw    INTEGER NOT NULL DEFAULT 0, -- 0..100
    repeat       INTEGER NOT NULL DEFAULT 0,
    notes        TEXT,
    started_at   TEXT,
    completed_at TEXT,
    updated_at   TEXT NOT NULL,             -- last local mutation
    remote_updated_at TEXT,                 -- last known server updatedAt
    dirty        INTEGER NOT NULL DEFAULT 0,
    deleted      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (service, media_id),
    FOREIGN KEY (service, media_id) REFERENCES media_cache (service, id) ON DELETE CASCADE
);

CREATE INDEX idx_list_entry_dirty ON list_entry (dirty) WHERE dirty = 1;
CREATE INDEX idx_list_entry_status ON list_entry (service, status);

CREATE TABLE app_setting (
    key        TEXT PRIMARY KEY,
    value_json TEXT NOT NULL
);

CREATE TABLE sync_state (
    service        TEXT PRIMARY KEY,
    last_full_sync TEXT
);
