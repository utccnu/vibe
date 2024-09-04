use axum::{
    extract::{State, Multipart},
    response::{Json, IntoResponse},
};
use serde::{Deserialize, Serialize};
use crate::setup::ModelContext;
use tokio::sync::mpsc;
use std::path::PathBuf;
use uuid::Uuid;
use vibe_core::{config::TranscribeOptions, transcribe};
use eyre::{Result, eyre};
use reqwest;
use crate::config::VadParameters;
// use vibe_core::transcribe;

#[derive(Deserialize)]
pub struct LoadPayload {
    model_name: String,
}

#[derive(Serialize)]
pub struct LoadResponse {
    success: bool,
    message: String,
}

async fn download_file(url: &str, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let mut file = tokio::fs::File::create(path).await?;
    let mut content = std::io::Cursor::new(response.bytes().await?);
    tokio::io::copy(&mut content, &mut file).await?;
    Ok(())
}

pub async fn load(
    State(context): State<ModelContext>,
    Json(payload): Json<LoadPayload>,
) -> impl IntoResponse {
    let model_dir = PathBuf::from(&context.model_config.model_directory);
    let embedding_model_path = model_dir.join(&context.transcribe_config.embedding_model_filename);
    let segment_model_path = model_dir.join(&context.transcribe_config.segment_model_filename);

    // Download embedding model if it doesn't exist
    if !embedding_model_path.exists() {
        if let Err(e) = download_file(&context.transcribe_config.embedding_model_url, &embedding_model_path).await {
            return Json(LoadResponse {
                success: false,
                message: format!("Failed to download embedding model: {}", e),
            });
        }
    }

    // Download segment model if it doesn't exist
    if !segment_model_path.exists() {
        if let Err(e) = download_file(&context.transcribe_config.segment_model_url, &segment_model_path).await {
            return Json(LoadResponse {
                success: false,
                message: format!("Failed to download segment model: {}", e),
            });
        }
    }

    // Get the actual filename from the model mappings
    let model_path = match context.model_config.mappings.get(&payload.model_name) {
        Some(filename) => model_dir.join(filename),
        None => return Json(LoadResponse {
            success: false,
            message: format!("Model '{}' not found in mappings", payload.model_name),
        }),
    };

    if !model_path.exists() {
        return Json(LoadResponse {
            success: false,
            message: format!("Model file not found: {}", model_path.display()),
        });
    }

    // Initialize the Whisper context
    let mut whisper_context = context.whisper.lock().await;
    match transcribe::create_context(&model_path, None) {
        Ok(ctx) => {
            *whisper_context = Some(ctx);
            *context.current_model_path.lock().await = Some(model_path.clone());
            Json(LoadResponse {
                success: true,
                message: format!("Model {} (file: {}) loaded successfully", payload.model_name, model_path.file_name().unwrap().to_string_lossy()),
            })
        },
        Err(e) => Json(LoadResponse {
            success: false,
            message: format!("Failed to initialize Whisper context: {}", e),
        }),
    }
}

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
    pub segment_model_path: Option<String>,
    pub embedding_model_path: Option<String>,
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
            segment_model_path: None,
            embedding_model_path: None,
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
        segment_model_path: Some(context.transcribe_config.segment_model_filename.clone()),
        embedding_model_path: Some(context.transcribe_config.embedding_model_filename.clone()),
    };

    let mut model_name = context.model_config.default_model.clone();

    // Process multipart form data
    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.name() {
            match name {
                "file" => {
                    let file_name = match field.file_name() {
                        Some(name) => name.to_string(),
                        None => return Json(serde_json::json!({
                            "status": "error",
                            "message": "File name not provided"
                        })),
                    };
                    
                    let content = match field.bytes().await {
                        Ok(data) => data,
                        Err(e) => return Json(serde_json::json!({
                            "status": "error",
                            "message": format!("Failed to read file data: {}", e)
                        })),
                    };
                    
                    let temp_dir = std::env::temp_dir();
                    let file_path_buf = temp_dir.join(&file_name);
                    
                    if let Err(e) = tokio::fs::write(&file_path_buf, content).await {
                        return Json(serde_json::json!({
                            "status": "error",
                            "message": format!("Failed to save file: {}", e)
                        }));
                    }
                    
                    file_path = Some(file_path_buf);
                    tracing::info!("File saved to: {:?}", file_path);
                },
                "task_options" => {
                    match field.text().await {
                        Ok(options_str) => {
                            tracing::info!("Received task_options: {}", options_str);
                            task_options = match serde_json::from_str(&options_str) {
                                Ok(options) => Some(options),
                                Err(e) => {
                                    tracing::error!("Failed to parse task options: {}", e);
                                    return Json(serde_json::json!({
                                        "status": "error",
                                        "message": format!("Failed to parse task options: {}", e)
                                    }));
                                }
                            };
                        },
                        Err(e) => {
                            tracing::error!("Failed to read task options: {}", e);
                            return Json(serde_json::json!({
                                "status": "error",
                                "message": format!("Failed to read task options: {}", e)
                            }));
                        }
                    }
                },
                "model" => {
                    match field.text().await {
                        Ok(model) => model_name = model,
                        Err(e) => {
                            return Json(serde_json::json!({
                                "status": "error",
                                "message": format!("Failed to read model name: {}", e)
                            }));
                        }
                    }
                },
                _ => {} // Ignore unknown fields
            }
        }
    }

    let file_path = match file_path {
        Some(path) => {
            tracing::info!("File path before transcription: {:?}", path);
            path
        },
        None => return Json(serde_json::json!({
            "status": "error",
            "message": "No file uploaded"
        })),
    };

    // Check if the file actually exists before passing it to perform_transcription
    if !file_path.exists() {
        tracing::error!("File does not exist: {:?}", file_path);
        return Json(serde_json::json!({
            "status": "error",
            "message": "Uploaded file not found"
        }));
    }

    let task_options: TranscribeModuleOptions = task_options.unwrap_or_default();

    // Merge task_options into module_options
    if let Some(diarize) = task_options.diarize {
        module_options.diarize = Some(diarize);
    }
    if let Some(max_speakers) = task_options.max_speakers {
        module_options.max_speakers = Some(max_speakers);
    }
    if let Some(threshold) = task_options.speaker_recognition_threshold {
        module_options.speaker_recognition_threshold = Some(threshold);
    }
    if let Some(core_options) = task_options.core_options.clone() {
        module_options.core_options = Some(core_options);
    }
    if let Some(segment_model_path) = task_options.segment_model_path.clone() {
        module_options.segment_model_path = Some(segment_model_path);
    }
    if let Some(embedding_model_path) = task_options.embedding_model_path.clone() {
        module_options.embedding_model_path = Some(embedding_model_path);
    }

    // Get the model path
    let model_path = match context.model_config.mappings.get(&model_name) {
        Some(filename) => PathBuf::from(&context.model_config.model_directory).join(filename),
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
        tracing::info!("Spawning transcription task with file_path: {:?}", file_path);
        let result = perform_transcription(file_path.clone(), model_path, module_options, tx, context_clone).await;
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

/// API endpoint for listing available transcription models
pub async fn list_models(State(context): State<ModelContext>) -> impl IntoResponse {
    let model_dir = PathBuf::from(&context.model_config.model_directory);
    let available_models: Vec<String> = context.model_config.mappings
        .iter()
        .filter(|(_, filename)| model_dir.join(filename).exists())
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

async fn perform_transcription(
    file_path: PathBuf,
    model_path: PathBuf,
    mut module_options: TranscribeModuleOptions,
    progress_tx: mpsc::Sender<f32>,
    context: ModelContext,
) -> Result<TranscriptionResult> {
    tracing::info!("Entering perform_transcription with file_path: {:?}", file_path);

    let mut whisper_context = context.whisper.lock().await;
    
    // Check if the context is initialized with the correct model
    let current_model_path = context.current_model_path.lock().await;
    if current_model_path.as_ref() != Some(&model_path) {
        drop(current_model_path); // Release the lock before modifying
        *whisper_context = Some(transcribe::create_context(&model_path, None)?);
        *context.current_model_path.lock().await = Some(model_path.clone());
    }

    let ctx = whisper_context.as_ref().ok_or_else(|| eyre!("Whisper context not initialized"))?;

    // Ensure the file path is set correctly in the core options
    if let Some(core_options) = module_options.core_options.as_mut() {
        core_options.path = file_path.to_str()
            .ok_or_else(|| eyre!("Invalid file path"))?
            .to_string();
        tracing::info!("Set core_options.path to: {}", core_options.path);
    } else {
        return Err(eyre!("Core options not initialized"));
    }

    // Log the file path for debugging
    tracing::info!("Transcribing file: {:?}", file_path);

    let progress_callback = move |progress: i32| {
        let _ = progress_tx.try_send(progress as f32 / 100.0);
    };

    // Prepare diarization options
    let diarize_options = if module_options.diarize.unwrap_or(false) {
        let model_dir = PathBuf::from(&context.model_config.model_directory);
        let options = Some(transcribe::DiarizeOptions {
            threshold: module_options.speaker_recognition_threshold.unwrap_or(0.5),
            max_speakers: module_options.max_speakers.unwrap_or(2),
            embedding_model_path: model_dir.join(&context.transcribe_config.embedding_model_filename).to_str().unwrap().to_string(),
            segment_model_path: model_dir.join(&context.transcribe_config.segment_model_filename).to_str().unwrap().to_string(),
        });
        tracing::info!("Diarization options: {:?}", options);
        options
    } else {
        None
    };

    let transcript = transcribe::transcribe(
        ctx,
        module_options.core_options.as_ref().unwrap(),
        Some(Box::new(progress_callback)),
        None, // new_segment_callback
        None, // abort_callback
        diarize_options,
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
