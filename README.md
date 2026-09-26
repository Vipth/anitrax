# AniTrax

I'm a longtime [Taiga](https://taiga.moe) user, and Taiga is the whole inspiration for this project: a real desktop app instead of another browser tab, synced with AniList. AniTrax is my own take on that same idea, built around one goal from day one: staying well clear of AniList's own rate limit, no matter how heavily I use it. Add a UI built on today's tools, and that's AniTrax.

It's built with Tauri 2 and React 19, so it's a real native app on Windows, macOS, and Linux, not a wrapped website.

Curious how it got here? `docs/milestones.md` has the whole build history, built out over about two weeks.

## What it does

- **Library**: status tabs, grid or list view, sort and filter, inline **+1** on an episode, a full edit dialog for score, notes, and dates, keyboard shortcuts if you'd rather not touch the mouse, right click context menus
- **Local library scanner**: point it at the folder where your episodes live and it matches files to your list by parsing filenames, entirely offline. Anything it can't confidently place lands in a review queue instead of guessing
- **Schedule**: a month calendar of when each show you're watching airs next, in your own timezone
- **Playback detection**: notices when you're watching, through the app's own launcher, a watched media player window, or (for VLC/mpv) actual playback position, and offers to bump your progress
- **RSS auto download**, wired into qBittorrent's Web API
- **Seasons and stats**: browse what's airing, or your own numbers: hours watched, score distribution, how many you've dropped
- A **system tray**, **auto updates** that check once and get out of your way, and **ten themes** (light/dark/system, plus Jade, Nord, Kanagawa, and the four Catppuccin flavours)
- **AniList sign-in** in one click: approve in your browser, paste the token it gives you, no API client to register yourself

Everything's cached locally, so the app works offline and just catches up whenever you're back online.

## Why the rate limit stuff matters so much

This is the part I actually care about, so it gets its own section instead of a bullet.

Every single request to AniList, sync, search, an episode bump, anything, funnels through one paced queue (`src-tauri/src/tracker/anilist/gateway.rs`) that caps itself at 45 requests a minute. That's half of what AniList actually allows. Not because I don't trust their limit, but because I'd rather leave headroom than find out the hard way that their hidden burst limiter exists too.

On top of that, the UI never talks to the network directly. It reads from a local SQLite cache, and that cache is the actual source of truth for what's on screen. The network gets touched on an explicit sync, a stale launch, or a slow background timer, never just because you scrolled. Rapid `+1` clicks on an episode collapse into a single debounced write instead of one request per click. And if you're curious how close to the edge you're running, there's a live request budget meter sitting in the sidebar and Settings.

## Running it yourself

```bash
npm install
npm run tauri dev
```

You'll need Node 20+, the Rust toolchain, and on Windows, the MSVC C++ build tools (WebView2 already ships with Windows 11).

### Connecting your AniList account

Settings, then **Sign in with AniList**. Approve in your browser, copy the access token it shows you, paste it back in. That's the whole flow.

## Tests

```bash
npm run test          # Vitest: UI helpers, calendar grid, hooks
npm run typecheck
cd src-tauri && cargo test    # gateway pacing, sync policy, scanner/matcher, RSS rules
```
