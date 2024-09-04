use axum::{extract::State, response::Json, extract::Multipart, response::IntoResponse};
use serde::{Deserialize, Serialize};
use crate::setup::ModelContext;
use tokio::sync::mpsc;
use std::path::PathBuf;
use uuid::Uuid;
use vibe_core::{config::TranscribeOptions, transcribe};
use crate::config::VadParameters;
use eyre::{Result, eyre};

#[allow(dead_code)]
#[derive(Deserialize, Default)]
pub struct TranscribeRequest {
    #[serde(flatten)]
    pub module_options: TranscribeModuleOptions,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TranscribeModuleOptions {
    pub core_options: Option<TranscribeOptions>,
    pub diarize: Option<bool>,
    pub max_speakers: Option<usize>,
    pub speaker_recognition_threshold: Option<f32>,
    pub vad_filter: Option<bool>,
    pub vad_parameters: Option<VadParameters>,
}

impl Default for TranscribeModuleOptions {
    fn default() -> Self {
        Self {
            core_options: None,
            diarize: None,
            max_speakers: None,
            speaker_recognition_threshold: None,
            vad_filter: None,
            vad_parameters: None,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TranscriptionResponse {
    job_id: String,
    status: String,
}

#[derive(Serialize, Clone)]
pub struct TranscriptionResult {
    text: String,
    segments: Vec<Segment>,
}

#[derive(Serialize, Clone)]
pub struct Segment {
    start: f32,
    end: f32,
    text: String,
    speaker: Option<String>,
}

/// API endpoint for initiating a transcription job
pub async fn transcribe(
    State(context): State<ModelContext>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Generate a unique job ID for this transcription task
    let job_id = Uuid::new_v4().to_string();
    let job_id_for_task = job_id.clone();

    let mut file_path = None;
    let mut task_options = None;
    // Initialize module options with values from the config file
    let mut module_options = TranscribeModuleOptions {
        core_options: Some(TranscribeOptions {
            path: "".to_string(), // This will be set later
            lang: context.transcribe_config.language.clone(),
            init_prompt: context.transcribe_config.initial_prompt.clone(),
            translate: Some(context.transcribe_config.translate),
            word_timestamps: Some(context.transcribe_config.word_timestamps),
            max_text_ctx: context.transcribe_config.max_text_ctx.map(|n| n as i32),
            max_sentence_len: context.transcribe_config.max_sentence_len.map(|n| n as i32),
            n_threads: context.transcribe_config.n_threads.map(|n| n as i32),
            temperature: context.transcribe_config.temperature,
            verbose: Some(false),
        }),
        diarize: Some(context.transcribe_config.diarize),
        max_speakers: context.transcribe_config.max_speakers,
        speaker_recognition_threshold: context.transcribe_config.speaker_recognition_threshold,
        vad_filter: Some(context.transcribe_config.vad_filter),
        vad_parameters: Some(context.transcribe_config.vad_parameters.clone()),
    };

    let mut model_name = context.model_config.default_model.clone();

    // Process multipart form data
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        if name == "file" {
            // Handle file upload
            let file_name = field.file_name().unwrap().to_string();
            let content = field.bytes().await.unwrap();
            let temp_dir = std::env::temp_dir();
            let file_path_buf = temp_dir.join(&file_name);
            tokio::fs::write(&file_path_buf, content).await.unwrap();
            file_path = Some(file_path_buf);
        } else if name == "task_options" {
            // Parse task-specific options
            let options_str = field.text().await.unwrap();
            task_options = Some(serde_json::from_str(&options_str).unwrap());
        } else if name == "module_options" {
            // Parse module-specific options (overriding config file values)
            let options_str = field.text().await.unwrap();
            module_options = serde_json::from_str(&options_str).unwrap();
        } else if name == "model" {
            model_name = field.text().await.unwrap();
        }
    }

    let file_path = file_path.unwrap();
    let task_options: TranscribeModuleOptions = task_options.unwrap_or_default();

    // Get the model path
    let model_path = match context.model_config.mappings.get(&model_name) {
        Some(filename) => context.model_config.model_directory.join(filename),
        None => return Json(serde_json::json!({
            "status": "error",
            "message": format!("Model '{}' not found in configuration", model_name)
        })),
    };

    // Check if the model file exists
    if !model_path.exists() {
        return Json(serde_json::json!({
            "status": "error",
            "message": format!("Model file for '{}' not found", model_name)
        }));
    }

    // Create a channel for progress updates
    let (tx, _rx) = mpsc::channel(100);
    let context_clone = context.clone();

    // Spawn a new task to perform the transcription asynchronously
    tokio::spawn(async move {
        let result = perform_transcription(file_path, model_path, task_options, module_options, tx, context_clone).await;
        match result {
            Ok(transcription) => {
                context.results.lock().await.insert(job_id_for_task, transcription);
            }
            Err(e) => {
                tracing::error!("Transcription error: {:?}", e);
                // TODO: Handle error (e.g., store error message in results)
            }
        }
    });

    // Return the job ID and status to the client
    Json(serde_json::json!({"job_id": job_id, "status": "processing"}))
}

/// API endpoint for checking the status of a transcription job
pub async fn get_transcription_status(
    State(context): State<ModelContext>,
    Json(payload): Json<JobStatusRequest>,
) -> impl IntoResponse {
    let results = context.results.lock().await;
    let status = if results.contains_key(&payload.job_id) {
        "completed"
    } else {
        "processing"
    };
    
    Json(TranscriptionResponse {
        job_id: payload.job_id,
        status: status.to_string(),
    })
}

/// API endpoint for retrieving the result of a completed transcription job
pub async fn get_transcription_result(
    State(context): State<ModelContext>,
    Json(payload): Json<JobStatusRequest>,
) -> impl IntoResponse {
    let results = context.results.lock().await;
    if let Some(result) = results.get(&payload.job_id) {
        Json(result.clone())
    } else {
        Json(TranscriptionResult {
            text: "Job not found".to_string(),
            segments: vec![],
        })
    }
}

/// API endpoint for loading a transcription model
pub async fn load(
    State(context): State<ModelContext>,
    Json(payload): Json<LoadModelRequest>,
) -> impl IntoResponse {
    let model_path = match context.model_config.mappings.get(&payload.model_name) {
        Some(filename) => context.model_config.model_directory.join(filename),
        None => return Json(serde_json::json!({
            "status": "error",
            "message": format!("Model '{}' not found in configuration", payload.model_name)
        })),
    };

    // Check if the model file exists
    if !model_path.exists() {
        return Json(serde_json::json!({
            "status": "error",
            "message": format!("Model file for '{}' not found", payload.model_name)
        }));
    }

    match transcribe::create_context(&model_path, None) {
        Ok(whisper_context) => {
            let mut context_guard = context.whisper.lock().await;
            *context_guard = Some(whisper_context);
            Json(serde_json::json!({"status": "success", "message": "Model loaded successfully"}))
        }
        Err(e) => {
            Json(serde_json::json!({"status": "error", "message": format!("Failed to load model: {}", e)}))
        }
    }
}

/// API endpoint for listing available transcription models
pub async fn list_models(State(context): State<ModelContext>) -> impl IntoResponse {
    let available_models: Vec<String> = context.model_config.mappings.iter()
        .filter(|(_, filename)| context.model_config.model_directory.join(filename).exists())
        .map(|(name, _)| name.clone())
        .collect();

    Json(serde_json::json!({
        "models": available_models,
        "default_model": context.model_config.default_model,
        "configured_models": context.model_config.mappings.keys().collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct JobStatusRequest {
    pub job_id: String,
}

#[derive(Deserialize)]
pub struct LoadModelRequest {
    model_name: String,
}

async fn perform_transcription(
    file_path: PathBuf,
    model_path: PathBuf,
    task_options: TranscribeModuleOptions,
    mut module_options: TranscribeModuleOptions,
    progress_tx: mpsc::Sender<f32>,
    context: ModelContext,
) -> Result<TranscriptionResult> {
    let mut whisper_context = context.whisper.lock().await;
    
    // Check if the context is initialized with the correct model
    let current_model_path = context.current_model_path.lock().await;
    if current_model_path.as_ref() != Some(&model_path) {
        drop(current_model_path); // Release the lock before modifying
        *whisper_context = Some(transcribe::create_context(&model_path, None)?);
        *context.current_model_path.lock().await = Some(model_path.clone());
    }

    let ctx = whisper_context.as_ref().ok_or_else(|| eyre!("Whisper context not initialized"))?;

    // Override module options with task-specific options if provided
    if let Some(core_options) = &task_options.core_options {
        if let Some(lang) = &core_options.lang {
            module_options.core_options.as_mut().unwrap().lang = Some(lang.clone());
        }
        if let Some(init_prompt) = &core_options.init_prompt {
            module_options.core_options.as_mut().unwrap().init_prompt = Some(init_prompt.clone());
        }
        // ... (apply other overrides as needed)
    }

    // Set the file path in the core options
    module_options.core_options.as_mut().unwrap().path = file_path.to_str().unwrap_or_default().to_string();

    let progress_callback = move |progress: i32| {
        let _ = progress_tx.try_send(progress as f32 / 100.0);
    };

    // TODO: Implement diarization and VAD options if supported by vibe_core
    // For now, we'll just use the core transcribe function

    let transcript = transcribe::transcribe(
        ctx,
        module_options.core_options.as_ref().unwrap(),
        Some(Box::new(progress_callback)),
        None, // diarize_options
        None, // vad_options
        None,
    )?;

    let result = TranscriptionResult {
        text: transcript.segments.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join(" "),
        segments: transcript.segments.into_iter().map(|s| Segment {
            start: s.start as f32 / 100.0,
            end: s.stop as f32 / 100.0,
            text: s.text,
            speaker: s.speaker.map(|s| format!("Speaker {}", s)),
        }).collect(),
    };

    Ok(result)
}
