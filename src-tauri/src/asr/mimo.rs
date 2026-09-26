// 小米 MiMo v2.5 ASR 真实实现（TODO：等涵涵提供 API 文档后对接）
use super::*;
use crate::domain::settings::ProviderConfig;

pub struct MimoAsrProvider {
    pub config: ProviderConfig,
}

impl MimoAsrProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl AsrProvider for MimoAsrProvider {
    fn name(&self) -> &'static str {
        "mimo"
    }

    async fn start(&self, _cfg: AsrConfig) -> Result<Box<dyn AsrStream>, AsrError> {
        let _ = &self.config;
        Err(AsrError("MiMo ASR 尚未接入，等 API 文档".to_string()))
    }
}
