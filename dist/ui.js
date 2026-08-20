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
.btn-primary {
  width: 100%; padding: 12px; background: var(--accent, #6ea8fe); color: #0b101b; border: none;
  border-radius: var(--radius-sm, 8px); font-weight: 700; font-size: 14px; cursor: pointer;
}
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
`;

  class LocarynVoiceTtsPanel extends HTMLElement {
    constructor() {
      super();
      this.attachShadow({ mode: "open" });
      this.text = "";
      this.voice = "fr-siwis";
      this.speed = 1.0;
      this.isSpeaking = false;
    }
    connectedCallback() { this.render(); }

    async speak() {
      if (!this.text.trim() || this.isSpeaking) return;
      this.isSpeaking = true;
      this.render();
      try {
        const bridge = window.locaryn || window.LocarynPluginAPI;
        if (bridge && bridge.invokeExtensionTool) {
          await bridge.invokeExtensionTool("synthesize_speech", {
            text: this.text,
            voice: this.voice,
            speed: Number(this.speed)
          });
        }
      } catch (err) {
        alert("Erreur de synthèse vocale: " + err);
      } finally {
        this.isSpeaking = false;
        this.render();
      }
    }

    render() {
      this.shadowRoot.innerHTML = `
        <style>${CSS}</style>
        <div class="panel-container">
          <div class="header-card">
            <div class="title-wrap">
              <div class="icon-box">🗣️</div>
              <div>
                <div class="title">Studio Synthèse Vocale</div>
                <div class="subtitle">Lecture audio haute fidélité via Kokoro & Piper</div>
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
              <label class="label">Profil de Voix</label>
              <select class="select" id="tts-voice">
                <option value="fr-siwis">Français (Naturel / Studio)</option>
                <option value="en-us-kokoro">English (Kokoro Neural)</option>
                <option value="fr-narrator">Français (Documentaire / Grave)</option>
              </select>
            </div>
            <div class="field-card">
              <label class="label">Débit (${this.speed}x)</label>
              <input class="input" type="number" step="0.1" min="0.5" max="2.0" id="tts-speed" value="${this.speed}" />
            </div>
          </div>

          <button class="btn-primary" id="tts-btn" ${this.isSpeaking || !this.text.trim() ? "disabled" : ""}>
            ${this.isSpeaking ? "Génération audio en cours..." : "Lire le texte à voix haute"}
          </button>
        </div>
      `;

      const textEl = this.shadowRoot.querySelector("#tts-text");
      if (textEl) {
        textEl.addEventListener("input", (e) => {
          this.text = e.target.value;
          const btn = this.shadowRoot.querySelector("#tts-btn");
          if (btn) btn.disabled = !this.text.trim() || this.isSpeaking;
        });
      }

      const voiceEl = this.shadowRoot.querySelector("#tts-voice");
      if (voiceEl) {
        voiceEl.addEventListener("change", (e) => { this.voice = e.target.value; });
      }

      const speedEl = this.shadowRoot.querySelector("#tts-speed");
      if (speedEl) {
        speedEl.addEventListener("input", (e) => { this.speed = Number(e.target.value); });
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
