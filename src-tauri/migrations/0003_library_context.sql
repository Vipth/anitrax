-- Folder-aware matching (M3 follow-up).
-- Files often carry only a generic title ("Sword Art Online") while the
-- containing folder says which entry it is. We store that folder guess, and
-- remember manual links as rules so a whole show/season links in one click.

ALTER TABLE library_file ADD COLUMN folder_title TEXT;

CREATE TABLE library_link_rule (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title_key  TEXT NOT NULL,          -- normalised folder/file title
    season     INTEGER,                -- NULL matches any season
    service    TEXT NOT NULL,
    media_id   INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (title_key, season)
);
