# AniTrax — Milestones

A modern desktop anime tracker in the spirit of Taiga. Cross-platform (Tauri 2 +
React 19), syncs with **AniList** and **Kitsu**, architected so it never trips
AniList's rate limit.

Full design doc: `~/.claude/plans/i-want-to-create-jiggly-galaxy.md`.

---

## ✅ M1 — Scaffold + list manager + AniList sync  *(done)*

The first usable build: a polished AniList list manager with two-way sync.

- Tauri 2 + React 19 + TypeScript + Vite; Tailwind v4, TanStack Router/Query, Zustand
- **`AniListGateway`** — the single paced queue every AniList request goes through:
  45 req/min self-cap (half the ceiling), burst smoothing, header-aware slow-down,
  429 parking + retry, live request-budget meter
- SQLite cache (`media_cache` / `list_entry`) as the UI's source of truth; sync
  reconciles server state without clobbering pending local edits
- `TrackerService` trait + AniList impl: `MediaListCollection` pull,
  `SaveMediaListEntry`, aliased batch `Media`, search
- OAuth (implicit grant) + OS-keychain token storage; pin-redirect fallback
- Debounced push worker — rapid **+1 episode** clicks collapse into one mutation
- Stale-only launch sync + 30-min focused background refresh
- **Library**: status tabs, grid/list layouts, sort + filter, inline +1 & score
- **Media detail**, **Discover** (debounced search), **Settings** (connect flow + budget meter)
- Tests: gateway pacing/window, OAuth parsing, score/sort/filter helpers

**Added along the way (not in the original plan):**
- Graceful handling of AniList's "API temporarily disabled" outages — edits stay
  queued, no retry-hammering, an outage banner explains it
- Extra themes beyond light/dark/system — **Jade**, **Nord**, and the four
  **Catppuccin** flavours — each with a portable spec under `docs/`
- Seanime-inspired custom theme picker with per-theme preview swatches

**Deviations from plan:** used a hand-rolled token-bucket in the gateway instead
of the `governor` crate; kept native window decorations (no custom titlebar yet).

---

## M2 — Polish & offline

- Skeleton loaders, empty states, error toasts everywhere they're missing
- Keyboard shortcuts: `j`/`k` navigate, `+` bump episode, `/` focus filter
- Verify the app is fully readable offline; edits queue as `dirty` and flush on reconnect
- Next-episode countdowns from AniList `airingSchedule` (partly in already)
- Wire `META_TTL` into `ensure_media` so stale metadata refetches on its own

---

## M3 — Kitsu

- `KitsuService` implementing the same `TrackerService` trait (JSON:API, OAuth)
- **Verify the live base URL first** — `kitsu.io/api/edge` vs the newer
  `kitsu.cloud`; the trait keeps this contained to one file
- Settings: connect Kitsu, choose the primary service, per-service list views
- Domain-model mapping tests with recorded fixtures, mirroring the AniList ones

---

## M4 — Local library scanner

- Settings: add / remove watched folders
- `scanner.rs` — walk folders, parse filenames with `anitomy-rs`, pull resolution
  & release group
- `matcher.rs` — fuzzy-match parsed titles to `media_cache` (normalized title +
  synonyms + year); an "unmatched" queue with a manual **link to media** picker
- Owned-episode badges on cards and the detail page
- `/library-local` — file list, re-scan button, filesystem watcher for incremental updates
- Unit tests: fixture filenames → expected media

---

## M5 — Season browser + statistics

- `/seasons` — year/season picker, grid of that season's anime, filter by
  format/genre, one-click add to list (single cached `Page` query per season)
- `/stats` — episodes & time watched, score distribution, genre breakdown,
  format split, completion rate, activity over time. Computed in Rust from the
  cache, rendered with Recharts
- Verify season grid matches AniList's own season page; stats reconcile with the
  AniList profile

---

## M6 — RSS auto-download (qBittorrent)

- `/rss` — manage feeds; rule builder: bind a rule to a tracked show, set
  quality / release group / episode range, destination path, qBittorrent category
- `qbittorrent.rs` — Web API client (`/api/v2/auth/login`, `/api/v2/torrents/add`),
  connection test in Settings
- `scheduler.rs` — `tokio` interval poll, evaluate rules, dedupe via `rss_history`,
  add magnet to qBittorrent, desktop notification
- "Check feeds now" button + per-feed download history
- Unit tests: feed item + rule → download / skip decision

---

## Verification (per milestone)

- **M1:** connect AniList, list renders with correct per-status counts, +1 shows
  on anilist.co within seconds, offline relaunch still renders, 10-min hammer
  test stays under 45 req/min with zero 429s
- **M3:** Kitsu round-trip mirrors the AniList checks
- **M4:** scan a real folder → ≥90% correct auto-matches, manual link works
- **M5:** season grid and stats numbers reconcile with AniList
- **M6:** an RSS rule adds the right torrent to qBittorrent exactly once, with the
  configured category and save path
