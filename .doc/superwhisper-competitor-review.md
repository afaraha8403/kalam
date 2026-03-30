# SuperWhisper — Competitor Review

**Date:** March 29, 2026
**Source:** https://superwhisper.com/docs
**Focus:** Modes system (especially "Super Mode"), model architecture, and features relevant to Kalam

---

## 1. Product Overview

SuperWhisper is an AI-powered dictation app for **macOS, Windows, and iOS**. Core flow:

1. **Dictation** — user speaks naturally
2. **Transcription** — voice model converts speech → raw text
3. **AI Processing** — language model transforms/formats the text based on the active "mode"

Pricing: Free tier (limited models), **Pro** subscription (unlocks all cloud/local models, unlimited AI, unlimited modes). Enterprise tier with SAML SSO, SCIM, seat-based billing.

Key differentiators vs Kalam:
- **Two-stage pipeline**: voice model (STT) → language model (LLM post-processing). Kalam currently does STT only, with a separate "command mode" for LLM.
- **Modes system**: pre-built and custom processing profiles that change how text is shaped after transcription.
- **Context awareness**: reads from active app, selected text, clipboard — feeds into the LLM prompt.
- **Vocabulary & Replacements**: custom word hints for STT + deterministic find/replace post-transcription.

---

## 2. The Modes System — Architecture

### What is a Mode?

A "mode" is a **named processing profile** that bundles:

| Setting | Description |
|---|---|
| **Voice Model** | Which STT model to use (cloud or local) |
| **Language** | Transcription language + optional translate-to-English |
| **AI Model** | Which LLM to use for post-processing (or none) |
| **AI Instructions** | System prompt / processing rules for the LLM |
| **Context toggles** | Application context, selected text, clipboard — on/off per mode |
| **Audio settings** | Mute audio, pause media, record system audio, identify speakers |
| **Auto-activation rules** | Automatically switch to this mode when a specific app/website is active |
| **Examples** | Input/output pairs to teach the LLM expected behavior (Custom mode) |

Modes are stored as JSON files in `~/Documents/superwhisper/modes/` with a unique `key` field.

### Built-in Modes

| Mode | AI Processing | Context Aware | Purpose |
|---|---|---|---|
| **Voice to Text** | None | No | Raw transcription only. Speed-optimized. |
| **Message** | Yes | No | Removes fillers, fixes grammar, formats for readability. Preserves tone. |
| **Email** | Yes | No | Structures dictation into email format. |
| **Note** | Yes | No | Structures into notes with headings, bullets, key points, action items. |
| **Meeting** | Yes | No (optional speaker ID) | Summarizes meetings, extracts action items. Can record system audio. |
| **Super** | Yes | **Yes (all 3 types)** | Context-aware dictation. The flagship "smart" mode. |
| **Custom** | Yes | Configurable | User writes their own AI instructions. Full control. |

### Mode Switching Methods

1. **Keyboard shortcut** — cycle through modes while recording window is open
2. **Menu bar** — click tray icon → Select Mode submenu
3. **Auto-activation rules** — per-mode rules that trigger based on active app/website (once activated, can't be overridden; doesn't auto-switch back)
4. **Deep links** — `superwhisper://mode?key=YOUR_MODE_KEY` + `superwhisper://record` for automation tools (Raycast, Alfred, Shortcuts)

---

## 3. Super Mode — Deep Dive

Super Mode is the **premium, context-aware** mode. It's the only built-in mode with all three context types enabled by default. Think of it as a smart assistant that knows what you're doing on-screen and shapes your dictation accordingly — not just transcribing, but understanding.

### Processing Pipeline

```
User speaks → Voice Model (STT) → Raw transcript
                                        ↓
                              Context gathered:
                              • Application context (accessibility APIs)
                              • Selected text
                              • Clipboard (if copied within 3s)
                              • System info (date, time, user name, computer name)
                                        ↓
                              LLM processes transcript + context
                                        ↓
                              Polished, context-adapted text output
```

### Context Awareness — Three Types

#### 1. Application Context
- Uses **accessibility APIs** to identify the active application
- Reads text from the **current input field or text editor**
- Also injects system metadata: current date/time, user's full name, computer name
- **Captured AFTER voice processing** (between transcription and AI processing) — so the active window at that moment determines context
- The recording window shows which app's information is being used, giving the user confidence the AI is reading the right screen

#### 2. Selected Text Context
- Reads any **highlighted/selected text** in the active window
- **Captured at the moment recording starts** — focus must be on the app with the selection
- Used to inform AI processing (e.g., "make this title case")
- This is the basis for **voice-activated editing**: highlight text → speak a command → AI edits in-place

#### 3. Clipboard Context
- Incorporates text **copied within 3 seconds before recording starts, or during dictation**
- Last copied item is used
- Primary use case: **spelling assistance** — if you copy a name, product term, or technical jargon just before dictating, the AI uses it to correctly spell those terms in your dictation without needing to add them to a permanent vocabulary list
- Acts as ephemeral, on-the-fly vocabulary hints

### What Super Mode Does

#### Context-Aware Dictation
- **Adapts messages to match application context** — e.g., more formal in email apps, casual in chat
- **Corrects spelling** of names, technical terms, and product names by reading them from the active app, selected text, or clipboard — without the user teaching those words individually
- **Converts spoken URLs and emails** into proper format
- **Preserves message tone and intent** while improving clarity

#### Smart Self-Correction
- Handles **mid-sentence corrections**: if the user trips over words or rephrases while speaking, Super Mode detects the self-correction and automatically removes the false start, outputting only the clean final version
- Fixes punctuation, capitalization, grammar, and spelling on the fly — this is all LLM-side processing, not STT

#### Voice-Activated Editing (In-Place)
This is a major UX feature. The user highlights text in **any** document or app and speaks a command to transform it:
- "Make this sound more professional"
- "Turn this into bullet points"
- "Translate this into French"
- "Make this text title case"
- "Convert selection to uppercase"

The AI edits the text **right there in the application** — no copy-paste to another tool. The edited text replaces the selection directly. This turns dictation from "input only" into a full **voice-driven editing tool**.

**Limitation**: commands should be **formatting/transformation** focused, not content generation. "Summarize this paragraph" or "expand on these ideas" are not recommended — those belong in Custom Mode.

#### Workflow Integration
- Everything happens in the app the user is already in — no switching to browser, email client, or word processor
- The recording window displays which app's context is being used, so the user can verify the AI is looking at the right screen
- Eliminates the copy → switch app → paste → process → copy → switch back → paste workflow

#### Customization Layer
- While Super Mode is smart out of the box, users can add **custom vocabulary** to teach it very specific technical terms or jargon, making dictation even more accurate for their domain
- This vocabulary is sent as hints to the STT model during transcription

### Limitations / Gotchas

- Voice-activated editing commands should be **formatting/transformation only**, not content generation
- Requires **capable LLM** (Claude, GPT recommended) — local/smaller models struggle with the multi-context complexity
- Application context can capture wrong window if user switches apps before processing completes
- Requires **accessibility permissions** on macOS
- Auto-activation rules are sticky — once activated for an app, can't be overridden; doesn't switch back automatically
- Clipboard context only works if text is copied within a tight 3-second window
- Smart self-correction is LLM-dependent — quality varies by model

---

## 4. Custom Mode — Deep Dive

Custom Mode is the "build your own" option. Key capabilities:

- **AI Instructions**: free-form system prompt (empty by default — must be filled or results are unpredictable)
- **Context toggles**: application context, selected text, clipboard — each independently toggleable
- **Examples**: input/output pairs to teach the LLM expected behavior
- **Prompting conventions**: reference content as `User Message`, `Application Context`, `Selected Text`, `Clipboard Context` in instructions
- **XML tags**: optional structured prompting with `<role>`, `<instructions>`, `<requirements>`, `<context>`, `<style>`, `<output-format>` tags for complex workflows
- Unlike Super Mode, Custom Mode is **not limited to text formatting** — can do content generation, analysis, translation, code generation, etc.

### Customizing Built-in Modes

Users can also create custom versions of built-in modes by accessing their underlying AI instructions and modifying them. This is a middle ground between built-in and fully custom.

---

## 5. Model Library — How SuperWhisper Manages Models

SuperWhisper treats models as a **unified library** that the user browses and selects from — both voice (STT) models and language (LLM) models live in one place. Key design decisions:

### Two Model Categories
1. **Voice Models** — convert speech to text (STT). Cloud or local.
2. **Language Models** — enhance/format/transform text after transcription (LLM post-processing). Cloud or local.

### Per-Mode Model Selection
Each mode can pick its own voice model AND its own language model independently. So a "Meeting" mode might use Deepgram Nova (best for speaker separation) + GPT-5 (best for summarization), while a "Quick Message" mode uses the fastest cloud STT + a lightweight LLM.

### Model Management UX
- Models are managed in the **Advanced Settings sidebar** (accessible from any mode's settings)
- Local models can be **downloaded** from within the app
- Users can **connect their own API tokens** for supported cloud providers
- An **"Experimental Models"** toggle shows/hides beta models
- **Voice Model Active Duration** setting controls how long local models stay loaded in RAM (10s to 1hr) — balances speed vs resource usage

### Enterprise Model Controls
- Admins can **enable/disable** SuperWhisper's hosted cloud models for their organization
- Admins can add **Custom Models** with their own API keys, endpoints, and model IDs
- Custom models are **distributed to all members** automatically on app start
- Three deployment patterns: cloud-only, custom-only, or hybrid

### BYOK (Bring Your Own Key)
Users can plug in their own API keys for: OpenAI, Anthropic, Deepgram, Groq. The app fetches available models from the provider's API and lets the user pick.

### What Kalam Has Today
- **STT models**: 2 local (SenseVoice, Whisper base) with download/install/lifecycle management. 2 cloud (Groq, OpenAI) with API key validation.
- **LLM models**: 5 providers (Groq, OpenRouter, OpenAI, Anthropic, Gemini) with `fetch_llm_models` to list available models, `test_llm_model` to validate, `generate_structured_data` to call. But these are only used for command mode, not for dictation post-processing.
- **No unified model library UI** — STT models are in Settings > STT Provider, LLM models are in Settings > Command Mode. These are separate, disconnected sections.

---

## 6. Voice Models (STT)

### Cloud Models

| Name | Provider | Languages | Speed | Accuracy | Notes |
|---|---|---|---|---|---|
| S1-Voice | SuperWhisper | Multi | 10 | 9 | Proprietary cloud, optimized for low latency |
| Ultra | SuperWhisper | Multi | 9 | 9 | Proprietary cloud |
| Nova 3 | Deepgram | Multi | 7 | 8 | Good for longer recordings, speaker separation |
| Nova 2 | Deepgram | Multi | 7 | 7 | |
| Nova Medical | Deepgram | English | 10 | 7 | Healthcare-specific |

### Local Models

| Name | Base | Languages | Speed | Accuracy | Size |
|---|---|---|---|---|---|
| Ultra V3 Turbo | Whisper | All | 8 | 8 | 1.6 GB |
| Ultra | Whisper | All | 6 | 10 | 3 GB |
| Pro | Whisper | All | 7 | 8 | 1.5 GB |
| Standard | Whisper | All | 8 | 5 | 500 MB |
| Nano | Whisper | All | 9 | 3 | 150 MB |
| Fast | Whisper | All | 10 | 1 | 75 MB |
| Parakeet | Nvidia | English | 10 | 8 | 476 MB |
| Parakeet Multi | Nvidia | Multi | 10 | 8 | 494 MB |

Local models run via **whisper.cpp** (Whisper models) or **WhisperKit SDK** (Parakeet models).

Parakeet is extremely fast and runs in parallel over long recordings but struggles with punctuation and has minor hallucination issues with single-word recordings.

---

## 7. Language Models (LLM Post-Processing)

| Name | Provider | Speed | Benchmark | License |
|---|---|---|---|---|
| S1-Language | SuperWhisper | 10 | ~80 | Pro |
| Claude 4.5 Sonnet | Anthropic | 8 | 89 | Pro |
| Claude 4 Sonnet | Anthropic | 8 | 87 | Pro |
| Claude 3.7 Sonnet | Anthropic | 8 | 85 | Pro |
| Claude 3.5 Sonnet | Anthropic | 8 | 89 | Pro |
| Claude 3.5 Haiku | Anthropic | 9 | 75 | Pro |
| GPT-5 | OpenAI | 7 | 91 | Pro |
| GPT-5 mini | OpenAI | 8 | 87 | Pro |
| GPT-5 nano | OpenAI | 9 | 83 | Pro |
| GPT-4.1 | OpenAI | 7 | 90 | Pro |
| GPT-4.1 mini | OpenAI | 8 | 86 | Pro |
| GPT-4.1 nano | OpenAI | 9 | 80 | Pro |
| Llama 3 8b | Groq | 10 | 67 | Pro |

Users can also **bring their own API keys** for supported providers.

---

## 8. Vocabulary System

Two-part system for improving accuracy:

### Vocabulary Words (STT Hints)
- Custom words sent alongside audio to the STT model as recognition hints
- Good for: names, acronyms, company names, specialized terminology
- **Caveat**: too many words can confuse the STT model and affect punctuation, language detection, formatting
- Recommendation: use sparingly

### Replacements (Post-Transcription)
- Deterministic find/replace applied AFTER transcription, BEFORE AI processing
- Case-insensitive matching, exact-case output
- Not AI-dependent — always consistent
- Good for: correcting persistent mis-transcriptions, expanding abbreviations
- Can add multiple replacement rules pointing to the same correct output

**Kalam parallel**: Kalam has `FormattingRule` (pattern/replacement with regex support) and `DictionaryEntry` (custom terms). The SuperWhisper vocabulary hints concept (sending words to STT) is something Kalam could adopt for Groq/Whisper's `prompt` field.

---

## 9. Other Notable Features

### Hallucination Mitigation
- **Silence removal**: cuts silent periods from audio before transcription to prevent the STT model from hallucinating words during silence
- **Vocabulary pruning**: fewer custom words = less STT confusion
- **History tab**: shows both raw transcription and AI-processed result for debugging which stage hallucinated

### Speaker Separation
- Available in Meeting mode and Voice mode
- Uses Deepgram Nova models for best results
- Speaker labels appear in Segments tab only — not passed to AI summaries automatically
- Advanced workflow: Voice mode with speaker ID → copy transcript → Custom mode with AI instructions to process per-speaker

### File Transcription
- Can transcribe existing audio/video files (not just live dictation)
- Drag-and-drop or menu bar → "Transcribe File"

### Auto-Activation Rules
- Per-mode rules: "when app X is active, switch to this mode"
- Sticky: once activated, can't be overridden; doesn't switch back automatically
- Works with apps and websites

### Deep Links & Automation
- `superwhisper://mode?key=MODE_KEY` — switch mode
- `superwhisper://record` — start recording
- Integrations: Raycast extension, Alfred workflow, Apple Shortcuts

### Sensitive Data
- Guidance for healthcare/financial/regulated data
- Local models recommended for sensitive contexts
- Enterprise controls for model access

---

## 10. Key Takeaways for Kalam

### What SuperWhisper does well that Kalam should consider:

1. **Modes as first-class concept**: bundling STT model + LLM + instructions + context settings into a switchable profile is powerful UX. Users don't think about "which model" — they think about "what am I doing" (messaging, emailing, note-taking, coding).

2. **Two-stage pipeline (STT → LLM)**: every mode (except Voice) runs the transcript through an LLM. This is fundamentally different from Kalam's current approach where command mode is a separate hotkey/flow. SuperWhisper makes AI post-processing the default, not an opt-in extra step.

3. **Context awareness via accessibility APIs**: reading the active app's input field, selected text, and clipboard gives the LLM crucial context. This is what makes Super Mode feel "intelligent". Kalam already captures `target_app` — extending to read input field content and selected text would be a major upgrade.

4. **Smart self-correction**: the LLM detects when the user corrects themselves mid-sentence and removes the false start automatically. This is a quality-of-life feature that makes dictation feel polished. It's purely an LLM prompting technique — the STT transcribes everything including mistakes, and the LLM cleans it up.

5. **Voice-activated editing (in-place)**: highlight text → speak a command → AI transforms the selection in-place. This turns dictation from an input-only tool into a full voice-driven editing tool. Huge for productivity — no copy/paste/switch-app workflow.

6. **Clipboard as ephemeral vocabulary**: copying a name or term just before dictating teaches the AI that spelling on the fly, without permanent vocabulary entries. Lightweight and intuitive.

7. **Recording window shows active context**: the UI tells the user which app's information the AI is reading, building trust and debuggability.

8. **Auto-activation rules**: automatically switching processing profile based on what app the user is in. E.g., "when in VS Code, use a coding-optimized mode; when in Slack, use casual message mode."

9. **Vocabulary hints to STT**: sending custom words in the STT prompt field to improve recognition of domain-specific terms. Kalam's Dictionary feature could be extended to do this.

10. **Silence removal**: pre-processing audio to cut silence before sending to STT, reducing hallucinations. Simple but effective.

11. **Customizing built-in modes**: letting users tweak the AI instructions of built-in modes without starting from scratch.

### Where SuperWhisper has gaps / where Kalam can differentiate:

1. **macOS-first**: SuperWhisper's context awareness relies heavily on macOS accessibility APIs. Windows support is newer and has documented limitations. Kalam targets Windows/Linux as first-class.

2. **No open-source / BYOK-first model**: SuperWhisper is proprietary with a subscription. Kalam is open-source with BYOK.

3. **No offline LLM processing**: SuperWhisper's AI processing requires cloud LLMs. Kalam could offer local LLM post-processing (e.g., Ollama, llama.cpp) for fully offline intelligent dictation.

4. **Auto-activation is sticky**: once a mode activates for an app, it can't be overridden and doesn't switch back. This is a UX limitation.

5. **No real-time / streaming transcription in all modes**: SuperWhisper's realtime feature has documented issues and limitations.

6. **Resource usage**: SuperWhisper doesn't emphasize lightweight footprint the way Kalam does (10-30MB idle RAM).

7. **Privacy**: SuperWhisper sends context data (app content, selected text, clipboard) to cloud LLMs. Kalam's privacy-first approach with local processing is a strong differentiator.

---

## 11. Feature Mapping: SuperWhisper → Kalam Equivalents

| SuperWhisper Feature | Kalam Equivalent | Gap |
|---|---|---|
| Voice to Text mode | Default dictation mode | ✅ Exists |
| Message/Email/Note modes | — | ❌ No LLM post-processing modes |
| Super Mode (context-aware) | — | ❌ No context-aware processing |
| Custom Mode | Command Mode (partial) | ⚠️ Kalam's command mode is separate flow, not integrated |
| Smart self-correction (LLM removes false starts) | — | ❌ Not implemented (LLM prompting technique) |
| Voice-activated editing (in-place on selection) | — | ❌ Not implemented |
| Clipboard as ephemeral vocabulary | — | ❌ Not implemented |
| Context indicator in recording UI | — | ❌ Recording overlay doesn't show context source |
| Auto-activation rules | Sensitive app detection (ForceLocal) | ⚠️ Kalam detects apps but only for privacy, not mode switching |
| Vocabulary hints | Dictionary entries | ⚠️ Dictionary exists but may not be sent as STT prompt hints |
| Replacements | FormattingRule | ✅ Exists (with regex support — more powerful) |
| Speaker separation | — | ❌ Not implemented |
| File transcription | — | ❌ Not implemented |
| Meeting recording | — | ❌ Not implemented |
| Mode switching (hotkey/menu/deeplink) | — | ❌ No modes system |
| History with debug (raw vs AI) | History entries | ⚠️ History exists but no raw-vs-processed comparison |
| Silence removal | VAD (Silero) | ✅ Kalam uses VAD which serves similar purpose |
