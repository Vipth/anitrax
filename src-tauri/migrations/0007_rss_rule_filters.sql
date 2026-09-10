-- M5 follow-up: two more rule filters.
--   exclude_contains — space-separated words; the item is rejected if ANY appear
--                      in its release title (kills "Batch", "V2", "S2", …).
--   season           — anitomy-parsed season number the release must be; an
--                      untagged title counts as season 1.

ALTER TABLE rss_rule ADD COLUMN exclude_contains TEXT;
ALTER TABLE rss_rule ADD COLUMN season INTEGER;
