-- AniList's `Media.popularity` (number of users with the title on a list).
-- Used by the local-library matcher to break ties: an obscure entry that shares
-- a synonym with a hugely popular one ("Onigiri" / "Demon Slayer") should lose.

ALTER TABLE media_cache ADD COLUMN popularity INTEGER;
