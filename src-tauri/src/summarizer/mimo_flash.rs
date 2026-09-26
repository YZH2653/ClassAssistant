// 小米 MiMo v2.6 flash 真实实现（TODO：等涵涵提供 API 文档后对接）
use super::*;
use crate::domain::settings::ProviderConfig;

pub struct MimoFlashSummarizer {
    pub config: ProviderConfig,
}

impl MimoFlashSummarizer {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SummarizerProvider for MimoFlashSummarizer {
    fn name(&self) -> &'static str {
        "mimo"
    }

    async fn summarize(&self, _req: SummaryRequest) -> Result<SummaryResult, SummaryError> {
        let _ = &self.config;
        Err(SummaryError(
            "MiMo flash 尚未接入，等 API 文档".to_string(),
        ))
    }
}
