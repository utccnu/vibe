use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::{env, process};
use tauri::App;
use vibe::config::{get_models_folder, TranscribeOptions};
use vibe::model;

/// Attach to console if cli detected in Windows
#[cfg(all(windows, feature = "attach-console"))]
pub fn attach_console() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    if env::var("RUST_LOG").is_ok() || is_cli_detected() {
        // we ignore the result here because
        // if the app started from a command line, like cmd or powershell,
        // it will attach sucessfully which is what we want
        // but if we were started from something like explorer,
        // it will fail to attach console which is also what we want.
        let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    }
}

pub fn is_cli_detected() -> bool {
    // Get the command-line arguments as an iterator
    let args: Vec<String> = env::args().collect();

    // Check if any argument starts with "--"
    for arg in &args {
        if arg.starts_with("--") || arg == "-h" {
            return true;
        }
    }
    false
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to model
    #[arg(long)]
    model: PathBuf,

    /// Path to file to transcribe
    #[arg(long)]
    file: PathBuf,

    /// Language to transcribe (default: "en")
    #[arg(long, default_value = "en")]
    language: Option<String>,

    /// Temperature (default: 0.4)
    #[arg(long, default_value = "0.4")]
    temperature: Option<f32>,

    /// Number of threads (default: 4)
    #[arg(long, default_value = "4")]
    n_threads: Option<i32>,

    /// Whether to translate (default: false)
    #[arg(long)]
    translate: Option<bool>,

    /// Initial prompt (default: None)
    #[arg(long)]
    init_prompt: Option<String>,

    /// Path to write transcript
    #[arg(long)]
    write: Option<PathBuf>,

    /// Format of the transcript (default: "srt") possible: (srt, vtt, text)
    #[structopt(long, default_value = "srt")] // TODO: use possible values. confusing crate!
    format: String,
}

fn prepare_model_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    // Check if relative to current dir
    if path.exists() {
        return path.to_path_buf();
    }
    // Check if relative to app config exists
    let relative_to_models_folder = get_models_folder().unwrap().join(path);
    if relative_to_models_folder.exists() {
        return relative_to_models_folder;
    }
    path.to_path_buf()
}

pub fn run(app: &App) {
    #[cfg(target_os = "macos")]
    crate::dock::set_dock_visible(false);

    let args = Args::parse();
    let mut options = TranscribeOptions {
        path: args.file,
        model_path: args.model,
        lang: args.language,
        init_prompt: args.init_prompt,
        n_threads: args.n_threads,
        temperature: args.temperature,
        translate: args.translate,
        verbose: false,
    };
    options.model_path = prepare_model_path(&options.model_path);

    eprintln!("Transcribe... 🔄");
    let start = Instant::now(); // Measure start time
    let transcript = model::transcribe(&options, None, None, None).unwrap();
    let elapsed = start.elapsed();
    println!(
        "{}",
        match args.format.as_str() {
            "srt" => transcript.as_srt(),
            "vtt" => transcript.as_vtt(),
            "text" => transcript.as_text(),
            _ => {
                eprintln!("Invalid format specified. Defaulting to SRT format.");
                transcript.as_srt()
            }
        }
    );

    // Write transcript if write path is provided
    if let Some(write_path) = args.write {
        if let Err(err) = std::fs::write(
            write_path,
            match args.format.as_str() {
                "srt" => transcript.as_srt(),
                "vtt" => transcript.as_vtt(),
                "text" => transcript.as_text(),
                _ => {
                    eprintln!("Invalid format specified. Defaulting to SRT format.");
                    transcript.as_srt()
                }
            },
        ) {
            eprintln!("Error writing transcript to file: {}", err);
        }
    }

    app.cleanup_before_exit();
    eprintln!(
        "Transcription completed in {:.1}s ⏱️",
        elapsed.as_secs_f64() + elapsed.subsec_nanos() as f64 * 1e-9
    );
    eprintln!("Done ✅");
    process::exit(0);
}
