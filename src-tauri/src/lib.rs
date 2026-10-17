mod attachments;
mod commands;
mod db;
mod entries;
mod error;
mod library;

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
            commands::get_entry,
            commands::create_entry,
            commands::save_entry,
            commands::import_attachment,
            commands::delete_attachment,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Memento");
}
