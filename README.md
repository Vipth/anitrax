# AniTrax

A modern desktop anime tracker in the spirit of [Taiga](https://taiga.moe), rebuilt
with a current UI. Cross-platform (Tauri 2 + React 19), syncs your list with
**AniList**, and is architected from the ground up to **never trip AniList's rate
limit**.

See `docs/milestones.md` for the full build history and what's planned next.

## Features

- **Library** — status tabs (Watching / Rewatching / Planning / On Hold /
  Completed / Dropped), grid & list layouts, sort and filter, inline **+1
  episode** and quick score, a full edit dialog, keyboard shortcuts, and a
  right-click context menu on entries
- **Media detail** pages, debounced **Discover** search, next-episode
  countdowns
- **Schedule** — a month-at-a-glance calendar of when each tracked show's
  next episode airs, in your local time, with status filters and
  owned-on-disk badges
- **Seasons** browser — every show airing in a given season, most popular
  first, with add-to-list from the grid
- **Statistics** — episodes/hours watched, score distribution, genre
  breakdown, completion rate, completions per month
- **Local library scanner** — point it at your episode folders; it parses
  filenames, matches them to your list entirely offline, and flags which
  episodes are on disk. Files it can't place go to a review queue with a
  manual link picker
- **Playback detection** — notices when you're watching (via the app's own
  Play button, any foreground media player window, or live position through
  a self-launched VLC/mpv) and offers to bump your progress automatically
- **RSS auto-download** — feed + rule manager that hands matched torrents to
  qBittorrent's Web API
- **System tray + autostart** — close-to-tray, start on login, start
  minimised
- **Auto-update** — signed releases, checked once per launch, with an
  unobtrusive in-app prompt
- **AniList sign-in** — just click "Sign in with AniList" and paste the
  access token it shows you; tokens live in the OS keychain, never on disk
- **Works fully offline** from a local cache; edits queue and sync on
  reconnect
- Ten themes — light/dark/system plus Jade, Nord, Kanagawa, and the four
  Catppuccin flavours

## Rate-limit resilience

This is a first-class design goal, not an afterthought:

- **One gateway.** Every call to `graphql.anilist.co` goes through a single paced
  queue (`src-tauri/src/tracker/anilist/gateway.rs`) capped at **45 req/min** —
  half of AniList's 90 ceiling — with burst smoothing, header-aware slow-down,
  and full-queue parking + retry on HTTP 429.
- **Cache-first.** SQLite (`media_cache` / `list_entry`) is the UI's source of
  truth. Screens render from it instantly; the network is touched only on
  explicit sync, a stale launch, or a slow background timer.
- **Batched & coalesced.** Media are fetched with aliased GraphQL batches; rapid
  `+1` clicks collapse into one debounced `SaveMediaListEntry` mutation.
- A live **request-budget meter** is visible in the sidebar and Settings.

## Develop

```bash
npm install
npm run tauri dev
```

Prerequisites: Node 20+, the Rust toolchain, and (Windows) the MSVC C++ build
tools + WebView2 (bundled on Windows 11).

### Connecting AniList

In the app: **Settings → Sign in with AniList**. Approve in your browser,
then paste the access token the page shows you. That's it — no API client to
create, no configuration on AniList's side.

## Test

```bash
npm run test          # Vitest: UI helpers, calendar grid, hooks
npm run typecheck
cd src-tauri && cargo test    # gateway pacing, sync policy, scanner/matcher, RSS rules
```

## Release

Push a `vX.Y.Z` tag to build and sign Windows/macOS/Linux bundles via GitHub
Actions and open a draft release — see `docs/releasing.md`.
