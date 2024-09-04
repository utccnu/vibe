use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ModelContext {
    pub whisper: Arc<Mutex<Option<String>>>,
}

impl ModelContext {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            whisper: Arc::new(Mutex::new(Some("Placeholder for WhisperContext".to_string()))),
        })
    }
}
