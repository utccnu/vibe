use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use crate::config::{TranscribeModuleConfig, ModelConfig};
use vibe_core::transcribe::WhisperContext;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ModelContext {
    pub transcribe_config: TranscribeModuleConfig,
    pub model_config: ModelConfig,
    pub whisper: Arc<Mutex<Option<WhisperContext>>>,
    pub results: Arc<Mutex<HashMap<String, crate::server::TranscriptionResult>>>,
    pub current_model_path: Arc<Mutex<Option<PathBuf>>>,
}

impl ModelContext {
    pub fn new(transcribe_config: TranscribeModuleConfig, model_config: ModelConfig) -> eyre::Result<Self> {
        Ok(Self {
            transcribe_config,
            model_config,
            whisper: Arc::new(Mutex::new(None)),
            results: Arc::new(Mutex::new(HashMap::new())),
            current_model_path: Arc::new(Mutex::new(None)),
        })
    }
}
