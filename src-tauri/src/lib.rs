mod attachments;
mod commands;
mod db;
mod entries;
mod error;
mod library;
mod tags;

#[cfg(test)]
mod tests;

use tauri::Manager;

use crate::library::Library;

const LIBRARY_DIR: &str = "Memento Library";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_uri_scheme_protocol("attachment", |context, request| {
            commands::attachment_response(context, request)
        })
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let library = Library::open(app_data_dir.join(LIBRARY_DIR))?;
            app.manage(library);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library_info,
            commands::list_entries,
            commands::list_entry_dates,
            commands::list_timeline,
            commands::get_entry,
            commands::create_entry,
            commands::save_entry,
            commands::import_attachment,
            commands::delete_attachment,
            commands::list_deleted_entries,
            commands::delete_entry,
            commands::restore_entry,
            commands::list_entry_tags,
            commands::add_entry_tag,
            commands::remove_entry_tag,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Memento");
}
