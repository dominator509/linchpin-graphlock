#[tauri::command]
fn get_system_health() -> Result<application::SystemHealth, String> {
    Ok(application::check_system_health())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_system_health])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
