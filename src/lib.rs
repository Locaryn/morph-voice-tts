//! Synthèse vocale locale, par les moteurs Python que la machine héberge.
//!
//! Deux moteurs, choisis d'après le nom du dépôt de poids : **Kokoro**, léger
//! et immédiat, avec ses voix livrées dans `voices/` ; **Qwen3-TTS**, plus
//! lent mais plus riche. Rien ne sort de la machine : le texte entre, un WAV
//! sort, et les poids ne bougent pas.
//!
//! Le clonage vocal n'est volontairement pas ici. La variante « Base » de
//! Qwen3-TTS ne sait que cloner : elle est refusée par son nom plutôt que de
//! finir en trace d'exécution Python illisible.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    /// Nom du dépôt de poids, tel que le rend [`list_voices`]. Vide = le
    /// premier modèle utilisable installé.
    #[serde(default)]
    pub voice: String,
    #[serde(default = "default_speed")]
    pub speed: f32,
    /// Code de langue ISO (« fr », « en », « ja »…). Absent = déduit du texte.
    #[serde(default)]
    pub language: Option<String>,
    pub output_dir: Option<PathBuf>,
}

fn default_speed() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResult {
    pub audio_path: PathBuf,
    pub duration_seconds: f32,
    /// Le dépôt qui a parlé, et la voix retenue à l'intérieur.
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    pub language: String,
}

// ── Emplacements ────────────────────────────────────────────────────────────

/// Où chercher les poids.
///
/// La bibliothèque de la personne d'abord (`LOCARYN_MODELS_DIR`) : c'est là
/// que vivent les modèles déjà téléchargés. Le dossier privé du morph ensuite,
/// vide au premier lancement. Sans hôte, `./models`, pour pouvoir travailler
/// hors application.
pub fn models_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for key in ["LOCARYN_MODELS_DIR", "LOCARYN_EXTENSION_MODELS_DIR"] {
        if let Ok(dir) = std::env::var(key) {
            if !dir.trim().is_empty() {
                out.push(PathBuf::from(dir));
            }
        }
    }
    out.push(
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("models"),
    );
    out
}

/// Le premier emplacement de poids, pour le code qui n'en veut qu'un.
pub fn models_dir() -> PathBuf {
    models_dirs()
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("models"))
}

/// Où déposer le WAV produit.
fn default_output_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("LOCARYN_EXTENSION_MEDIA_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("output")
}

// ── Inventaire des voix ─────────────────────────────────────────────────────

/// Les dépôts installés qui savent parler.
///
/// Un dépôt HuggingFace extrait porte le moteur dans son nom
/// (`hexgrad__Kokoro-82M`, `Qwen__Qwen3-TTS-…`) ; un modèle Piper est un
/// `.onnx` posé à la racine. La liste est vide quand rien n'est installé —
/// annoncer des voix absentes ne ferait que déplacer l'échec.
pub fn list_voices() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for dir in models_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_ascii_lowercase();
            if path.is_dir() {
                let qwen_tts = lower.contains("qwen3") && lower.contains("tts");
                if qwen_tts
                    || ["kokoro", "xtts", "piper", "parler", "omni"]
                        .iter()
                        .any(|k| lower.contains(k))
                {
                    names.push(name);
                }
            } else if lower.ends_with(".onnx") {
                names.push(name);
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Une voix capable de parler seule, sans enregistrement de référence.
///
/// La variante « Base » de Qwen3-TTS n'a pas de voix propre : elle ne sait que
/// cloner celle qu'on lui donne. La proposer en tête — l'ordre alphabétique la
/// place là — menait à un échec systématique quand personne n'avait choisi.
pub fn has_own_voice(name: &str) -> bool {
    !name.to_ascii_lowercase().contains("base")
}

/// Les voix installées qui parlent sans référence, la plus autonome d'abord.
///
/// Kokoro tourne avec ce que l'installation apporte ; Qwen3-TTS réclame des
/// paquets qui peuvent manquer. Sans choix explicite, mieux vaut commencer par
/// celle qui a le plus de chances de répondre.
pub fn list_usable_voices() -> Vec<String> {
    let mut v: Vec<String> = list_voices()
        .into_iter()
        .filter(|n| has_own_voice(n))
        .collect();
    v.sort_by_key(|n| {
        let l = n.to_ascii_lowercase();
        if l.contains("kokoro") {
            0
        } else if l.ends_with(".onnx") || l.contains("piper") {
            1
        } else {
            2
        }
    });
    v
}

/// Le dossier d'un dépôt de poids, cherché dans chaque emplacement connu.
pub fn repo_path(model: &str) -> Option<PathBuf> {
    models_dirs()
        .into_iter()
        .map(|d| d.join(model))
        .find(|p| p.exists())
}

// ── Moteurs ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Engine {
    Kokoro,
    Qwen3,
}

fn resolve_engine(model: &str) -> Result<Engine, String> {
    let lower = model.to_ascii_lowercase();
    if lower.contains("kokoro") {
        Ok(Engine::Kokoro)
    } else if lower.contains("qwen3") && lower.contains("tts") {
        Ok(Engine::Qwen3)
    } else {
        Err(format!(
            "Modèle audio non pris en charge : {model}. Moteurs connus : Kokoro, Qwen3-TTS."
        ))
    }
}

/// La langue du texte, à la même heuristique que le reste de l'application.
pub fn detect_language(text: &str) -> &'static str {
    let has = |ranges: &[(char, char)]| {
        text.chars()
            .any(|c| ranges.iter().any(|(a, b)| (*a..=*b).contains(&c)))
    };
    if has(&[('\u{4e00}', '\u{9fff}'), ('\u{3400}', '\u{4dbf}')]) {
        return "zh";
    }
    if has(&[('\u{3040}', '\u{309f}'), ('\u{30a0}', '\u{30ff}')]) {
        return "ja";
    }
    if has(&[('\u{ac00}', '\u{d7af}')]) {
        return "ko";
    }
    if has(&[('\u{0600}', '\u{06ff}')]) {
        return "ar";
    }
    if has(&[('\u{0400}', '\u{04ff}')]) {
        return "ru";
    }
    let any = |set: &str| text.chars().any(|c| set.contains(c));
    if any("ãõ") {
        return "pt";
    }
    if any("äöüÄÖÜ") {
        return "de";
    }
    if any("àèéêëçôû") {
        return "fr";
    }
    if any("ñ¿¡") {
        return "es";
    }
    if any("ìòù") {
        return "it";
    }
    "en"
}

// ── Kokoro ──────────────────────────────────────────────────────────────────

/// Les voix livrées dans un dépôt Kokoro : `af_heart`, `am_michael`…
pub fn kokoro_voices_in_repo(repo_dir: &Path) -> Vec<String> {
    let mut voices = Vec::new();
    if let Ok(entries) = std::fs::read_dir(repo_dir.join("voices")) {
        voices = entries
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("pt") {
                    return None;
                }
                p.file_stem().and_then(|s| s.to_str()).map(str::to_string)
            })
            .collect();
    }
    voices.sort();
    voices
}

/// La langue qu'annonce le préfixe d'une voix Kokoro.
fn kokoro_voice_lang(voice: &str) -> Option<&'static str> {
    Some(match voice.chars().next()? {
        'a' | 'b' => "en",
        'f' => "fr",
        'e' => "es",
        'd' => "de",
        'i' => "it",
        'p' => "pt",
        'j' => "ja",
        'z' | 'c' => "zh",
        _ => return None,
    })
}

fn resolve_kokoro_voice(repo_dir: &Path, language: &str) -> Result<String, String> {
    let voices = kokoro_voices_in_repo(repo_dir);
    if voices.is_empty() {
        return Err("Aucune voix Kokoro dans le dossier voices/.".into());
    }
    let pick = |lang: &str| {
        voices
            .iter()
            .find(|v| kokoro_voice_lang(v) == Some(lang))
            .cloned()
    };
    // La langue demandée, sinon l'anglais, sinon la première venue : mieux
    // vaut une voix au mauvais accent qu'un refus.
    pick(language)
        .or_else(|| pick("en"))
        .or_else(|| voices.first().cloned())
        .ok_or_else(|| format!("Aucune voix Kokoro utilisable pour « {language} »."))
}

async fn run_kokoro(
    python: &str,
    repo_dir: &Path,
    text: &str,
    out_file: &Path,
    speed: f32,
    language: &str,
) -> Result<String, String> {
    let weights = walkdir_recursive(repo_dir, 3)
        .into_iter()
        .find(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("pth") || e.eq_ignore_ascii_case("onnx"))
                .unwrap_or(false)
        })
        .ok_or_else(|| "Aucun fichier .pth ou .onnx dans le dépôt Kokoro.".to_string())?;

    let voice_name = resolve_kokoro_voice(repo_dir, language)?;
    let voice_pt = repo_dir.join("voices").join(format!("{voice_name}.pt"));
    if !voice_pt.exists() {
        return Err(format!("Voix Kokoro introuvable : {}", voice_pt.display()));
    }
    let lang_code = kokoro_voice_lang(&voice_name).unwrap_or("en");

    let script = format!(
        r#"
import sys
model_path = {model}
voice_pt   = {voice_pt}
out_path   = {out}
voice_name = {voice}
lang_code  = {lang}
speed = {speed}

text = sys.stdin.read()

try:
    from kokoro import KPipeline
    import soundfile as sf
    import numpy as np
    pipeline = KPipeline(lang_code=lang_code[0])
    chunks = []
    for _gs, _ps, audio in pipeline(text, voice=voice_name, speed=speed):
        chunks.append(audio)
    if not chunks:
        print("Kokoro n'a produit aucun segment audio.", file=sys.stderr)
        sys.exit(1)
    sf.write(out_path, np.concatenate(chunks), 24000)
except ImportError:
    try:
        from kokoro_onnx import Kokoro
        import soundfile as sf
        k = Kokoro(model_path, voice_pt)
        audio, sr = k.create(text, voice=voice_name, speed=speed, lang=lang_code)
        sf.write(out_path, audio, sr)
    except ImportError:
        print("Ni `kokoro` ni `kokoro-onnx` n'est installe dans ce Python.", file=sys.stderr)
        sys.exit(1)
"#,
        model = json_str(&weights.to_string_lossy()),
        voice_pt = json_str(&voice_pt.to_string_lossy()),
        out = json_str(&out_file.to_string_lossy()),
        voice = json_str(&voice_name),
        lang = json_str(lang_code),
    );

    run_python(python, &script, text)
        .await
        .map_err(|e| format!("Kokoro a échoué : {e}"))?;
    Ok(voice_name)
}

// ── Qwen3-TTS ───────────────────────────────────────────────────────────────

async fn run_qwen3(
    python: &str,
    repo_dir: &Path,
    text: &str,
    out_file: &Path,
    speed: f32,
    language: &str,
) -> Result<(), String> {
    let dirname = repo_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if dirname.contains("base") {
        return Err(
            "La variante « Base » de Qwen3-TTS exige un enregistrement de référence \
             (clonage vocal), que ce morph ne fait pas. Prenez « CustomVoice », ou Kokoro."
                .into(),
        );
    }

    let script = format!(
        r#"
import sys
repo_dir = {repo}
out_path = {out}
lang  = {lang}
speed = {speed}

text = sys.stdin.read()

try:
    from qwen_tts import Qwen3TTSModel
except ImportError:
    print("Le paquet `qwen-tts` n'est pas installe dans ce Python.", file=sys.stderr)
    sys.exit(1)

import torch
device = "cuda" if torch.cuda.is_available() else "cpu"
dtype  = torch.float16 if device == "cuda" else torch.float32

model = Qwen3TTSModel.from_pretrained(repo_dir, dtype=dtype, device_map=device)
speakers = model.get_supported_speakers()
wavs, sr = model.generate_custom_voice(
    text=text,
    speaker=speakers[0] if speakers else None,
    language=lang,
    temperature=max(0.05, 0.7 + (1.0 - speed) * 0.1),
)

wav = wavs[0] if isinstance(wavs, list) else wavs
if hasattr(wav, "detach"):
    wav = wav.detach().cpu().numpy()
elif hasattr(wav, "numpy"):
    wav = wav.numpy()

import soundfile as sf
sf.write(out_path, wav, sr)
"#,
        repo = json_str(&repo_dir.to_string_lossy()),
        out = json_str(&out_file.to_string_lossy()),
        lang = json_str(language),
    );

    run_python(python, &script, text)
        .await
        .map_err(|e| format!("Qwen3-TTS a échoué : {e}"))
}

// ── Synthèse ────────────────────────────────────────────────────────────────

/// Rendre `text` en WAV et renvoyer son chemin.
pub async fn synthesize_speech(req: TtsRequest) -> Result<TtsResult, String> {
    if req.text.trim().is_empty() {
        return Err("Texte vide : rien à dire.".into());
    }
    let python = find_python()
        .ok_or_else(|| "Python introuvable. Installez Python 3.10 ou plus récent.".to_string())?;

    // Sans choix explicite, la première voix utilisable installée.
    let model = if req.voice.trim().is_empty() {
        list_usable_voices().into_iter().next().ok_or_else(|| {
            "Aucun modèle de synthèse installé. Ajoutez par exemple hexgrad/Kokoro-82M \
             depuis le catalogue de modèles."
                .to_string()
        })?
    } else {
        req.voice.clone()
    };

    let repo_dir = repo_path(&model)
        .ok_or_else(|| format!("Modèle introuvable dans la bibliothèque : {model}"))?;

    let out_dir = req.output_dir.clone().unwrap_or_else(default_output_dir);
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("Impossible de créer {} : {e}", out_dir.display()))?;
    let out_file = out_dir.join(format!("tts_{}.wav", unix_millis()));

    let speed = req.speed.clamp(0.5, 2.0);
    let language = req
        .language
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| detect_language(&req.text).to_string());

    let voice = match resolve_engine(&model)? {
        Engine::Kokoro => {
            Some(run_kokoro(&python, &repo_dir, &req.text, &out_file, speed, &language).await?)
        }
        Engine::Qwen3 => {
            run_qwen3(&python, &repo_dir, &req.text, &out_file, speed, &language).await?;
            None
        }
    };

    if !out_file.exists() {
        return Err("Le moteur s'est terminé sans écrire de fichier audio.".into());
    }
    let duration_seconds = wav_duration_seconds(&out_file).unwrap_or(0.0);

    Ok(TtsResult {
        audio_path: out_file,
        duration_seconds,
        model,
        voice,
        language,
    })
}

// ── Utilitaires ─────────────────────────────────────────────────────────────

fn unix_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Un littéral Python sûr : on passe par JSON, dont la syntaxe de chaîne est
/// compatible. Sans cela, un chemin Windows glisse ses antislashs dans le
/// script.
fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// La durée d'un WAV, lue dans son en-tête. Sans en-tête exploitable, `None`
/// plutôt qu'une estimation inventée.
fn wav_duration_seconds(path: &Path) -> Option<f32> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let u32_at = |o: usize| -> Option<u32> {
        Some(u32::from_le_bytes(bytes.get(o..o + 4)?.try_into().ok()?))
    };
    let byte_rate = u32_at(28)?;
    if byte_rate == 0 {
        return None;
    }
    // Parcourir les blocs jusqu'à `data` : un WAV écrit par soundfile peut
    // intercaler d'autres blocs avant lui.
    let mut pos = 12usize;
    while pos + 8 <= bytes.len() {
        let id = bytes.get(pos..pos + 4)?;
        let size = u32_at(pos + 4)? as usize;
        if id == b"data" {
            return Some(size as f32 / byte_rate as f32);
        }
        pos += 8 + size + (size & 1);
    }
    None
}

/// Fichiers sous `dir`, jusqu'à `max_depth` niveaux.
pub fn walkdir_recursive(dir: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if max_depth == 0 {
        return out;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                out.extend(walkdir_recursive(&p, max_depth - 1));
            } else if p.is_file() {
                out.push(p);
            }
        }
    }
    out
}

/// Le Python à utiliser : l'environnement géré par l'application d'abord, puis
/// celui du PATH, puis une installation par utilisateur sous Windows.
pub fn find_python() -> Option<String> {
    for venv in python_venv_candidates() {
        let exe = if cfg!(windows) {
            venv.join("Scripts").join("python.exe")
        } else {
            venv.join("bin").join("python")
        };
        if exe.exists() {
            return Some(exe.to_string_lossy().to_string());
        }
    }
    if let Ok(out) = std::process::Command::new("python")
        .arg("--version")
        .output()
    {
        if out.status.success() {
            return Some("python".to_string());
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if let Ok(entries) = std::fs::read_dir(Path::new(&local).join("Programs").join("Python")) {
            for entry in entries.flatten() {
                let exe = entry.path().join("python.exe");
                if exe.exists() {
                    return Some(exe.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn python_venv_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(v) = std::env::var("LOCARYN_PYTHON_VENV") {
        if !v.trim().is_empty() {
            out.push(PathBuf::from(v));
        }
    }
    for dir in models_dirs() {
        if let Some(parent) = dir.parent() {
            out.push(parent.join("python-env"));
            out.push(parent.join(".venv"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join(".venv"));
    }
    out
}

/// L'environnement que doit hériter tout Python lancé ici.
///
/// `transformers` tire TensorFlow uniquement pour deviner un moteur qu'on
/// n'utilise pas — vingt secondes par appel. Et les téléchargements
/// HuggingFace vont par défaut dans `~/.cache`, ce qui remplit le disque
/// système. L'hôte nous dit où poser tout ça ; on le respecte.
fn python_env() -> Vec<(&'static str, String)> {
    let mut env = vec![
        ("TRANSFORMERS_NO_TF", "1".to_string()),
        ("USE_TF", "0".to_string()),
        ("TF_CPP_MIN_LOG_LEVEL", "3".to_string()),
    ];
    if let Ok(hf) = std::env::var("LOCARYN_HF_CACHE_DIR") {
        if !hf.trim().is_empty() {
            let _ = std::fs::create_dir_all(&hf);
            env.push(("HF_HOME", hf));
        }
    }
    if let Ok(tmp) = std::env::var("LOCARYN_TEMP_DIR") {
        if !tmp.trim().is_empty() {
            let _ = std::fs::create_dir_all(&tmp);
            env.push(("TMPDIR", tmp.clone()));
            env.push(("TEMP", tmp.clone()));
            env.push(("TMP", tmp));
        }
    }
    env
}

/// Lancer `python -c <script>`, lui donner le texte sur l'entrée standard.
///
/// En cas d'échec on rend la fin de la sortie d'erreur : une trace Python
/// complète noie le message utile, mais ses dernières lignes le portent.
async fn run_python(python: &str, script: &str, text: &str) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;

    let mut child = tokio::process::Command::new(python)
        .envs(python_env())
        .arg("-c")
        .arg(script)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("impossible de lancer Python : {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|e| format!("écriture sur l'entrée de Python : {e}"))?;
        stdin
            .shutdown()
            .await
            .map_err(|e| format!("fermeture de l'entrée de Python : {e}"))?;
    }

    let out = child
        .wait_with_output()
        .await
        .map_err(|e| format!("attente de Python : {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = err.lines().filter(|l| !l.trim().is_empty()).collect();
    let msg = lines
        .iter()
        .rev()
        .take(3)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join(" / ");
    Err(if msg.is_empty() {
        format!("code de sortie {}", out.status)
    } else {
        msg
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_langue_se_devine_sur_les_ecritures_et_les_accents() {
        assert_eq!(detect_language("hello there"), "en");
        assert_eq!(detect_language("où est la clé"), "fr");
        assert_eq!(detect_language("grüße dich"), "de");
        assert_eq!(detect_language("こんにちは"), "ja");
        assert_eq!(detect_language("你好世界"), "zh");
        assert_eq!(detect_language("привет"), "ru");
    }

    #[test]
    fn le_prefixe_d_une_voix_kokoro_donne_sa_langue() {
        assert_eq!(kokoro_voice_lang("af_heart"), Some("en"));
        assert_eq!(kokoro_voice_lang("ff_siwis"), Some("fr"));
        assert_eq!(kokoro_voice_lang("jf_alpha"), Some("ja"));
        assert_eq!(kokoro_voice_lang("_bizarre"), None);
    }

    /// La variante « Base » ne sait que cloner : elle ne doit jamais être le
    /// choix par défaut, sinon toute demande sans voix échoue.
    #[test]
    fn la_variante_base_n_est_pas_une_voix_autonome() {
        assert!(!has_own_voice("Qwen__Qwen3-TTS-Base"));
        assert!(has_own_voice("Qwen__Qwen3-TTS-CustomVoice"));
        assert!(has_own_voice("hexgrad__Kokoro-82M"));
    }

    #[test]
    fn le_moteur_se_deduit_du_nom_du_depot() {
        assert!(matches!(
            resolve_engine("hexgrad__Kokoro-82M"),
            Ok(Engine::Kokoro)
        ));
        assert!(matches!(
            resolve_engine("Qwen__Qwen3-TTS-CustomVoice"),
            Ok(Engine::Qwen3)
        ));
        assert!(resolve_engine("llama-3-8b").is_err());
    }

    /// Un chemin Windows contient des antislashs : injecté brut dans le script
    /// Python, il produirait des séquences d'échappement.
    #[test]
    fn les_chemins_partent_en_litteraux_python_surs() {
        assert_eq!(json_str(r"C:\models\a.pth"), r#""C:\\models\\a.pth""#);
    }

    #[test]
    fn un_texte_vide_est_refuse_sans_lancer_python() {
        let req = TtsRequest {
            text: "   ".into(),
            voice: String::new(),
            speed: 1.0,
            language: None,
            output_dir: None,
        };
        let err = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(synthesize_speech(req))
            .unwrap_err();
        assert!(err.contains("vide"), "{err}");
    }

    /// Une vraie synthèse, de bout en bout, sur la machine qui exécute le
    /// test. Ignoré par défaut : il exige un modèle installé et un Python
    /// équipé. Lancer avec
    /// `cargo test -- --ignored --nocapture`.
    #[test]
    #[ignore = "exige un modèle TTS installé et un Python équipé"]
    fn synthetise_reellement_un_wav_audible() {
        let voix = list_usable_voices();
        assert!(
            !voix.is_empty(),
            "aucun modèle TTS installé ; rien à éprouver"
        );
        println!("voix utilisables : {voix:?}");

        let sortie = std::env::temp_dir().join("morph-voice-tts-epreuve");
        let req = TtsRequest {
            text: "Bonjour, ceci est une synthèse vocale produite localement.".into(),
            voice: String::new(),
            speed: 1.0,
            language: None,
            output_dir: Some(sortie.clone()),
        };

        let res = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(synthesize_speech(req))
            .expect("la synthèse doit aboutir");

        println!(
            "produit : {} — {:.2}s, modèle {}, voix {:?}, langue {}",
            res.audio_path.display(),
            res.duration_seconds,
            res.model,
            res.voice,
            res.language
        );
        assert!(res.audio_path.is_file(), "aucun fichier écrit");
        let taille = std::fs::metadata(&res.audio_path).unwrap().len();
        // Un en-tête WAV seul fait 44 octets : en dessous d'un kilo-octet, il
        // n'y a pas de son, seulement un fichier.
        assert!(taille > 1024, "fichier trop court : {taille} octets");
        assert!(res.duration_seconds > 0.3, "durée {}", res.duration_seconds);
        assert_eq!(res.language, "fr", "la langue aurait dû se deviner");
        let _ = std::fs::remove_dir_all(&sortie);
    }

    #[test]
    fn la_duree_se_lit_dans_l_en_tete_du_wav() {
        // 8000 octets par seconde, 16000 octets de données = 2 s.
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&0u32.to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes()); // PCM
        w.extend_from_slice(&1u16.to_le_bytes()); // mono
        w.extend_from_slice(&8000u32.to_le_bytes()); // taux
        w.extend_from_slice(&8000u32.to_le_bytes()); // octets/s
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&8u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&16000u32.to_le_bytes());
        w.extend_from_slice(&vec![0u8; 16000]);

        let p = std::env::temp_dir().join("morph-voice-tts-duree.wav");
        std::fs::write(&p, &w).unwrap();
        let d = wav_duration_seconds(&p).expect("durée lisible");
        assert!((d - 2.0).abs() < 0.01, "durée = {d}");
        let _ = std::fs::remove_file(&p);
    }
}
