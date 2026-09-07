mod auth;
mod commands;
mod db;
mod error;
mod state;
mod sync;
mod tracker;

use tauri::{Emitter, Manager};

use crate::state::AppState;
use crate::tracker::model::ServiceKind;

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
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_deep_link::init())
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
            commands::sync_now,
            commands::get_media,
            commands::edit_entry,
            commands::remove_entry,
            commands::search_anime,
            commands::budget_snapshot,
            commands::last_sync,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let state = tauri::async_runtime::block_on(AppState::init(&handle))
                .expect("failed to initialise app state");
            app.manage(state.clone());

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
                    if let Err(e) = sync::sync_on_launch_if_stale(&s).await {
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
