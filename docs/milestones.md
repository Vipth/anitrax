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

## ✅ M3 — Local library scanner  *(done)*

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
- ✅ **Popularity tiebreak** — caches AniList `Media.popularity`; when a title
  collides (an obscure short "Onigiri" carries the synonym "Demon Slayer"), the
  household name wins. Synonym matches are also discounted vs romaji/english
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
- ✅ **Play next episode** — a play button (poster hover / list / detail) opens
  `progress + 1` in the OS default player when that file is in the library.
  Progress stays manual (no scrobbling) — bump it with +1 when you're done
- ✅ 18 scanner/matcher unit tests (folder parsing, season→sequel, synonym
  collision, arc-named seasons) + `episodeRanges` test
- ✅ **Acceptance passed** (2026-09-09) — scanned a real 305-file library:
  100 % matched, every distinct auto-match correct on inspection; the
  "Onigiri"/"Demon Slayer" synonym collision was caught and fixed with the
  popularity tiebreak; the review queue + remembered links handled the rest

---

## ✅ M4 — Season browser + statistics  *(done)*

- ✅ `/seasons` — season tabs + year picker + format/genre filters, poster grid,
  add-to-list per card, defaults to the current season. One paced `Page` query
  set per (year, season), cached (`season_state` / `season_media`, media reuse
  `media_cache`); TTL 12h current / 30d past; capped at 3 popularity pages
- ✅ `/stats` — episodes & hours watched, mean score + 1–10 distribution,
  status + format breakdown, top genres, completion rate, completions per month
  for the last 12 months. Computed in Rust (`stats.rs`, 5 unit tests) from the
  cache; charts are hand-rolled SVG/flex (dropped Recharts — its v3 `<Bar>` was
  broken under React 19, and the app already had themed bar primitives)
- ✅ sidebar: Seasons + Stats (nav is now Library / Discover / Seasons / Local
  files / Stats / Settings, hotkeys 1–6)
- ✅ **Acceptance passed** (2026-09-10) — season grid and stat totals eyeballed
  against AniList and line up

---

## ✅ M5 — RSS auto-download (qBittorrent)  *(done)*

- ✅ `/rss` — feed manager (add / enable / remove, last-check time + last error)
  and rule builder: bind a rule to a tracked show, set title-contains, release
  group, min resolution, episode range, save path, qBittorrent category, "add
  paused". Recent-downloads list with a "Clear" button (also clears the dedupe
  guard). Auto-check toggle (default on).
- ✅ **`DownloadClient` trait** (`src-tauri/src/download/`) — `add(AddTorrent)` +
  `test_connection()`. `qbittorrent.rs` is the only impl; the scheduler only
  sees the trait, so Transmission / Deluge is a new file, not a refactor.
- ✅ `qbittorrent.rs` — Web API v2 client: cookie-`SID` login (lazy, 30-min
  reuse, one re-auth + retry on 403), `torrents/add` by URL/magnet with
  savepath + category, `Referer` header for non-localhost, connection test in
  Settings (returns the version string).
- ✅ `rss/feeds.rs` — RSS 2.0 fetch + parse (`rss` crate); link from
  `<enclosure>` then `<link>`, guid from `<guid>` then `<link>`. 4 MB cap.
  Atom not supported.
- ✅ `rss/rules.rs` — **pure** `evaluate(rule, parsed) -> Decision`; the release
  title is run through the M3 anitomy parser for episode / resolution / season /
  group. Filters: title-contains (all words), **title-excludes (any word — kills
  Batch / V2)**, **season (untagged = S1)**, release group, min resolution,
  episode range. 10 unit tests.
- ✅ `rss/scheduler.rs` — `check_all_feeds`: poll enabled feeds, evaluate the
  rules bound to each (feed-specific + all-feed), dedupe via `rss_history`
  (guid spent regardless of rule), hand matches to the client, desktop
  notification. Serialised by a mutex so the 15-min timer and "Check now"
  can't double-add. Runs on a timer in `lib.rs`; off when the master toggle is.
- ✅ Migration `0006_rss` (`rss_feed` / `rss_rule` / `rss_history`); qBittorrent
  config in `app_setting` as one JSON blob, never the torrent password in a
  column beyond that.
- ✅ 12 RSS unit tests (feeds parse + rule decisions); `cargo test` + `vitest`
  green.
- ✅ **Acceptance passed** (2026-09-10) — live qBittorrent + a Nyaa feed: a rule
  adds the matched torrent with the configured category + save path, honours
  "add paused", and a second poll does not re-add an item already in
  `rss_history`.

---

## M6 — Playback detection ("now watching" → auto-progress)

Reversing the original "no auto-detection" call (2026-09-09) — this is what
makes AniTrax a full Taiga replacement. Built in phases, each useful on its own:

**6a — detect what AniTrax launched.** When you hit *Play* we already know the
show, episode and file. Track the player process we spawned (plus mpv's IPC
socket when it's mpv) and, once playback passes a threshold (~80% or the last
few minutes), offer to bump progress — a toast by default, silent if you opt in.
Zero window-scraping. Reuses the push pipeline.

**6b — detect any player.** A background monitor reads the foreground media
player's window title (Windows: `windows` crate; macOS: Accessibility; Linux:
MPRIS/X11), parses it with the anitomy code from M3, matches to `media_cache`
with the M3 matcher. Player list + per-player enable in Settings.

**6c — richer player hooks.** mpv JSON IPC, VLC HTTP interface, MPC-HC/BE web
interface — exact position/duration/path instead of guessing from a title.

- Settings: master toggle, per-show opt-out, "confirm vs auto" mode, watched-%
  threshold, monitored-player list
- A "Now watching" strip in the app while detection is active
- Never touches AniList beyond the existing debounced progress push
- Unit tests: window title → (show, episode); position → watched / not-yet

---

## M7 — Tray + background running  *(built 2026-09-12, acceptance owed)*

Quality-of-life, and a prerequisite for M6 being useful (a detector wants the
app resident). Small, mostly plumbing.

- ✅ **System tray icon** — `TrayIconBuilder` with the `[A]` mark: menu (Open /
  Sync now / Quit), left-click toggles the window (hide if visible, show +
  focus + un-minimise otherwise).
- ✅ **Close-to-tray** — the window starts hidden (`tauri.conf.json`) and a
  `WindowEvent::CloseRequested` handler hides instead of closing when the
  setting is on; checked against an in-memory `AtomicBool` on `AppState` (kept
  in sync by the Settings toggle) so the handler stays synchronous. Settings
  toggle, **default on**. Quit from the tray menu calls `app.exit()` directly,
  bypassing the handler entirely, so it always really quits.
- ✅ **Start on login** — `tauri-plugin-autostart`, registered with a
  `--minimized` launch arg baked in at plugin init; Settings toggle (off by
  default) calls the plugin's `enable()`/`disable()` and the OS registry entry
  is the source of truth for what Settings shows (queried live, not trusted
  from our own DB copy). **Start minimised to tray** sub-option (only shown
  when "start on login" is on) decides — at *runtime*, from the DB setting —
  whether a launch carrying that arg actually stays hidden, so toggling it
  takes effect without re-registering the OS entry.
- ✅ Single-instance handler upgraded to show + un-minimise + focus (was
  focus-only, which didn't un-hide a tray-hidden window).
- ⏳ **Acceptance owed** — needs eyes on a real Windows session: tray icon
  shows and its menu works, X hides to tray and the app keeps running, Quit
  actually exits, Settings toggles persist across a relaunch, and (if you're
  willing to log out/in) a login launch with "start minimised" on comes up
  hidden while off pops the window normally.
- Verify on Windows first (macOS/Linux tray behaviour differs — menubar item,
  AppIndicator).

---

## M8 — Auto-update

So a build ships itself instead of the user re-downloading an installer. Also
forces the cross-platform build story to exist.

- **`tauri-plugin-updater` + `tauri-plugin-process`** — checks a `latest.json`
  manifest on GitHub Releases once at launch, plus a manual "Check for updates"
  in Settings. One check, not a background poller — same restraint as the AniList
  gateway.
- **Signed updates** — Ed25519 keypair via `tauri signer generate`; public key in
  `tauri.conf.json`, private key + password as CI secrets. An unsigned or
  tampered bundle is refused.
- **GitHub Actions release workflow** — `tauri-apps/tauri-action` on a version
  tag builds Windows / macOS / Linux bundles and uploads them plus the generated
  `latest.json` to the release. This is the cross-platform build pipeline we've
  been deferring.
- **Frontend** — unobtrusive "v0.2.0 is ready" prompt with release notes,
  download with a progress bar, install + relaunch on confirm. Never forced,
  dismissible, "skip this version".
- **Settings** — current version, last-checked time, "check automatically on
  launch" toggle (default on), stable channel only for now.

---

## M9 — Your schedule (airing calendar)

A month-at-a-glance calendar of every airing show in your library: which
episode drops on which day, and at what time, in your local timezone.

- **`/schedule` route** — monthly calendar grid (prev / next month, "Today"
  jump, today's cell highlighted, leading/trailing days from adjacent months
  dimmed). Each day cell lists that day's releases sorted by air time:
  small poster, show title, **episode number + episode title**, local time
  (`Intl.DateTimeFormat`, honours 12h/24h locale). Cells that overflow show
  "+N more", which opens a day popover with the full list. Clicking an entry
  opens media detail.
- **Which shows** — library entries whose media is `RELEASING` or
  `NOT_YET_RELEASED`. Status filter chips: Watching (default on), Planning,
  Paused. Shows not in the library never appear.
- **Data: `airingSchedules`, not `nextAiringEpisode`** — today the cache only
  holds the single next episode per show, which can't fill a month. New paged
  query: `Page { airingSchedules(mediaId_in: [...], airingAt_greater,
  airingAt_lesser) { mediaId episode airingAt } }` for the visible grid range
  (month + padding weeks). Goes through the `AniListGateway` like everything
  else; ~30 weekly shows is ~130 rows ≈ 3 pages of 50.
- **Cache** — migration `0008_airing_schedule` (`airing_schedule` keyed on
  service + media id + episode, `schedule_state` per month range). TTL 12h for
  the current/future months, 30d for past months; a list sync that adds or
  drops an airing show marks the affected months stale. Offline = render from
  the cache, same as the library.
- **Episode titles** — AniList's schedule has no per-episode name. Pull
  `streamingEpisodes { title }` in the same request where available (mostly
  licensed shows, and usually only once an episode is out); fall back to
  "Episode N". Don't scrape a second source for this.
- **Status tints** — next unwatched episode (`progress + 1`) highlighted;
  episodes you're already behind on marked; owned-on-disk badge from M3 on past
  days; a subtle "premiere" / "finale" tag when `episode == 1` /
  `episode == episodes`.
- **Settings** — week starts on Monday / Sunday (default from locale).
- **Sidebar** — Schedule entry after Seasons (`CalendarDays` icon); nav becomes
  Library / Discover / Seasons / Schedule / Local files / RSS / Stats /
  Settings, hotkeys 1–8.
- **Tests** — month grid generation (leading/trailing days, week start, 6-row
  months); bucketing `airingAt` into local days across midnight + DST
  boundaries; paged-response merge + dedupe; title fallback.

---

## Backlog / polish

- ~~**Sidebar logo**~~ — done (2026-09-10). `Wordmark.tsx`: `[AniTrax]` —
  two-tone extrabold name in primary brackets, no icon glyph so it doesn't
  clash with the nav lucide icons.

---

## Known bugs

- ~~**Season viewer loses its place on back-nav**~~ — fixed 2026-09-10. The
  selected year / season / format / genre now live in the `useUi` store
  (`seasonSel` / `seasonFormat` / `seasonGenre`) instead of `React.useState`, so
  the `/seasons` route keeps its place when you open a show and come back — same
  pattern already used for the library filter and discover search.

---

## Verification (per milestone)

- **M1:** connect AniList, list renders with correct per-status counts, +1 shows
  on anilist.co within seconds, offline relaunch still renders, 10-min hammer
  test stays under 45 req/min with zero 429s
- **M3:** scan a real folder → ≥90% correct auto-matches, manual link works
- **M4:** season grid matches AniList's own season page; stat totals reconcile
  with the AniList profile
- **M5:** an RSS rule adds the right torrent to qBittorrent exactly once, with the
  configured category and save path
- **M6:** play an episode → progress offer fires at the right point; a title from
  an external player resolves to the correct show + episode; no false positives
  on non-anime windows
- **M7:** X hides to tray and the app keeps running; tray menu opens / syncs /
  quits; with autostart on, a login launch comes up minimised; a second launch
  focuses the running window
- **M8:** publish a dummy higher-versioned release → the app detects it at
  launch, shows the prompt with notes, downloads, installs, and relaunches on the
  new version; an unsigned or tampered bundle is rejected
- **M9:** the current month's calendar matches the airing times on each show's
  AniList page (converted to local time); every airing Watching show appears
  on the right days; flipping months stays within the gateway budget and a
  revisit inside the TTL makes zero requests; offline relaunch still renders

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
