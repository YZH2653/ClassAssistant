// 未接入的实现暂不告警（cpal / mimo 占位）
#![allow(dead_code)]

mod app;
mod asr;
mod audio;
mod domain;
mod error;
mod ipc;
mod session;
mod storage;
mod summarizer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::commands::get_app_info,
            ipc::commands::start_class,
            ipc::commands::end_class,
            ipc::commands::get_current_session,
            ipc::commands::reset_session,
            ipc::commands::reveal_data_dir,
            ipc::commands::get_settings,
            ipc::commands::save_settings,
            ipc::commands::test_provider_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
