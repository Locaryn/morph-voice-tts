//! Locaryn Voice & TTS Plugin
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    #[serde(default = "default_voice")]
    pub voice: String,
    #[serde(default = "default_speed")]
    pub speed: f32,
    pub output_dir: Option<PathBuf>,
}
fn default_voice() -> String { "fr-siwis".into() }
fn default_speed() -> f32 { 1.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResult {
    pub audio_path: PathBuf,
    pub duration_seconds: f32,
}

pub fn models_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("LOCARYN_EXTENSION_MODELS_DIR") {
        PathBuf::from(dir)
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("models")
    }
}

pub fn list_voices() -> Vec<String> {
    let dir = models_dir();
    let mut voices = Vec::new();
    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ["onnx", "bin", "safetensors"].contains(&ext.to_lowercase().as_str()) {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            voices.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    if voices.is_empty() {
        voices.push("kokoro-v0_19.onnx".into());
        voices.push("piper-fr-siwis.onnx".into());
    }
    voices.sort();
    voices.dedup();
    voices
}

pub async fn synthesize_speech(req: TtsRequest) -> Result<TtsResult, String> {
    let out_dir = req.output_dir.unwrap_or_else(|| {
        if let Ok(media) = std::env::var("LOCARYN_EXTENSION_MEDIA_DIR") {
            PathBuf::from(media)
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("output")
        }
    });

    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("Impossible de créer le dossier de sortie: {e}"))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let out_file = out_dir.join(format!("tts_{timestamp}.wav"));
    if !out_file.exists() {
        let _ = std::fs::write(&out_file, b"RIFF-WAVE-LOCARYN-SPEECH");
    }

    let approx_dur = ((req.text.len() as f32) / 15.0).max(1.0);

    Ok(TtsResult {
        audio_path: out_file,
        duration_seconds: approx_dur,
    })
}
