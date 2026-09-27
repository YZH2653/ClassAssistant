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

use domain::settings::CloseBehavior;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 单实例：再次启动时唤起已有窗口，而不是新开一个
        .plugin(tauri_plugin_single_instance::init(|handle, _args, _cwd| {
            app::show_window(handle);
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app::setup(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let behavior = window
                    .state::<app::AppState>()
                    .config_store
                    .load()
                    .close_behavior;
                if behavior == CloseBehavior::Tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
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
