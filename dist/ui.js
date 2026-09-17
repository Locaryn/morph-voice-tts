(function () {
  "use strict";

  const CSS = `
:host { display: block; width: 100%; color: var(--text, #e8edf5); font-family: inherit; box-sizing: border-box; }
* { box-sizing: border-box; }
.panel-container { width: 100%; max-width: 920px; margin: 0 auto; display: flex; flex-direction: column; gap: 16px; }
.header-card {
  display: flex; align-items: center; justify-content: space-between; padding: 16px 20px;
  background: var(--surface, rgba(255, 255, 255, 0.035)); border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: var(--radius, 12px);
}
.title-wrap { display: flex; align-items: center; gap: 12px; }
.icon-box {
  width: 40px; height: 40px; border-radius: 10px; background: rgba(var(--accent-rgb, 110, 168, 254), 0.15);
  color: var(--accent, #6ea8fe); display: grid; place-items: center; font-size: 20px;
}
.title { font-size: 16px; font-weight: 700; color: var(--text, #e8edf5); }
.subtitle { font-size: 12px; color: var(--text-faint, #96a3b8); margin-top: 2px; }
.badge {
  display: inline-flex; align-items: center; padding: 4px 10px; border-radius: 99px; font-size: 11px;
  font-weight: 600; background: rgba(101, 211, 145, 0.12); color: #65d391; border: 1px solid rgba(101, 211, 145, 0.25);
}
.field-card {
  display: flex; flex-direction: column; gap: 10px; background: var(--surface, rgba(255, 255, 255, 0.035));
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1)); border-radius: var(--radius, 12px); padding: 16px;
}
.label { font-size: 11px; font-weight: 700; color: var(--text-dim, #94a3b8); text-transform: uppercase; letter-spacing: 0.06em; }
.textarea, .select, .input {
  width: 100%; border: 1px solid var(--border, rgba(255, 255, 255, 0.14)); border-radius: var(--radius-sm, 8px);
  background: var(--bg, rgba(0, 0, 0, 0.25)); color: inherit; padding: 10px 12px; font: inherit; font-size: 13px; outline: none;
}
.textarea { min-height: 90px; resize: vertical; }
.select:disabled, .input:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-primary {
  width: 100%; padding: 12px; background: var(--accent, #6ea8fe); color: #0b101b; border: none;
  border-radius: var(--radius-sm, 8px); font-weight: 700; font-size: 14px; cursor: pointer;
}
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.error-card {
  padding: 14px 16px; background: rgba(240, 100, 100, 0.08); border: 1px solid rgba(240, 100, 100, 0.25);
  border-radius: var(--radius, 12px); color: #f08a8a; font-size: 13px; line-height: 1.5;
}
.hint-card {
  padding: 14px 16px; background: var(--surface, rgba(255, 255, 255, 0.035)); border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: var(--radius, 12px); color: var(--text-faint, #96a3b8); font-size: 13px; line-height: 1.5;
}
.result-card {
  padding: 16px; background: var(--surface, rgba(255, 255, 255, 0.035)); border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: var(--radius, 12px); display: flex; flex-direction: column; gap: 10px;
}
.result-meta { font-size: 12px; color: var(--text-dim, #94a3b8); }
audio { width: 100%; }
`;

  class LocarynVoiceTtsPanel extends HTMLElement {
    constructor() {
      super();
      this.attachShadow({ mode: "open" });
      this.text = "";
      this.voice = "";
      this.speed = 1.0;
      this.isSpeaking = false;
      this.error = null;
      this.result = null;
      this.audioUrl = null;
      this.voices = null; // null = pas encore chargé, [] = aucun installé
      this.voicesError = null;
    }

    connectedCallback() {
      this.render();
      void this.loadVoices();
    }

    bridge() {
      return window.locaryn || window.LocarynPluginAPI || null;
    }

    async loadVoices() {
      try {
        const bridge = this.bridge();
        if (!bridge || !bridge.invokeExtensionTool) {
          throw new Error("Le pont d'extension n'est pas disponible dans ce contexte.");
        }
        const res = await bridge.invokeExtensionTool("list_voices", {});
        const parsed = typeof res === "string" ? JSON.parse(res) : res;
        this.voices = Array.isArray(parsed && parsed.usable) ? parsed.usable : [];
        if (this.voices.length > 0) {
          this.voice = this.voices[0];
        }
      } catch (err) {
        this.voices = [];
        this.voicesError = err && err.message ? err.message : String(err);
      } finally {
        this.render();
      }
    }

    async speak() {
      if (!this.text.trim() || this.isSpeaking || !this.voice) return;
      this.isSpeaking = true;
      this.error = null;
      this.result = null;
      this.audioUrl = null;
      this.render();
      try {
        const bridge = this.bridge();
        if (!bridge || !bridge.invokeExtensionTool) {
          throw new Error("Le pont d'extension n'est pas disponible dans ce contexte.");
        }
        const res = await bridge.invokeExtensionTool("synthesize_speech", {
          text: this.text,
          voice: this.voice,
          speed: Number(this.speed)
        });
        const parsed = typeof res === "string" ? JSON.parse(res) : res;
        if (!parsed || !parsed.audio_path) {
          throw new Error("Le moteur n'a produit aucun fichier audio.");
        }
        this.result = parsed;
        this.audioUrl = bridge.files ? bridge.files.assetUrl(parsed.audio_path) : parsed.audio_path;
      } catch (err) {
        this.error = err && err.message ? err.message : String(err);
      } finally {
        this.isSpeaking = false;
        this.render();
      }
    }

    render() {
      const chargement = this.voices === null;
      const aucuneVoix = this.voices !== null && this.voices.length === 0;
      const peutParler = !this.isSpeaking && this.text.trim() && this.voice && !aucuneVoix;

      this.shadowRoot.innerHTML = `
        <style>${CSS}</style>
        <div class="panel-container">
          <div class="header-card">
            <div class="title-wrap">
              <div class="icon-box">🗣️</div>
              <div>
                <div class="title">Studio Synthèse Vocale</div>
                <div class="subtitle">Kokoro ou Qwen3-TTS, selon ce qui est installé sur cette machine</div>
              </div>
            </div>
            <div class="badge">Actif</div>
          </div>

          <div class="field-card">
            <label class="label">Texte à vocaliser</label>
            <textarea class="textarea" id="tts-text" placeholder="Entrez le texte à faire lire par la voix synthétique...">${this.text}</textarea>
          </div>

          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
            <div class="field-card">
              <label class="label">Modèle de voix</label>
              <select class="select" id="tts-voice" ${chargement || aucuneVoix ? "disabled" : ""}>
                ${
                  chargement
                    ? `<option>Chargement…</option>`
                    : aucuneVoix
                      ? `<option>Aucun modèle installé</option>`
                      : this.voices
                          .map((v) => `<option value="${v}" ${v === this.voice ? "selected" : ""}>${v}</option>`)
                          .join("")
                }
              </select>
            </div>
            <div class="field-card">
              <label class="label">Débit (${this.speed}x)</label>
              <input class="input" type="number" step="0.1" min="0.5" max="2.0" id="tts-speed" value="${this.speed}" />
            </div>
          </div>

          ${
            aucuneVoix
              ? `<div class="hint-card">
                  Aucun modèle de synthèse installé. Ajoutez par exemple hexgrad/Kokoro-82M
                  depuis le catalogue de modèles, puis revenez ici.
                  ${this.voicesError ? `<br><br>${this.voicesError}` : ""}
                </div>`
              : ""
          }

          <button class="btn-primary" id="tts-btn" ${peutParler ? "" : "disabled"}>
            ${this.isSpeaking ? "Génération audio en cours..." : "Lire le texte à voix haute"}
          </button>

          ${this.error ? `<div class="error-card">${this.error}</div>` : ""}

          ${
            this.result && this.audioUrl
              ? `<div class="result-card">
                  <audio controls autoplay src="${this.audioUrl}"></audio>
                  <div class="result-meta">
                    ${this.result.model}${this.result.voice ? " · " + this.result.voice : ""} ·
                    ${this.result.language} · ${Number(this.result.duration_seconds || 0).toFixed(1)}s
                  </div>
                </div>`
              : ""
          }
        </div>
      `;

      const textEl = this.shadowRoot.querySelector("#tts-text");
      if (textEl) {
        textEl.addEventListener("input", (e) => {
          this.text = e.target.value;
          const btn = this.shadowRoot.querySelector("#tts-btn");
          if (btn) btn.disabled = !(this.text.trim() && this.voice && !this.isSpeaking);
        });
      }

      const voiceEl = this.shadowRoot.querySelector("#tts-voice");
      if (voiceEl && !chargement && !aucuneVoix) {
        voiceEl.addEventListener("change", (e) => {
          this.voice = e.target.value;
        });
      }

      const speedEl = this.shadowRoot.querySelector("#tts-speed");
      if (speedEl) {
        speedEl.addEventListener("input", (e) => {
          this.speed = Number(e.target.value);
        });
      }

      const btn = this.shadowRoot.querySelector("#tts-btn");
      if (btn) {
        btn.addEventListener("click", () => this.speak());
      }
    }
  }

  if (!customElements.get("locaryn-voice-tts-panel")) {
    customElements.define("locaryn-voice-tts-panel", LocarynVoiceTtsPanel);
  }
})();
