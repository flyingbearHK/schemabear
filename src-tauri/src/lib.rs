mod commands;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_version,
            load_sample,
            parse_mermaid,
            parse_dbml,
            to_mermaid,
            to_dbml,
            layout_diagram,
            validate_diagram,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ER Diagram");
}
