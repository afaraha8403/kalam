# Kalam vs SuperWhisper — Feature Comparison

**Date:** March 29, 2026
**Companion doc:** `.doc/superwhisper-competitor-review.md` (full SuperWhisper research)
**Purpose:** Side-by-side comparison of what Kalam has built today vs what SuperWhisper ships, identifying parity, advantages, and gaps.

---

## 1. Product-Level Comparison

| Dimension | Kalam (current) | SuperWhisper |
|---|---|---|
| **Platforms** | Windows (primary), macOS & Linux planned | macOS (primary), Windows, iOS |
| **Pricing** | Free / open-source / BYOK | Free tier (limited) + Pro subscription |
| **Architecture** | Tauri v2 (Rust + Svelte) | Native macOS app (proprietary) |
| **Idle RAM** | ~10-30 MB target | Not disclosed (likely higher) |
| **Bundle size** | ~15-20 MB (without models) | Not disclosed |
| **Open source** | Yes (100%) | No |
| **Offline capable** | Yes (SenseVoice / Whisper.cpp local) | Yes (local Whisper models) |
| **BYOK** | Yes (Groq, OpenAI keys) | Yes (own API keys supported) |
| **Enterprise** | Planned (Phase 4) | Yes (SAML SSO, SCIM, seat billing) |

---

## 2. Core Dictation Pipeline

### STT (Speech-to-Text)

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Cloud STT providers** | Groq (whisper-large-v3-turbo), OpenAI (whisper-1) | Proprietary S1-Voice, Ultra, Deepgram Nova 2/3/Medical | Kalam has 2 cloud providers; SW has 5+ |
| **Local STT models** | SenseVoice-Small (200MB), Whisper.cpp base (142MB) | Whisper variants (75MB–3GB), Nvidia Parakeet (476MB) | Both offer local. SW has more model tiers. |
| **VAD** | Silero VAD (Rust port) with 3 presets (Fast/Balanced/Accurate) | Not documented publicly | Kalam's VAD is well-implemented |
| **Audio chunking** | VAD-based + hard fallback at 30s + overlap + prompt chaining | Not documented | Kalam has a solid chunking pipeline |
| **Audio filters** | Highpass, noise gate, compressor, normalize (preset: Off/Light/Moderate/Custom) | Mute audio, pause media, record system audio | Different focus: Kalam = input quality; SW = recording context |
| **Language support** | Configurable language list, toggle hotkey for quick switch | 100+ languages, per-mode language setting | SW has broader language UX |
| **Model lifecycle** | Full: download, install, start, stop, restart, delete, sidecar management | Download local models from Advanced Settings | Kalam has richer model management |
| **Silence removal** | VAD handles this (min speech duration, silence timeout) | Explicit "Remove Silence" toggle in Sound settings | Functionally equivalent |
| **Hallucination mitigation** | VAD min-speech-duration filters coughs/noise; prompt chaining for context | Silence removal + vocabulary pruning + history debug | SW has explicit hallucination debugging UX |

### Post-Processing (Formatting)

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Filler word removal** | Yes (regex-based: um, uh, er, ah, like, you know, etc.) | Yes (AI-based in Message/Super modes) | Kalam = deterministic regex; SW = LLM |
| **Auto-punctuation** | Basic heuristic (append `.` if missing ending punct) | LLM-based in all AI modes | SW's is smarter (LLM); Kalam's is minimal |
| **Voice commands** | Yes: punctuation (period, comma, etc.), new line, tab, undo, delete that, scratch that | Not documented as explicit voice commands | **Kalam advantage** — explicit voice command system |
| **Custom formatting rules** | Yes: regex or literal find/replace, per-rule enable/disable | Replacements (case-insensitive find/replace, post-transcription) | Kalam's regex support is more powerful |
| **Smart self-correction** | No | Yes (LLM detects mid-sentence corrections, removes false starts) | Gap — requires LLM post-processing |
| **Grammar/spelling fix** | No (beyond filler removal) | Yes (LLM-based in Message/Super modes) | Gap — requires LLM post-processing |

### Text Injection

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Injection method** | Dual: keystrokes (enigo) + clipboard, auto-select by length | Presumably clipboard/keystroke (not documented) | Kalam has explicit dual-strategy |
| **Retry mechanism** | Yes (configurable attempts + delay) | Not documented | Kalam advantage |
| **Force-clipboard apps** | Yes (per-app override list) | Not documented | Kalam advantage |
| **Keystroke delay** | Configurable (ms) | Not documented | Kalam advantage |

---

## 3. AI / LLM Processing

This is the **biggest gap** between the two products.

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **LLM post-processing of dictation** | No (STT output is the final output) | Yes — every mode except Voice runs transcript through an LLM | **Fundamental architectural difference** |
| **Command Mode (separate)** | Yes — separate hotkey triggers LLM processing via `command_config` (Groq, OpenRouter, OpenAI, Anthropic, Gemini) | N/A (LLM is integrated into every mode) | Kalam has the LLM infra but it's a separate flow |
| **LLM providers available** | Groq, OpenRouter, OpenAI, Anthropic, Gemini (for command mode) | Proprietary S1-Language, Claude 3.5–4.5, GPT-4.1–5, Llama 3 8b (Groq) | Both have multi-provider LLM support |
| **LLM model selection** | `fetch_llm_models` + `test_llm_model` in Settings | Per-mode model selection in Advanced Settings | Both let users pick models |
| **Context-aware processing** | No — LLM doesn't receive app context, selected text, or clipboard | Yes — Super Mode sends all three context types to LLM | **Major gap** |
| **Voice-activated editing** | No | Yes — highlight text → speak command → AI edits in-place | **Major gap** |
| **Structured data generation** | Yes — `generate_structured_data` command exists | Not documented as a feature | Kalam has this infra |

### Kalam's Command Mode vs SuperWhisper's Modes

Kalam already has the **building blocks** for LLM-powered dictation:
- `CommandConfig` with provider, API keys, model selection
- `fetch_llm_models` to list available models from 5 providers
- `generate_structured_data` for LLM calls
- `test_llm_model` for validation
- Separate hotkey for command mode (`command_config.hotkey`)
- `RecordingType` enum distinguishes `Dictation` vs `Command`

What's missing is **integrating this into the dictation pipeline** — currently command mode is a separate flow, not a post-processing step on regular dictation.

---

## 4. Modes / Profiles System

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Named processing profiles** | No — single global config | Yes — 7 built-in + unlimited custom modes | **Major gap** |
| **Per-mode STT model** | No | Yes | Gap |
| **Per-mode LLM model** | No | Yes | Gap |
| **Per-mode AI instructions** | No | Yes (Custom Mode: free-form system prompt) | Gap |
| **Per-mode context toggles** | No | Yes (app context, selected text, clipboard — each toggleable) | Gap |
| **Per-mode audio settings** | No | Yes (mute audio, pause media, system audio, speaker ID) | Gap |
| **Per-mode language** | No (global language list) | Yes | Gap |
| **Mode switching** | N/A | Hotkey cycle, menu bar, auto-activation, deep links | Gap |
| **Auto-activation rules** | Sensitive app detection → ForceLocal (privacy only) | Per-mode rules: app/website → switch mode | Kalam has the app-detection infra but uses it only for privacy |
| **Built-in mode presets** | N/A | Voice, Message, Email, Note, Meeting, Super | Gap |
| **Custom mode creation** | N/A | Yes (AI instructions + context toggles + examples) | Gap |
| **Customizing built-in modes** | N/A | Yes (access underlying AI instructions, modify) | Gap |

---

## 5. Context Awareness

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Active app detection** | Yes — captures `target_app` process name + resolves display name + icon | Yes — accessibility APIs read active app + input field content | Kalam captures the app; SW also reads the app's text content |
| **Read input field content** | No | Yes (Application Context via accessibility APIs) | Gap |
| **Read selected text** | No | Yes (Selected Text Context) | Gap |
| **Read clipboard** | No | Yes (Clipboard Context — within 3s window) | Gap |
| **System metadata** | No | Yes (date, time, user name, computer name injected) | Gap |
| **Context display in UI** | No — overlay shows waveform only | Yes — recording window shows which app's context is active | Gap |
| **Sensitive app detection** | Yes — regex patterns on process name / window title → ForceLocal | Yes — guidance for regulated data, enterprise controls | **Kalam advantage** — more granular pattern matching |

---

## 6. Vocabulary & Dictionary

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Custom dictionary** | Yes — `DictionaryEntry` CRUD (term + created_at) in SQLite | Yes — Vocabulary Words (hints sent to STT model) | Different implementation |
| **Dictionary sent as STT hints** | Partially — `build_transcription_vocabulary_prompt` builds a prompt from dictionary entries | Yes — words sent alongside audio to STT model | Kalam does build a vocabulary prompt for STT |
| **Prompt leakage sanitization** | Yes — `sanitize_prompt_leakage` removes dictionary terms that leak into output | Not documented | **Kalam advantage** |
| **Replacement rules** | Yes — `FormattingRule` with regex/literal, per-rule enable, case-insensitive | Yes — Replacements (case-insensitive find/replace) | Kalam's regex support is more powerful |
| **Clipboard as ephemeral vocab** | No | Yes — copy a term before dictating, AI uses it for spelling | Gap |

---

## 7. History & Data

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Transcription history** | Yes — SQLite with full metadata (app, language, duration, STT mode/provider, latency, word count) | Yes — History tab with segments view | Both have history |
| **Search** | Yes — `search_history` | Yes | Parity |
| **Raw vs AI-processed comparison** | No — only final text stored | Yes — shows both raw transcription and AI-processed result | Gap (useful for debugging) |
| **Export** | Yes — JSON/CSV | Not documented | Kalam advantage |
| **Retention policy** | Yes — configurable days, auto-prune | Not documented | Kalam advantage |
| **History encryption** | Yes — AES-256 at rest (legacy path) | Not documented | Kalam advantage |
| **Speaker separation** | No | Yes (Deepgram Nova models, segments view) | Gap |
| **Save as note/task** | Yes — history entry → create note or task | No | **Kalam advantage** |

---

## 8. Notes, Tasks & Reminders

This is a **Kalam-only feature area** — SuperWhisper has no equivalent.

| Feature | Kalam | SuperWhisper |
|---|---|---|
| **Notes** | Yes — full CRUD, Tiptap rich text editor, colors, tags, pinning, manual reorder, archive/trash | No |
| **Tasks** | Yes — full CRUD, subtasks, priority, due dates, completion, tags, manual reorder, archive/trash | No |
| **Reminders** | Yes — datetime reminders on notes and tasks, combined reminders view | No |
| **Labels/tags** | Yes — filterable across notes and tasks | No |
| **Unified entry model** | Yes — `Entry` type covers history, notes, tasks in one schema | No |
| **Semantic search** | Stub — `search_similar` with sqlite-vec, zero embeddings placeholder | No |
| **Dashboard** | Yes — 7-day stats, activity heatmap, top apps, dictation flow, streak, tasks/reminders due today | No |

---

## 9. UI & UX

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **System tray** | Yes — left-click opens main window, right-click menu (Settings, History, Dictionary, Snippets, Updates, Quit) | Yes — menu bar icon with mode switching, status indicators | Both have tray; SW's shows mode state |
| **Tray icon states** | Tooltip-only (recording/processing icons not wired yet) | Animated icon states (idle, recording, processing, error) | Gap — Kalam has the code but icons aren't wired |
| **Main window** | Full app: sidebar nav, pages (Home, History, Notes, Tasks, Reminders, Dictionary, Snippets, Settings) | Settings/History panel (simpler — most interaction is tray + overlay) | **Kalam advantage** — richer app shell |
| **Overlay (recording window)** | Yes — waveform visualizations (7 styles), expand/collapse, processing/error states, latency tracing | Yes — shows active mode, context source, recording status | Both have overlays; SW shows context info |
| **Onboarding** | Yes — multi-step wizard (terms, email, permissions, mic test, API key, hotkeys, languages, STT mode) | Yes — permissions, language, model, mic, hotkey, tutorial | Both have onboarding |
| **Theme** | Light / Dark / Auto (system preference) | Not documented | Kalam advantage |
| **Sidebar** | Collapsible, icon-only mode | N/A (no main window sidebar) | Kalam advantage |
| **Status bar** | Yes — dictation phase, latency, DB status, STT mode quick-switch, mic picker, language badge | N/A | **Kalam advantage** |
| **Prototype/design mode** | Yes — `?page=prototype` with mock data for design iteration | No | Kalam advantage (dev tooling) |

---

## 10. Settings & Configuration

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Hotkey config** | Yes — visual key capture, hold/toggle mode, separate command hotkey, language toggle hotkey | Yes — per-mode keyboard shortcuts | Both have hotkey config |
| **Audio device selection** | Yes — dropdown with mic test | Yes | Parity |
| **STT provider/mode** | Yes — Cloud/Local/Hybrid/Auto, provider dropdown, API key validation | Yes — per-mode voice model selection | Kalam is global; SW is per-mode |
| **Formatting options** | Yes — voice commands, filler removal, auto-punctuation, custom rules, injection method/delays | Not exposed as user settings (AI handles it) | Different approach |
| **Overlay customization** | Yes — 7 waveform styles, position (9 options), offset, expand direction | Basic recording window | **Kalam advantage** |
| **Privacy settings** | Yes — retention days, telemetry toggle, sensitive app patterns editor | Enterprise-level controls | Kalam has more user-facing privacy controls |
| **Logging/diagnostics** | Yes — in-app log viewer, export CSV, logging level, latency tracing | History tab for debugging | **Kalam advantage** |
| **Update channel** | Yes — stable/beta toggle | Not documented | Kalam advantage |
| **Command mode config** | Yes — provider, API keys, model selection per provider | N/A (LLM is per-mode) | Different architecture |

---

## 11. Privacy & Security

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Fully offline mode** | Yes — SenseVoice/Whisper.cpp, zero network | Yes — local Whisper models | Parity |
| **Sensitive app detection** | Yes — regex patterns → ForceLocal STT | Guidance only (use local models) | **Kalam advantage** — automatic enforcement |
| **Audio storage** | Never stored to disk (in-memory only) | Not documented | Kalam is explicit about this |
| **History encryption** | AES-256 at rest | Not documented | Kalam advantage |
| **Telemetry** | Opt-in only (PostHog) | Not documented | Kalam is explicit |
| **Context data to cloud** | No context data sent (no context awareness yet) | Yes — app content, selected text, clipboard sent to cloud LLMs | **Kalam advantage** (privacy) / **gap** (intelligence) |
| **BYOK** | Yes — user's own keys, zero data retention by Kalam | Yes — own API keys supported | Parity |

---

## 12. Developer & Automation Features

| Feature | Kalam | SuperWhisper | Notes |
|---|---|---|---|
| **Dev bridge** | Yes — HTTP API on localhost for Cypress/dev testing | No | **Kalam advantage** |
| **Deep links** | No | Yes — `superwhisper://mode?key=...` + `superwhisper://record` | Gap |
| **Raycast/Alfred integration** | No | Yes | Gap |
| **Plugin/extension system** | Planned (Phase 4) | No | Neither has it yet |
| **Open source** | Yes | No | **Kalam advantage** |

---

## 13. Summary: Where Kalam Leads

1. **Open source & BYOK-first** — zero cost, full transparency, community-driven
2. **Lightweight** — 10-30MB RAM target vs proprietary bloat
3. **Voice commands** — explicit, deterministic regex-based command system (punctuation, editing, formatting)
4. **Notes, Tasks & Reminders** — full productivity suite built into the dictation app (SuperWhisper has nothing like this)
5. **Dashboard & analytics** — 7-day stats, activity heatmap, streak tracking, top apps
6. **Text injection sophistication** — dual-strategy with retries, per-app clipboard override, configurable delays
7. **Privacy enforcement** — automatic ForceLocal on sensitive apps, AES-256 history encryption, explicit no-audio-storage policy
8. **Formatting rules** — regex-powered custom rules (more powerful than SuperWhisper's literal replacements)
9. **Dictionary prompt leakage sanitization** — prevents vocabulary hints from appearing in transcription output
10. **Developer tooling** — dev bridge HTTP API, prototype mode, latency tracing, logging/diagnostics
11. **Overlay customization** — 7 waveform styles, 9 position options, offset control, expand direction
12. **Cross-platform ambition** — Windows-first with macOS/Linux planned (vs SuperWhisper's macOS-first)

---

## 14. Summary: Where SuperWhisper Leads

1. **Modes system** — named profiles bundling STT + LLM + instructions + context + audio settings, switchable per-task
2. **Integrated LLM post-processing** — every dictation (except Voice mode) runs through an LLM for grammar, formatting, tone adaptation
3. **Context awareness** — reads active app content, selected text, clipboard via accessibility APIs; feeds to LLM
4. **Voice-activated editing** — highlight text in any app → speak a command → AI transforms in-place
5. **Smart self-correction** — LLM detects mid-sentence rephrasing and removes false starts
6. **Auto-activation rules** — automatically switch processing profile based on active app/website
7. **Clipboard as ephemeral vocabulary** — copy a term before dictating, AI uses it for correct spelling
8. **Recording UI shows context source** — user sees which app the AI is reading
9. **Speaker separation** — identify speakers in meeting recordings
10. **File transcription** — transcribe existing audio/video files (not just live dictation)
11. **Deep links & automation** — URL scheme for mode switching + recording, Raycast/Alfred integrations
12. **Enterprise** — SAML SSO, SCIM provisioning, seat-based billing, model access controls (shipping today)
13. **iOS app** — mobile companion
14. **More cloud STT options** — Deepgram Nova (including Medical), proprietary optimized models

---

## 15. Gap Priority Matrix

Gaps ranked by user impact and implementation complexity.

| Gap | User Impact | Complexity | Kalam Infra Exists? | Priority |
|---|---|---|---|---|
| **LLM post-processing integrated into dictation** | Very High | Medium | Yes (CommandConfig, LLM providers, generate_structured_data) | P0 |
| **Modes / profiles system** | Very High | High | Partial (global config, RecordingType enum) | P0 |
| **Context awareness (app content, selection, clipboard)** | Very High | High | Partial (target_app capture, sensitive app detection) | P1 |
| **Voice-activated editing (in-place)** | High | High | Partial (text injection exists, LLM exists, but no selection reading) | P1 |
| **Smart self-correction** | High | Low | Yes (just LLM prompting) | P1 |
| **Auto-activation rules (mode switching)** | Medium | Medium | Yes (sensitive app patterns infra) | P2 |
| **Clipboard as ephemeral vocabulary** | Medium | Low | Partial (clipboard access exists for injection) | P2 |
| **Context display in overlay** | Medium | Low | Yes (overlay exists) | P2 |
| **Recording UI shows context source** | Medium | Low | Yes (overlay exists) | P2 |
| **Speaker separation** | Medium | High | No | P3 |
| **File transcription** | Medium | Medium | No | P3 |
| **Deep links / URL scheme** | Low | Low | No | P3 |
| **Enterprise (SSO, SCIM)** | Low (for now) | High | No | P4 |
| **iOS app** | Low (for now) | Very High | No | P4 |

---

## 16. Strategic Observations

### Kalam's Unique Position

Kalam is building something SuperWhisper doesn't have: a **voice-first productivity hub** (dictation + notes + tasks + reminders + dashboard). SuperWhisper is purely a dictation tool — it processes voice and injects text. Kalam captures the text AND gives users a place to organize, track, and act on it.

### The LLM Integration Gap

The single biggest gap is that SuperWhisper treats LLM post-processing as **the default** for every dictation, while Kalam treats it as a **separate mode** (command mode with its own hotkey). Kalam already has all the LLM infrastructure (5 providers, model listing, testing, structured generation) — the gap is architectural, not capability. Integrating LLM processing as an optional-but-default step in the regular dictation pipeline would close the largest competitive gap.

### Context Awareness as the Differentiator

SuperWhisper's "Super Mode" is impressive because of context awareness, not because of the LLM itself. The LLM is commodity (Claude, GPT). The magic is **what context you feed it**. Kalam already captures `target_app` and has sensitive app detection via OS APIs — extending this to read input field content and selected text is the natural next step. On Windows, this would use UI Automation APIs (which Kalam's PRD already specifies).

### Privacy Tension

Context awareness creates a privacy tension: reading app content and sending it to cloud LLMs is the opposite of Kalam's privacy-first philosophy. The resolution is **local LLM processing** — Kalam could offer context-aware dictation with a local LLM (Ollama, llama.cpp) for users who want intelligence without cloud data exposure. SuperWhisper can't do this (their AI processing is cloud-only).
