# AniTrax

A modern desktop anime tracker in the spirit of [Taiga](https://taiga.moe), rebuilt
with a current UI. Cross-platform (Tauri 2 + React 19), syncs your list with
**AniList**, and is architected from the ground up to **never trip AniList's rate
limit**.

> Status: **Milestones 1–2 done, M3 in progress** — polished AniList list
> manager with two-way sync, offline cache, keyboard shortcuts, and a local
> library scanner (watched folders, filename parsing, cache-only matching). A
> season browser, statistics and RSS auto-download are next. Kitsu is parked
> (AniList-only for now); see `docs/milestones.md`.

## Features

- Library with status tabs (Watching / Rewatching / Planning / On Hold /
  Completed / Dropped), grid & list layouts, sort and filter
- Inline **+1 episode**, quick score, full edit sheet — all optimistic
- Media detail pages, debounced Discover search, next-episode countdowns
- AniList sign-in (OAuth implicit grant; tokens stored in the OS keychain)
- Works fully offline from a local cache; edits queue and sync on reconnect
- **Local library scanner** — point it at your episode folders; it parses
  filenames, matches them to your list offline, and flags which episodes are on
  disk. Files it can't place go to a review queue with a manual link picker

## Rate-limit resilience

This is a first-class design goal, not an afterthought:

- **One gateway.** Every call to `graphql.anilist.co` goes through a single paced
  queue (`src-tauri/src/tracker/anilist/gateway.rs`) capped at **45 req/min** —
  half of AniList's 90 ceiling — with burst smoothing, header-aware slow-down,
  and full-queue parking + retry on HTTP 429.
- **Cache-first.** SQLite (`media_cache` / `list_entry`) is the UI's source of
  truth. Screens render from it instantly; the network is touched only on
  explicit sync, a stale launch, or a slow 30-minute background timer.
- **Batched & coalesced.** Media are fetched with aliased GraphQL batches; rapid
  `+1` clicks collapse into one debounced `SaveMediaListEntry` mutation.
- A live **request-budget meter** is visible in the sidebar and Settings.

## Architecture

```
src-tauri/src/
  tracker/anilist/gateway.rs   paced single request queue (the core of the rate-limit story)
  tracker/{mod,model}.rs       TrackerService trait + service-agnostic domain model
  tracker/anilist/*            GraphQL queries, JSON->domain mapping, TrackerService impl
  db/{mod,repo}.rs             SQLite pool + typed repository (cache reconciliation lives here)
  auth.rs                      OAuth redirect parsing + OS keychain
  sync.rs                      "when do we hit the network" policy
  commands.rs                  thin #[tauri::command] wrappers
  lib.rs                       plugin wiring, deep-link handler, push + background-sync workers
src/
  routes/                      TanStack Router: library (/), media/$mediaId, discover, settings
  lib/{ipc,query,hooks,library,format}.ts
  components/{media,layout,ui}/
```

## Develop

```bash
npm install
npm run tauri dev
```

Prerequisites: Node 20+, the Rust toolchain, and (Windows) the MSVC C++ build
tools + WebView2 (bundled on Windows 11).

### Connecting AniList

1. Create a client at <https://anilist.co/settings/developer> with redirect URL
   `anitrax://oauth/anilist`.
2. In the app: **Settings → paste the client ID → Sign in with AniList**.
   (If the deep-link redirect doesn't fire, the developer page also lets you mint
   an access token directly — paste it under "Sign-in didn't redirect back?".)

## Test

```bash
npm run test          # Vitest: score/sort/filter helpers
npm run typecheck
cd src-tauri && cargo test    # gateway pacing, OAuth parsing, mapping
```
