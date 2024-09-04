use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub models: ModelConfig,
    pub transcribe_module: TranscribeModuleConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ModelConfig {
    pub model_directory: String,
    pub default_model: String,
    pub mappings: HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct TranscribeModuleConfig {
    pub language: Option<String>,
    pub initial_prompt: Option<String>,
    pub translate: bool,
    pub word_timestamps: bool,
    pub max_text_ctx: Option<usize>,
    pub max_sentence_len: Option<usize>,
    pub n_threads: Option<usize>,
    pub temperature: Option<f32>,
    pub detect_language: bool,
    pub diarize: bool,
    pub max_speakers: Option<usize>,
    pub beam_size: Option<usize>,
    pub best_of: Option<usize>,
    pub speaker_recognition_threshold: Option<f32>,
    pub vad_filter: bool,
    pub vad_parameters: VadParameters,
    pub segment_model_filename: String,
    pub embedding_model_filename: String,
    pub embedding_model_url: String,
    pub segment_model_url: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct VadParameters {
    pub threshold: f32,
    pub min_speech_duration_ms: u32,
    pub min_silence_duration_ms: u32,
    pub speech_pad_ms: u32,
}

pub fn load_config(config_path: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string(config_path)?;
    info!("Config content: {}", config_str);
    let config: Config = toml::from_str(&config_str)?;
    info!("Parsed config: {:?}", config);
    Ok(config)
}
