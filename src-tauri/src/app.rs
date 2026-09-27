// 应用装配：数据目录、Provider 工厂、会话编排启动
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tauri::{App, Manager};

use crate::asr::mimo::MimoAsrProvider;
use crate::asr::{AsrProvider, MockAsrProvider};
use crate::audio::cpal_capture::CpalCapture;
use crate::audio::{AudioCapture, MockAudioCapture};
use crate::domain::settings::{AppSettings, ProviderKind};
use crate::session::manager::{Providers, SessionManager, SharedProviders};
use crate::storage::config_store::ConfigStore;
use crate::storage::fs_store::FsSessionStore;
use crate::summarizer::mimo_flash::MimoFlashSummarizer;
use crate::summarizer::{MockSummarizer, SummarizerProvider};

// 全局应用状态
pub struct AppState {
    pub manager: SessionManager,
    pub config_store: ConfigStore,
    pub providers: SharedProviders,
    pub data_dir: PathBuf,
    pub app_version: String,
}

// 按配置装配 Provider（目前仅支持小米 MiMo 与 mock）
pub fn build_providers(settings: &AppSettings) -> Providers {
    let capture: Arc<dyn AudioCapture> = match settings.asr.provider {
        ProviderKind::Mock => Arc::new(MockAudioCapture::default()),
        ProviderKind::Mimo => Arc::new(CpalCapture),
    };
    let asr: Arc<dyn AsrProvider> = match settings.asr.provider {
        ProviderKind::Mock => Arc::new(MockAsrProvider::default()),
        ProviderKind::Mimo => Arc::new(MimoAsrProvider::new(settings.asr.clone())),
    };
    let summarizer: Arc<dyn SummarizerProvider> = match settings.summarizer.provider {
        ProviderKind::Mock => Arc::new(MockSummarizer::default()),
        ProviderKind::Mimo => Arc::new(MimoFlashSummarizer::new(settings.summarizer.clone())),
    };
    Providers {
        capture,
        asr,
        summarizer,
    }
}

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;

    let config_store = ConfigStore::new(data_dir.join("config.json"));
    let settings = config_store.load();
    let providers: SharedProviders = Arc::new(RwLock::new(Arc::new(build_providers(&settings))));
    let store = Arc::new(FsSessionStore::new(&data_dir));

    let (sink, mut events) = tokio::sync::mpsc::unbounded_channel();
    let (manager, actor) = SessionManager::create(providers.clone(), store, sink);
    tauri::async_runtime::spawn(actor);

    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.recv().await {
            crate::ipc::events::emit_ui_event(&handle, &event);
        }
    });

    app.manage(AppState {
        manager,
        config_store,
        providers,
        data_dir,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    });
    Ok(())
}
