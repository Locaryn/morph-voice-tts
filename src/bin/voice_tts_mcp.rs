//! Stdio MCP server shipped by morph-voice-tts.
use locaryn_plugin_voice_tts::{
    has_own_voice, kokoro_voices_in_repo, list_usable_voices, list_voices, repo_path,
    synthesize_speech, TtsRequest,
};
use serde_json::{json, Value};
use std::io::Write;
use tokio::io::{AsyncBufReadExt, BufReader};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle_request(request).await,
            Err(error) => error_response(Value::Null, -32700, format!("JSON invalide : {error}")),
        };
        if let Ok(serialized) = serde_json::to_string(&response) {
            println!("{serialized}");
            let _ = std::io::stdout().flush();
        }
    }
}

async fn handle_request(request: Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match method {
        "initialize" => success(
            id,
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "morph-voice-tts", "version": VERSION }
            }),
        ),
        "tools/list" => success(id, tools_list()),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match call_tool(name, args).await {
                Ok(value) => success(id, text_content(value)),
                Err(error) => error_response(id, -32000, error),
            }
        }
        notification if notification.starts_with("notifications/") => Value::Null,
        _ => error_response(id, -32601, format!("méthode MCP inconnue : {method}")),
    }
}

fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "list_voices",
                "description": "Les modèles de synthèse installés. `usable` ne garde que ceux qui \
                                savent parler sans enregistrement de référence : c'est la liste à \
                                proposer. Les dépôts Kokoro donnent aussi leurs voix internes.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "synthesize_speech",
                "description": "Lit un texte à voix haute et écrit un WAV sur cette machine. Rend le \
                                chemin du fichier, sa durée, le modèle et la voix retenus.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "Texte à lire à voix haute" },
                        "voice": {
                            "type": "string",
                            "description": "Modèle à employer, tel que rendu par `list_voices`. Omis : le premier modèle utilisable installé."
                        },
                        "speed": { "type": "number", "description": "Vitesse d'élocution, de 0.5 à 2.0 (défaut 1.0)" },
                        "language": {
                            "type": "string",
                            "description": "Code ISO de la langue (fr, en, ja…). Omis : deviné d'après le texte."
                        }
                    },
                    "required": ["text"]
                }
            }
        ]
    })
}

async fn call_tool(name: &str, args: Value) -> Result<Value, String> {
    match name {
        "list_voices" => {
            // Le détail des voix internes évite un aller-retour : sans lui, le
            // modèle ne peut ni nommer une voix ni savoir quelles langues le
            // dépôt couvre réellement.
            let details: Vec<Value> = list_voices()
                .into_iter()
                .map(|nom| {
                    let voix = repo_path(&nom)
                        .map(|p| kokoro_voices_in_repo(&p))
                        .unwrap_or_default();
                    json!({
                        "name": nom,
                        "usable": has_own_voice(&nom),
                        "inner_voices": voix,
                    })
                })
                .collect();
            Ok(json!({
                "models": details,
                "usable": list_usable_voices(),
            }))
        }
        "synthesize_speech" => {
            let req: TtsRequest = serde_json::from_value(args)
                .map_err(|e| format!("Paramètres TTS invalides: {e}"))?;
            let res = synthesize_speech(req).await?;
            Ok(json!(res))
        }
        _ => Err(format!("Outil TTS inconnu : {name}")),
    }
}

fn text_content(value: Value) -> Value {
    json!({ "content": [{ "type": "text", "text": serde_json::to_string(&value).unwrap_or_else(|_| "{}".into()) }] })
}
fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}
fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
