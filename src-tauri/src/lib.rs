mod auth;
mod commands;
mod db;
mod download;
mod error;
mod library;
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
        .plugin(tauri_plugin_deep_link::init())
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
            commands::set_anilist_client_id,
            commands::set_sync_on_startup,
            commands::anilist_login_url,
            commands::anilist_complete_login,
            commands::anilist_connect_token,
            commands::list_accounts,
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

            #[cfg(desktop)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                // Best-effort: makes the custom scheme work in dev on Windows/Linux.
                let _ = app.deep_link().register_all();

                let dl_handle = handle.clone();
                let dl_state = state.clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        let url = url.to_string();
                        if !url.starts_with("anitrax://oauth/anilist") {
                            continue;
                        }
                        let h = dl_handle.clone();
                        let s = dl_state.clone();
                        match auth::parse_anilist_redirect(&url) {
                            Some(token) => {
                                tauri::async_runtime::spawn(async move {
                                    match sync::connect_anilist(&s, &token).await {
                                        Ok(_) => {
                                            let _ = h.emit("auth-changed", "anilist");
                                            let _ = h.emit("entries-updated", ());
                                        }
                                        Err(e) => {
                                            let _ = h.emit("auth-error", &e);
                                        }
                                    }
                                });
                            }
                            None => {
                                let _ = h.emit(
                                    "auth-error",
                                    &error::AppError::other(
                                        "The AniList redirect didn't contain an access token.",
                                    ),
                                );
                            }
                        }
                    }
                });
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
