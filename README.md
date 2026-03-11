# Kalam

![Kalam logo](public/logo/kalam-logo-horizontal.svg)

**Kalam** (كلام — *speech* in Arabic) is an open-source, cross-platform voice dictation app that turns speech into text in any application on Windows, macOS, and Linux. A free, privacy-friendly alternative to Whisperflow.

[**Website**](https://afaraha8403.github.io/kalam/) · [**Documentation & manual**](https://afaraha8403.github.io/kalam/documentation.html) · [**Download (GitHub Releases)**](https://github.com/afaraha8403/kalam/releases)

---

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/afaraha8403/kalam?label=release)](https://github.com/afaraha8403/kalam/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/afaraha8403/kalam/releases)
[![License](https://img.shields.io/badge/license-Dual%20(MIT%20NC%20%2B%20Commercial)-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri)](https://v2.tauri.app/)
[![Open source](https://img.shields.io/badge/open--source-✓-green.svg)](https://github.com/afaraha8403/kalam)

## Features

- ⚡ **4× faster** than typing — speak; it types everywhere
- 🔒 **Privacy-first** — local mode keeps audio on-device
- 🌍 **Cross-platform** — Windows, macOS, Linux
- ☁️ **Cloud + local STT** — Groq, OpenAI, or offline (SenseVoice / Whisper)
- 🎯 **Global hotkey** — hold to dictate in any app
- 📝 **History** with search and optional encryption
- 🎭 **Voice commands** for formatting (e.g. “new paragraph”)
- 📌 **Command mode** — create notes, tasks, and reminders by voice (optional LLM parsing)
- 📎 **Snippets** for frequently used text

## Speech-to-Text (STT) providers

| Mode  | Providers / models |
|-------|--------------------|
| **Cloud** | **Groq** (Whisper large v3 turbo), **OpenAI** (Whisper-1) — BYOK in Settings |
| **Local** | **SenseVoice** (Sherpa-ONNX), **Whisper Base** (whisper.cpp) — no API key |

You can run **Cloud only**, **Local only**, or **Hybrid** (cloud with local fallback). Local mode works fully offline.

## Command mode (notes, tasks, reminders)

Use a **separate hotkey** from dictation. When you press it and speak, Kalam creates a **note**, **task**, or **reminder** instead of typing into another app.

- Say *“new note …”*, *“new task …”*, or *“new reminder …”* with your content.
- **Optional LLM parsing** (Settings → Command Mode): Kalam can use **Groq**, **OpenRouter**, **Gemini**, **OpenAI**, or **Anthropic** to infer type and extract title, due date, repetition, and description from natural speech.

## Quick start

1. **Download** the latest build for your OS from [GitHub Releases](https://github.com/afaraha8403/kalam/releases). Install and open Kalam.
2. **Configure** (Settings → Audio & dictation):
   - **Cloud:** enter your [Groq](https://console.groq.com) or OpenAI API key and pick the provider.
   - **Local:** choose SenseVoice or Whisper Base; the app will download the engine and model when needed.
3. **Dictate:** press **Ctrl+Win** (Windows), **Ctrl+Super** (Linux), or **Ctrl+Cmd** (macOS), hold while you speak, then release. Text is inserted into the app that had focus.

For setup details, API keys, and the full user manual, see the [**documentation**](https://afaraha8403.github.io/kalam/documentation.html).

## Building from source

For development or when pre-built binaries aren’t available for your platform:

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) 1.75+

### Setup and task runner

We use a PowerShell task runner (`tasks.ps1`) for deps, dev, tests, and builds.

```powershell
# Clone
git clone https://github.com/afaraha8403/kalam.git
cd kalam

# Install dependencies
./tasks.ps1 deps

# Run in development
./tasks.ps1 dev

# Run tests
./tasks.ps1 test

# Build for production
./tasks.ps1 build
```

All commands (including release and signing): `./tasks.ps1 help`

## Architecture

- **Framework:** [Tauri v2](https://v2.tauri.app/) (Rust backend + WebView frontend)
- **Frontend:** Svelte + TypeScript
- **Audio:** `cpal` (cross-platform capture)
- **Text injection:** `enigo` (keystroke simulation)
- **Cloud STT:** Groq API, OpenAI API
- **Local STT:** SenseVoice (Sherpa-ONNX), Whisper (whisper.cpp sidecar)

## API keys (BYOK)

- **Cloud STT:** Sign up at [Groq Console](https://console.groq.com) or [OpenAI](https://platform.openai.com), get an API key, add it in **Settings → STT Provider**.
- **Command mode LLM:** Optional; add provider and key in **Settings → Command Mode**. Local dictation and basic command phrases work without any key.

## Privacy

- **Audio** is not stored to disk (in-memory only).
- **Cloud mode** sends audio to the chosen provider over TLS; we do not retain it.
- **Local mode** processes everything on-device.
- **History** is stored locally (SQLite) with optional AES-256 encryption.
- **Telemetry** is opt-in and off by default.

## Contributing

Issues and pull requests are welcome. See [GitHub](https://github.com/afaraha8403/kalam) to get started.

## License

Dual license (MIT NC + Commercial). See [LICENSE](LICENSE) for details.

## Acknowledgments

- [Tauri](https://tauri.app/) for the framework
- [Groq](https://groq.com/) and [OpenAI](https://openai.com/) for cloud STT
- [SenseVoice](https://github.com/FunAudioLLM/SenseVoice) and [whisper.cpp](https://github.com/ggml-org/whisper.cpp) for local STT
- Everyone who contributes and supports the project
