mod credentials;
mod generator;
mod github;
mod repos;

pub use repos::DbConn;

use repos::init_db;
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

/// Initialise the Feature Requests plugin.
///
/// Register this in your Tauri app with:
/// ```rust
/// tauri::Builder::default()
///     .plugin(tauri_plugin_feature_requests::init())
/// ```
///
/// All commands (`generate_feature_request`, `create_github_issues`,
/// `save_credential`, `get_credential`, `list_saved_repos`,
/// `upsert_saved_repo`, `delete_saved_repo`) become available to the
/// frontend via `invoke("plugin:feature-requests|<command>")`.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("feature-requests")
        .invoke_handler(tauri::generate_handler![
            // OS keychain — GitHub PAT + Anthropic API key
            credentials::save_credential,
            credentials::get_credential,
            // SQLite saved repository list
            repos::list_saved_repos,
            repos::upsert_saved_repo,
            repos::delete_saved_repo,
            // Claude AI generation + GitHub publishing
            generator::generate_feature_request,
            github::create_github_issues,
        ])
        .setup(|app, _api| {
            // Resolve the OS app data directory for this app identifier.
            // Falls back to an in-memory DB if the path cannot be resolved.
            let db_path = app
                .path()
                .app_data_dir()
                .map(|p| p.join("feature_requests.db"))
                .unwrap_or_else(|_| std::path::PathBuf::from("feature_requests.db"));

            if let Some(parent) = db_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let conn = Connection::open(&db_path).unwrap_or_else(|_| {
                Connection::open_in_memory().expect("Failed to open in-memory SQLite")
            });

            init_db(&conn).expect("Failed to initialise feature_requests DB schema");

            app.manage(DbConn(Mutex::new(conn)));
            Ok(())
        })
        .build()
}
