# AniTrax — Milestones

A modern desktop anime tracker in the spirit of Taiga. Cross-platform (Tauri 2 +
React 19), syncs with **AniList**, architected so it never trips AniList's rate
limit. (Kitsu is parked — see the bottom of this file — but the `TrackerService`
trait keeps the door open.)

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

## ✅ M2 — Polish & offline  *(done)*

- ✅ Skeleton loaders, empty states, error toasts everywhere they're missing
- ✅ Keyboard shortcuts: `j`/`k` navigate, `+` bump episode, `/` focus filter
  (plus arrow grid-nav, `1/2/3` + `↑↓` sections, `t` theme picker, `?` help)
- ✅ Offline verified end-to-end — library reads straight from SQLite (zero
  network); an edit made while the endpoint was unreachable stayed `dirty=1`
  with no data loss, the push worker retried on its 90s cadence, and on
  reconnect the row flushed clean with progress preserved through the next
  full sync
- ✅ Next-episode countdowns from AniList `airingSchedule`
- ✅ `META_TTL` (14d) wired into `ensure_media` — stale metadata refetches on
  its own, falling back to the stale copy if the network is down

---

## M3 — Local library scanner  *(implemented — needs a real-folder acceptance pass)*

- ✅ Settings → **Watched folders**: add / remove / enable folders (native
  picker), per-folder file count + last-scan time, "Rescan now"
- ✅ `library/scanner.rs` — `walkdir` + `anitomy` (Rapptz's pure-Rust port, git
  dep): title / episode / season / year / resolution / release group;
  batch-release detection; 20 MB floor to skip samples
- ✅ `library/matcher.rs` — **cache-only** title matching against `media_cache`
  (normalised, season/year aware, ordinal + roman-numeral folding), confidence
  threshold + ambiguity guard. A show that's never been synced or searched
  stays unmatched until linked by hand — no surprise AniList traffic
- ✅ **Folder-aware matching** — reads the show name and season from the
  containing folders (`Sword Art Online/Season 2/…`), so files with only a
  generic name still match; the season number picks the right sequel entry
  ("… S2" → "Sword Art Online II") via AniList's own synonyms
- ✅ **Remembered links** — linking a review-queue group stores a rule keyed on
  the folder title + season, so future episodes of that show/season auto-link
  on the next scan. Rules drop when you unlink the show
- ✅ `library/watcher.rs` — debounced `notify` watcher; filesystem changes
  trigger an incremental rescan, folder set rebuilt when it changes
- ✅ `/library-local` route — matched shows grouped with owned-episode ranges;
  the **To review** queue is grouped by show + season (one "Link" per group,
  not per file) with a link dialog (debounced AniList search — the one place M3
  can touch the network, one request) and a "remember this" toggle
- ✅ Owned-episode badges on library cards + detail page, tinted when the next
  unwatched episode is already on disk
- ✅ 16 scanner/matcher unit tests (folder parsing, season→sequel, rule keys)
  + `episodeRanges` test
- ⏳ **Acceptance:** scan a real anime folder, confirm ≥90% correct auto-matches
  against a synced list, and that the manual link picker fills the gaps

---

## M4 — Season browser + statistics

- `/seasons` — year/season picker, grid of that season's anime, filter by
  format/genre, one-click add to list (single cached `Page` query per season)
- `/stats` — episodes & time watched, score distribution, genre breakdown,
  format split, completion rate, activity over time. Computed in Rust from the
  cache, rendered with Recharts
- Verify season grid matches AniList's own season page; stats reconcile with the
  AniList profile

---

## M5 — RSS auto-download (qBittorrent)

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
- **M3:** scan a real folder → ≥90% correct auto-matches, manual link works
- **M4:** season grid and stats numbers reconcile with AniList
- **M5:** an RSS rule adds the right torrent to qBittorrent exactly once, with the
  configured category and save path

---

## Parked — Kitsu

Dropped from the active roadmap (2026-09-07). AniList-only for the foreseeable
future. The `TrackerService` trait, the `service` column on every cached row, and
the `ServiceKind::Kitsu` stubs all stay — adding Kitsu later is a new impl file,
not a refactor.

- `KitsuService` implementing the same `TrackerService` trait (JSON:API, OAuth)
- **Verify the live base URL first** — `kitsu.io/api/edge` vs the newer
  `kitsu.cloud`; the trait keeps this contained to one file
- Settings: connect Kitsu, choose the primary service, per-service list views
- Domain-model mapping tests with recorded fixtures, mirroring the AniList ones
