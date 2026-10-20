mod attachments;
mod backup;
mod commands;
mod db;
mod entries;
mod error;
mod export;
mod letters;
mod library;
mod location;
mod tags;
mod tray;

#[cfg(test)]
mod tests;

use tauri::Manager;

use crate::library::Library;

/// 数据目录相关的全局信息：配置文件位置与默认根目录。
pub struct LocationState {
    pub config_dir: std::path::PathBuf,
    pub default_root: std::path::PathBuf,
}

const LIBRARY_DIR: &str = "Memento Library";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("attachment", |context, request| {
            commands::attachment_response(context, request)
        })
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let config_dir = app.path().app_config_dir()?;
            // 用户选过别的目录就用那个，否则用默认位置。
            let root = location::configured_root(&config_dir)
                .unwrap_or_else(|| app_data_dir.join(LIBRARY_DIR));
            let library = Library::open(root)?;
            app.manage(library);
            app.manage(LocationState {
                config_dir,
                default_root: app_data_dir.join(LIBRARY_DIR),
            });
            tray::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library_info,
            commands::quit_app,
            commands::relocate_library,
            commands::reset_library_location,
            commands::list_entries,
            commands::list_entry_dates,
            commands::list_timeline,
            commands::search_entries,
            commands::list_tags,
            commands::get_entry,
            commands::create_entry,
            commands::save_entry,
            commands::import_attachment,
            commands::delete_attachment,
            commands::list_deleted_entries,
            commands::delete_entry,
            commands::restore_entry,
            commands::purge_entry,
            commands::purge_trash,
            commands::list_entry_tags,
            commands::add_entry_tag,
            commands::remove_entry_tag,
            commands::list_letters,
            commands::get_letter,
            commands::create_letter,
            commands::update_letter,
            commands::delete_letter,
            commands::open_letter,
            commands::save_letter_reply,
            commands::export_entries,
            commands::create_backup,
            commands::inspect_backup,
            commands::restore_backup,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Memento");
}
