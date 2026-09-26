mod auth;
mod commands;
mod db;
mod download;
mod error;
mod library;
mod playback;
mod rss;
mod state;
mod stats;
mod sync;
mod tracker;

use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;
use crate::tracker::model::ServiceKind;

/// Show (un-hiding / un-minimising as needed) and focus the main window.
fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Left-click on the tray icon: hide if visible, show+focus otherwise.
fn toggle_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            drop(w);
            show_main_window(app);
        }
    }
}

/// Open a small popup window for the playback confirm prompt — a real app
/// window rather than an OS notification, since Windows silently suppresses
/// notification toasts while a fullscreen app has focus (exactly when this
/// fires — you're watching something). Always-on-top, because Windows'
/// anti-focus-stealing protection means a plain focused (non-topmost) window
/// from a background process is simply not raised over a fullscreen
/// foreground app — confirmed by testing, not theoretical. It isn't
/// permanently intrusive though: the window itself is short-lived (the
/// frontend auto-closes it in 30s, or instantly on either button), so
/// "always on top" only ever applies for that brief window, not forever.
pub(crate) fn show_playback_popup(
    app: &AppHandle,
    service: &str,
    media_id: i64,
    episode: i64,
    title: &str,
    episodes_total: Option<i64>,
) {
    let label = format!("playback-{service}-{media_id}-{episode}");
    if app.get_webview_window(&label).is_some() {
        return; // already showing this exact prompt
    }

    let payload = serde_json::json!({
        "service": service,
        "mediaId": media_id,
        "episode": episode,
        "title": title,
        "episodesTotal": episodes_total,
    });
    let init_script = format!("window.__playbackPopup = {payload};");

    const WIN_W: f64 = 360.0;
    const WIN_H: f64 = 160.0;
    let (x, y) = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let pos = m.position();
            let size = m.size();
            let scale = m.scale_factor();
            let right = (pos.x as f64 + size.width as f64) / scale;
            let bottom = (pos.y as f64 + size.height as f64) / scale;
            (right - WIN_W - 24.0, bottom - WIN_H - 24.0)
        })
        .unwrap_or((900.0, 700.0));

    let result = tauri::WebviewWindowBuilder::new(
        app,
        &label,
        tauri::WebviewUrl::App("/playback-prompt".into()),
    )
    .title("Finished an episode?")
    .inner_size(WIN_W, WIN_H)
    .position(x, y)
    .decorations(false)
    .resizable(false)
    .focused(true)
    .always_on_top(true)
    .initialization_script(&init_script)
    .build();

    if let Err(e) = result {
        tracing::warn!(?e, "couldn't open playback popup window");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "anitrax_lib=info,warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // Always registers the login-launch command with `--minimized`; whether
        // that actually starts hidden is decided at runtime by the
        // `start_minimized` setting (see the show/hide logic below) — so
        // toggling that setting takes effect without touching the OS entry.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_sync_on_startup,
            commands::anilist_login_url,
            commands::anilist_connect_token,
            commands::disconnect_account,
            commands::get_library,
            commands::get_stats,
            commands::sync_now,
            commands::get_media,
            commands::edit_entry,
            commands::remove_entry,
            commands::search_anime,
            commands::current_season,
            commands::get_season,
            commands::budget_snapshot,
            commands::last_sync,
            commands::library_folders,
            commands::add_library_folder,
            commands::remove_library_folder,
            commands::set_library_folder_enabled,
            commands::scan_library,
            commands::library_files,
            commands::library_owned,
            commands::open_media_folder,
            commands::play_episode,
            commands::link_library_files,
            commands::unlink_library_file,
            commands::library_link_rules,
            commands::delete_link_rule,
            commands::rss_feeds,
            commands::add_rss_feed,
            commands::remove_rss_feed,
            commands::set_rss_feed_enabled,
            commands::rss_rules,
            commands::save_rss_rule,
            commands::delete_rss_rule,
            commands::set_rss_rule_enabled,
            commands::rss_history,
            commands::clear_rss_history,
            commands::check_feeds_now,
            commands::get_qb_config,
            commands::set_qb_config,
            commands::test_qb_connection,
            commands::rss_poll_enabled,
            commands::set_rss_poll_enabled,
            commands::set_close_to_tray,
            commands::set_start_on_login,
            commands::set_start_minimized,
            commands::set_playback_enabled,
            commands::set_playback_mode,
            commands::now_watching,
            commands::known_players,
            commands::set_playback_window_detect,
            commands::set_monitored_players,
            commands::set_player_integration,
            commands::set_auto_update_check,
            commands::set_skipped_update_version,
            commands::mark_update_checked,
            commands::get_schedule,
            commands::set_week_starts_monday,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let state = tauri::async_runtime::block_on(AppState::init(&handle))
                .expect("failed to initialise app state");
            app.manage(state.clone());

            // System tray: Open / Sync now / Quit, left-click toggles the window.
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

                let open_i = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
                let sync_i = MenuItem::with_id(app, "sync", "Sync now", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let tray_menu = Menu::with_items(app, &[&open_i, &sync_i, &quit_i])?;

                let tray_state = state.clone();
                TrayIconBuilder::new()
                    .icon(tauri::include_image!("icons/32x32.png"))
                    .tooltip("AniTrax")
                    .menu(&tray_menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(move |app, event| match event.id.as_ref() {
                        "open" => show_main_window(app),
                        "sync" => {
                            let h = app.clone();
                            let s = tray_state.clone();
                            tauri::async_runtime::spawn(async move {
                                use tauri_plugin_notification::NotificationExt;
                                match sync::full_sync(&s, None).await {
                                    Ok(report) => {
                                        let _ = h.emit("entries-updated", ());
                                        let _ = h
                                            .notification()
                                            .builder()
                                            .title("AniTrax")
                                            .body(format!("Synced — {} entries", report.entries))
                                            .show();
                                    }
                                    Err(e) => {
                                        tracing::warn!(?e, "tray sync error");
                                        let _ = h
                                            .notification()
                                            .builder()
                                            .title("Sync failed")
                                            .body(e.to_string())
                                            .show();
                                    }
                                }
                            });
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            toggle_main_window(tray.app_handle());
                        }
                    })
                    .build(app)?;
            }

            // Close-to-tray: hide instead of quitting when the setting is on
            // (checked in memory — `AppState.close_to_tray` — so this stays
            // synchronous). "Quit" from the tray menu bypasses this via `exit()`.
            if let Some(window) = app.get_webview_window("main") {
                let close_state = state.clone();
                let win = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if close_state.is_close_to_tray() {
                            api.prevent_close();
                            let _ = win.hide();
                        }
                    }
                });
            }

            // The window starts hidden (see tauri.conf.json); show it now unless
            // this is a login-launch (`--minimized`, set by the autostart plugin
            // above) AND the user has asked to start minimised to tray.
            {
                let launched_minimized = std::env::args().any(|a| a == "--minimized");
                let start_minimized = tauri::async_runtime::block_on(db::repo::get_bool_setting(
                    &state.db,
                    sync::START_MINIMIZED_KEY,
                    false,
                ))
                .unwrap_or(false);
                if !(launched_minimized && start_minimized) {
                    show_main_window(&handle);
                }
            }

            // Background push worker: debounced flush of dirty rows, then retry
            // on a slow cadence until the queue actually clears (covers offline
            // / rate-limited / API-down edits made earlier).
            {
                let h = handle.clone();
                let s = state.clone();
                tauri::async_runtime::spawn(async move {
                    // Flush anything left dirty from a previous session.
                    if sync::has_pending_pushes(&s, ServiceKind::AniList).await {
                        s.push.nudge();
                    }
                    loop {
                        s.push.wait().await;
                        tokio::time::sleep(sync::PUSH_DEBOUNCE).await;
                        // Retry a bounded number of times, then wait for the next
                        // edit or full sync rather than hammering forever.
                        for attempt in 0..8u32 {
                            if attempt > 0 {
                                tokio::time::sleep(sync::PUSH_RETRY_EVERY).await;
                            }
                            if let Err(e) = sync::push_dirty(&s, ServiceKind::AniList).await {
                                tracing::warn!(?e, "push worker error");
                            }
                            let _ = h.emit("entries-updated", ());
                            if !sync::has_pending_pushes(&s, ServiceKind::AniList).await {
                                break;
                            }
                        }
                    }
                });
            }

            // Background sync: one stale-only sync on launch, then a slow timer.
            {
                let h = handle.clone();
                let s = state.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = sync::sync_on_launch(&s).await {
                        tracing::warn!(?e, "launch sync error");
                    }
                    let _ = h.emit("entries-updated", ());

                    loop {
                        tokio::time::sleep(sync::BACKGROUND_SYNC_EVERY).await;
                        let focused = h
                            .get_webview_window("main")
                            .and_then(|w| w.is_focused().ok())
                            .unwrap_or(false);
                        if !focused {
                            continue;
                        }
                        if auth::load_token(ServiceKind::AniList).ok().flatten().is_none() {
                            continue;
                        }
                        match sync::full_sync(&s, Some("anilist")).await {
                            Ok(_) => {
                                let _ = h.emit("entries-updated", ());
                            }
                            Err(e) => tracing::warn!(?e, "background sync error"),
                        }
                    }
                });
            }

            // Local library: scan on launch, then keep it live with a
            // filesystem watcher that triggers incremental rescans.
            {
                let h = handle.clone();
                let s = state.clone();
                tauri::async_runtime::spawn(async move {
                    s.watcher.refresh(&s).await;
                    match sync::scan_library(&s).await {
                        Ok(r) if r.files_seen > 0 || r.files_removed > 0 => {
                            let _ = h.emit("library-updated", ());
                        }
                        Ok(_) => {}
                        Err(e) => tracing::warn!(?e, "launch library scan error"),
                    }

                    let Some(mut rx) = s.watcher.take_receiver() else {
                        return;
                    };
                    while let Some(roots) = rx.recv().await {
                        match sync::rescan_paths(&s, &roots).await {
                            Ok(_) => {
                                let _ = h.emit("library-updated", ());
                            }
                            Err(e) => tracing::warn!(?e, "watched-folder rescan error"),
                        }
                    }
                });
            }

            // RSS auto-download: poll enabled feeds on a slow timer, add matches
            // to the download client. Never touches AniList. Off when the user
            // disables the master switch in Settings.
            {
                let h = handle.clone();
                let s = state.clone();
                tauri::async_runtime::spawn(async move {
                    // A short delay so a launch check doesn't race the first sync.
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                    loop {
                        let on = db::repo::get_bool_setting(
                            &s.db,
                            rss::scheduler::RSS_POLL_KEY,
                            true,
                        )
                        .await
                        .unwrap_or(true);
                        if on {
                            match rss::scheduler::check_all_feeds(&s, &h).await {
                                Ok(r) if r.added > 0 => {
                                    let _ = h.emit("rss-updated", ());
                                }
                                Ok(_) => {}
                                Err(e) => tracing::warn!(?e, "rss poll error"),
                            }
                        }
                        tokio::time::sleep(rss::scheduler::POLL_EVERY).await;
                    }
                });
            }

            // M6a — playback detection: check for episodes that crossed their
            // "probably watched" threshold, then confirm-toast or silently bump.
            {
                let h = handle.clone();
                let s = state.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(sync::PLAYBACK_POLL_EVERY).await;
                        for item in s.playback.tick() {
                            sync::fire_ready(
                                &h,
                                &s,
                                &item.service,
                                item.media_id,
                                item.episode,
                                &item.title,
                                item.episodes_total,
                            )
                            .await;
                        }
                    }
                });
            }

            // M6b — foreground-window detection: notice a show playing in any
            // monitored player, not just ones AniTrax itself launched. Off by
            // default (`PLAYBACK_WINDOW_DETECT_KEY`); feeds the same tracker
            // 6a uses, so the threshold/confirm/silent handling above applies
            // unchanged regardless of which phase found the episode.
            {
                let s = state.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(sync::WINDOW_DETECT_POLL_EVERY).await;
                        match sync::detect_foreground_playback(&s).await {
                            Ok(Some(session)) => s.playback.touch_or_start(session),
                            Ok(None) => {}
                            Err(e) => tracing::warn!(?e, "foreground playback detection error"),
                        }
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
