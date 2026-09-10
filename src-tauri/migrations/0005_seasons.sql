-- Season browser (M4). One cached result set per (year, season); the media
-- objects themselves live in media_cache, this just records membership + order.

CREATE TABLE season_state (
    year       INTEGER NOT NULL,
    season     TEXT NOT NULL,              -- WINTER | SPRING | SUMMER | FALL
    fetched_at TEXT NOT NULL,
    PRIMARY KEY (year, season)
);

CREATE TABLE season_media (
    year       INTEGER NOT NULL,
    season     TEXT NOT NULL,
    media_id   INTEGER NOT NULL,
    sort_order INTEGER NOT NULL,           -- popularity rank within the season
    PRIMARY KEY (year, season, media_id)
);

CREATE INDEX idx_season_media ON season_media (year, season, sort_order);
