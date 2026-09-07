---
name: voice-tts
description: Transformer du texte en voix parlée naturelle (TTS) et cloner des voix depuis un échantillon audio.
---

# Compétence Synthèse Vocale

## Lire un texte à voix haute

`synthesize_speech` avec le `text` suffit : la langue se devine du texte, et le
premier modèle utilisable installé parle. `list_voices` donne les modèles
installés — le champ `usable` ne garde que ceux qui savent parler **sans**
enregistrement de référence, et c'est cette liste-là qu'il faut proposer.

Un dépôt Kokoro apporte ses propres voix internes (`inner_voices`) ; Qwen3-TTS
en variante « CustomVoice » parle avec ses locuteurs livrés.

## Cloner une voix

Le clonage demande un enregistrement de référence et, idéalement, **sa
transcription**. C'est la transcription qui fait la différence : avec elle, le
moteur entend *comment* la phrase est dite et reproduit cette diction. Sans
elle, seul le timbre est repris et le débit reste plat.

Deux façons de procéder.

**Par préréglage — la voie normale.** `save_voice_preset` enregistre une fois
l'échantillon, sa transcription et les réglages de diction ; l'audio est
**copié** dans le préréglage, si bien qu'il survit au rangement du dossier
d'origine. Ensuite, chaque `synthesize_speech` passe simplement `preset` avec
l'identifiant rendu. `list_voice_presets` les liste, `delete_voice_preset` en
retire un.

**Par référence directe.** Pour un clonage unique, `reference_audio` et
`reference_text` se donnent à `synthesize_speech` sans rien enregistrer.

Le clonage exige un modèle **Qwen3-TTS**. La variante « Base » ne sait *que*
cloner : sans référence, elle refuse et le dit. Kokoro, lui, ne clone pas — il
parle avec ses voix livrées, et une référence qui lui serait donnée est refusée
plutôt qu'ignorée en silence.

`voice_preset_support` dit ce qu'un moteur donné honorera réellement d'un
préréglage, plutôt que d'en ignorer la moitié sans le signaler.

## Ce qui reste sur la machine

Tout. Le texte entre, un WAV sort, les poids ne bougent pas, et les
échantillons de voix vivent dans le dossier de données que l'hôte attribue au
morph.
