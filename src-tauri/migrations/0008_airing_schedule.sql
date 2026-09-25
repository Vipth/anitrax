-- Airing calendar (M9). Unlike season_media, this is a flat pointer pool, not
-- per-partition membership: rows are upserted opportunistically and never
-- deleted, since a stale row for a show that's left the tracked-status set is
-- harmless (the frontend only ever renders rows it can also find in the
-- user's current list). schedule_state just tracks which (year, month)
-- windows have been fetched, and when, for TTL purposes.

CREATE TABLE airing_schedule (
    service   TEXT NOT NULL,
    media_id  INTEGER NOT NULL,
    episode   INTEGER NOT NULL,
    airing_at TEXT NOT NULL,               -- RFC3339 UTC
    PRIMARY KEY (service, media_id, episode)
);

CREATE INDEX idx_airing_schedule_time ON airing_schedule (airing_at);

CREATE TABLE schedule_state (
    year       INTEGER NOT NULL,
    month      INTEGER NOT NULL,           -- 1-12
    fetched_at TEXT NOT NULL,
    PRIMARY KEY (year, month)
);
