use std::sync::Arc;
use tokio::sync::Mutex;
use vibe_core::whisper::WhisperContext;

pub struct ModelContext {
    pub whisper: Arc<Mutex<Option<WhisperContext>>>,
}

impl ModelContext {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            whisper: Arc::new(Mutex::new(None)),
        })
    }
}
