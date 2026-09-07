-- Local library scanner (M3).
-- Watched folders on disk + the video files found in them, each optionally
-- matched to a cached media row. All timestamps are RFC3339 UTC strings.

CREATE TABLE library_folder (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    path       TEXT NOT NULL UNIQUE,          -- absolute path
    enabled    INTEGER NOT NULL DEFAULT 1,
    added_at   TEXT NOT NULL,
    scanned_at TEXT                            -- last successful scan
);

CREATE TABLE library_file (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id      INTEGER NOT NULL REFERENCES library_folder (id) ON DELETE CASCADE,
    path           TEXT NOT NULL UNIQUE,       -- absolute path
    file_name      TEXT NOT NULL,
    size_bytes     INTEGER,
    modified_at    TEXT,                       -- filesystem mtime

    -- anitomy parse output
    parsed_title   TEXT,
    parsed_episode INTEGER,                    -- NULL when absent or a range/batch
    parsed_season  INTEGER,
    parsed_year    INTEGER,
    resolution     TEXT,                       -- e.g. '1080p'
    release_group  TEXT,

    -- match to media_cache (service, id); NULL = unmatched / in the review queue
    service        TEXT,
    media_id       INTEGER,
    match_kind     TEXT,                       -- 'auto' | 'manual'
    match_score    REAL,                       -- auto-match confidence, 0..1

    scanned_at     TEXT NOT NULL
);

CREATE INDEX idx_library_file_media  ON library_file (service, media_id);
CREATE INDEX idx_library_file_folder ON library_file (folder_id);
CREATE INDEX idx_library_file_unmatched ON library_file (media_id) WHERE media_id IS NULL;
