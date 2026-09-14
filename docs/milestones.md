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

**6a — detect what AniTrax launched.** *(built 2026-09-13, acceptance owed)*
When you hit *Play* we already know the show, episode and file. Track the
player process we spawned (plus mpv's IPC socket when it's mpv) and, a fixed
delay after playback starts, offer to bump progress — a toast by default,
silent if you opt in. Zero window-scraping. Reuses the push pipeline.

- ✅ `src-tauri/src/playback/` — a `PlaybackTracker` on `AppState` (in-memory,
  not persisted) keyed by (service, media, episode). `play_episode` registers
  a session only when the episode is genuinely `progress + 1` — never a
  rewatch or a batch jump-ahead — via `sync::prepare_watch_session`.
  Threshold is `playback::CONFIRM_AFTER`, a **flat 2 minutes** after the
  session starts (or is first detected) — not tied to the episode's runtime.
  (Originally sized off ~80% of the cached duration; changed to a flat delay
  2026-09-13 per explicit request, trading "waits until the episode's
  actually almost over" for "fires quickly and predictably regardless of
  length" — revisit if that trade stops making sense.) A session that fires
  isn't cleared — it's **re-armed at `RENOTIFY_AFTER` (12 minutes)** instead
  of the initial 2, so an unresolved prompt (dismissed, ignored, or — the
  case that actually surfaced this — M6b re-detecting the same still-playing
  episode every poll) comes back on a slower, less naggy cadence rather than
  immediately. Only a progress edit or `MAX_SESSION_AGE` (6h) actually clears
  a session.
- ✅ A background poll (15s) checks for sessions past their threshold. Silent
  mode calls `sync::bump_from_playback` directly and notifies via an **in-app
  toast** (`playback-bumped` event → `useBackendEvents` → `toast.success`) —
  changed 2026-09-13 from an OS notification after live testing surfaced an
  unwanted Windows notification sound that couldn't be reliably silenced: the
  toast schema's `<audio silent="true">` element (what `notify-rust`/
  `tauri-plugin-notification` already emit by default when no sound is set)
  is honoured inconsistently for an unpackaged app without a properly
  registered AUMID. An in-app toast sidesteps the OS layer entirely — no
  sound, ever. `notify_playback` (the old OS-notification helper) was
  removed; nothing else used it. Confirm mode opens a **dedicated popup window**
  (`show_playback_popup` in `lib.rs`, small/always-on-top/no-decorations,
  bottom-right of the primary monitor, route `playback-prompt.tsx`) instead
  of an in-app toast or OS notification — both of those turned out unreliable
  for this specific case: a same-window toast is invisible behind a
  fullscreen player, and Windows silently drops notification toasts under its
  fullscreen focus-assist rule (confirmed via a real test — nothing even
  reached Action Center). A real app window is exempt from both. Auto-closes
  after 30s if ignored; data reaches the popup via a Tauri
  `initialization_script` (`window.__playbackPopup`), not URL params.
  Its own capability entry (`capabilities/default.json`, `windows: ["main",
  "playback-*"]`) — window-scoped permissions are per-label in Tauri 2, so a
  new window label needs an explicit grant or every command call from it is
  silently rejected. **Always-on-top is load-bearing, not optional** — tried
  dropping it (focused-but-not-topmost) on request and confirmed via testing
  that Windows' anti-focus-stealing protection simply never raises a
  background process's window over a fullscreen foreground app; the popup
  built and the log showed it firing, but nothing appeared. Kept acceptable
  by being short-lived (30s auto-dismiss / instant on either button) rather
  than a persistent floating window. Also learned along the way: no
  `skip_taskbar` — pair that with non-topmost and a window that loses focus
  becomes unreachable (no taskbar entry to reclaim it).
- ✅ Any progress edit (manual, silent-bumped, or otherwise) retires tracked
  sessions at or below the new progress, so a stale timer can't fire after
  you've already moved past it.
- ✅ **Fixed 2026-09-13**: manually typing the final episode into the edit
  dialog didn't auto-complete the show, even though clicking **+1** for that
  same episode did — the two paths had separately duplicated "reaching the
  final episode while Watching completes it" logic, and the edit dialog's
  copy was simply missing. Moved the rule into `sync::edit_entry` itself (the
  one function every progress-changing path — `+1`, the manual dialog, and
  `bump_from_playback` — already funnels through), so it can't drift out of
  sync again; the frontend/`bump_from_playback` copies were deleted.
- ✅ Settings → **Playback detection**: master toggle (default on) + Confirm /
  Silent mode, shown only once the deps merge.
- ✅ "Now watching" strip (root layout, above the page content) lists tracked
  episodes, links to media detail. Polls the in-memory tracker every 15s — no
  network cost, nothing persisted.
- ✅ Never touches AniList beyond the existing debounced progress push.
- ✅ 5 unit tests (threshold math, one-shot tick reporting, stale-session
  pruning, `stop_up_to` range clearing).
- ⏳ **Acceptance owed** — needs a real watch: confirm the toast/notification
  fires around the expected time for an on-disk episode, "Bump progress"
  applies correctly (including auto-completing a finale), and silent mode
  bumps with no prompt.
- Explicitly **not** in this pass: real player-process tracking (exit
  detection) and mpv IPC — this heuristic is wall-clock-since-Play only, so
  walking away for 2 minutes and coming back still counts as "watched." Any
  actual runtime-aware timing, per-show opt-out, and a configurable delay are
  also not yet exposed (`CONFIRM_AFTER` is a fixed constant).

**6b — detect any player.** *(built 2026-09-13, accepted 2026-09-13)* A background
monitor reads the foreground media player's window title, parses it with the
anitomy code from M3, matches to `media_cache` with the M3 matcher. Off by
default — opt-in on top of 6a. Player list + per-player enable in Settings.

- ✅ `active-win-pos-rs` (Win32 / macOS Accessibility / Linux X11 — one
  dependency covers all three targets from the original plan) reads the
  focused window's process + title every 10s
  (`sync::WINDOW_DETECT_POLL_EVERY`), gated on `PLAYBACK_WINDOW_DETECT_KEY`
  (default off) and the per-player `enabled_players` list (defaults to every
  known player once window-detect itself is turned on).
- ✅ `playback::detect::strip_player_chrome` trims a known player's own
  trailing title decoration ("… - VLC media player") before handing the rest
  to `scanner::parse_name` — same anitomy parse M3 uses on filenames.
- ✅ Matches via `matcher::best_match` against `repo::media_match_index` (the
  same cache-only index M3 file-matching uses) — a show has to already be
  cached (synced or searched) to be recognised, same limitation M3 has.
- ✅ Feeds the **exact same** `sync::prepare_watch_session` /
  `PlaybackTracker` pipeline 6a built — same threshold math, same
  confirm/silent handling, same "only ever `progress + 1`" guard. A new
  `touch_or_start` (vs. 6a's `start`) means re-detecting the same still-playing
  episode on every 10s poll doesn't keep resetting its clock.
- ✅ Settings: "Detect any player" toggle (under the base playback-detection
  section) + a monitored-players checklist (VLC, MPC-HC/BE ×2 bitness, mpv,
  PotPlayer ×2, WMP, SMPlayer).
- ✅ 3 unit tests for `strip_player_chrome` (known-player suffix stripping,
  unrecognised players left alone, never over-stripping a title that's pure
  chrome).
- ✅ **Acceptance passed** (2026-09-13) — opened an episode directly in an
  external player (not via AniTrax's Play button): recognised, tracked, and
  prompted the same way a self-launched session does. Took longer than the
  flat 2-minute threshold in practice, which tracks — the 10s window-detect
  poll has to notice the foreground window before `touch_or_start` even
  starts the clock, on top of the 2 minutes itself; confirmed acceptable
  as-is.
- Since this shares 6a's tracker, it inherits the same wall-clock-only
  limitation — no real position, just "the window's been up a while."

**6c — live position via VLC.** *(built 2026-09-13, acceptance owed)* Scoped
down from the original three-protocol plan (mpv IPC + VLC HTTP + MPC-HC web
interface) after weighing it against "this should stay easy for someone else
to pick up" — VLC and mpv's *already-running* instances both need the user to
go configure something outside AniTrax first (an `mpv.conf` line, VLC's own
web-interface toggle + password), which is exactly the kind of setup burden
the rest of the app avoids. The fix: for a **self-launched** session (the
Play button), AniTrax controls how the process is spawned, so it can just
pass VLC's HTTP-interface flags on the command line — a random port +
password generated per launch, nothing to configure in VLC itself, ever.
mpv would need its own transport (a named pipe/socket) and is a reasonable
follow-up, not built here; MPC-HC/BE has no CLI equivalent for its web
interface, so it's not a candidate for this zero-setup approach at all.
6b (any already-running player, including VLC opened outside AniTrax) is
untouched — still the wall-clock heuristic.

- ✅ `playback::live` — `PlayerKind::Vlc` only. `spawn_and_track` launches
  `vlc.exe <file> --extraintf http --http-host 127.0.0.1 --http-port <n>
  --http-password <token>` (random port + token per launch), then a
  dedicated poller asks `http://127.0.0.1:<n>/requests/status.json` (HTTP
  Basic auth, empty username) every 10s for `time`/`length`. Fires the usual
  confirm/silent flow once `time / length >= 0.90` — a fraction of the
  reported length rather than a fixed delay, so it adapts to the episode's
  actual runtime instead of guessing.
- ✅ Fixes 6a/6b's shared blind spot for free: since this reads the player's
  actual position instead of counting wall-clock time, pausing or seeking
  back just means the reported position stops advancing — nothing extra to
  detect. Closing VLC before the episode actually finishes now means no bump
  at all, instead of "walked away and it still counts."
- ✅ `WatchSession` gained `ProgressSource` (`WallClock` vs `Live`) and an
  optional shared `LivePosition` cell the poller updates — `tick()` skips
  `Live` sessions from ever firing itself (only the poller does), but still
  prunes them at `MAX_SESSION_AGE` as a safety net if a poller task dies
  without cleaning up. `PlaybackTracker::remove` added for the "player closed
  without finishing" case; `update_live` feeds the "Now watching" strip a
  real `N%` instead of nothing.
- ✅ `sync::fire_ready` — the confirm-popup-vs-silent-bump decision, factored
  out of the 6a/6b tick loop in `lib.rs` so both it and the new poller agree
  on exactly how a "ready" result is delivered; nothing about the popup/toast
  mechanism itself changed.
- ✅ Settings → **Track exact progress via VLC** (nested under the base
  playback-detection toggle): a native file picker for `vlc.exe`, no other
  configuration. `play_episode` only takes this path when the episode is
  actually going to be tracked (same `progress + 1` gate as always) and the
  configured exe still exists on disk; otherwise it falls back to the OS
  opener + wall-clock heuristic exactly as before, silently.
- ✅ 4 new unit tests (`Live` sessions never fire from `tick()`, still expire
  at `MAX_SESSION_AGE`, `update_live`/`remove` behave) — 61 backend tests
  total, all passing.
- ⏳ **Acceptance owed** — this is the one part of M6 not yet touched by a
  real playback test. VLC's exact `--extraintf`/`--http-*` flag names and the
  `status.json` field names (`time`, `length`) are implemented from
  documented VLC behaviour, not verified against a real launch — expect a
  small flag/parsing fix on first live test, same pattern 6a/6b went through.
- Not built: mpv support (needs its own IPC transport), MPC-HC/BE (no CLI
  hook for its web interface), and anything for 6b's already-running-player
  case — all remain on the wall-clock heuristic.

---

## ✅ M7 — Tray + background running  *(done)*

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
- ✅ **Acceptance accepted** (2026-09-12) — tray icon + menu, X-hides-to-tray,
  Quit-exits, and tray "Sync now" (after two follow-up fixes: no feedback on
  success/failure → desktop notification; "Last synced" not updating after a
  tray/background sync → `entries-updated` wasn't invalidating that query)
  all confirmed on Windows. Settings-persistence and the login-launch /
  "start minimised" path weren't separately re-verified but share the same
  code path as the rest — not re-tested, accepted as-is.
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
