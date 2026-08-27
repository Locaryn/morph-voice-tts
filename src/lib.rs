//! Locaryn Voice & TTS Plugin
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    #[serde(default = "default_voice")]
    pub voice: String,
    #[serde(default = "default_speed")]
    pub speed: f32,
    pub output_dir: Option<PathBuf>,
}
fn default_voice() -> String {
    "fr-siwis".into()
}
fn default_speed() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResult {
    pub audio_path: PathBuf,
    pub duration_seconds: f32,
}

pub fn models_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("LOCARYN_EXTENSION_MODELS_DIR") {
        PathBuf::from(dir)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("models")
    }
}

pub fn list_voices() -> Vec<String> {
    let dir = models_dir();
    let mut voices = Vec::new();
    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
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

/// Non implemente. La signature est conservee pour que l'interface et le
/// serveur MCP gardent leur forme, mais l'appel echoue franchement plutot
/// que de fabriquer un resultat.
pub async fn synthesize_speech(_req: TtsRequest) -> Result<TtsResult, String> {
    Err("La synthese vocale n'est pas implementee : ce morph n'embarque aucun moteur TTS. Aucun fichier n'a ete produit.".into())
}
