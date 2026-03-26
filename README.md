# Kalam

![Kalam logo](public/logo/kalam-logo-horizontal.svg)

**Kalam** (كلام — *speech* in Arabic) is an open-source, cross-platform voice dictation app that turns speech into text in any application on Windows, macOS, and Linux. A free, privacy-friendly alternative to Whisperflow.

[**Website**](https://kalam.stream/) · [**Documentation & manual**](https://kalam.stream/documentation.html) · [**Download (GitHub Releases)**](https://github.com/afaraha8403/kalam/releases) — *beta pre-releases available*

---

[![GitHub release (latest by date including pre-releases)](https://img.shields.io/github/v/release/afaraha8403/kalam?include_prereleases&label=release)](https://github.com/afaraha8403/kalam/releases)
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
3. **Dictate:** press **Ctrl+Win** (Windows), **Ctrl+Super** (Linux), or **Ctrl+Cmd** (macOS), hold while you speak, then release. Text is inserted into the app that had focus. On macOS, when you first use the hotkey, allow **Input Monitoring** if prompted—required for the hotkey to work in other apps.

For setup details, API keys, and the full user manual, see the [**documentation**](https://kalam.stream/documentation.html).

## Uninstalling

Kalam keeps data in **`~/.kalam`** (on Windows: **`%USERPROFILE%\.kalam`**) and, separately, downloaded models and sidecars under your OS app data directory (e.g. **`%LOCALAPPDATA%\Kalam`** on Windows, **`~/Library/Application Support/com.Kalam.Kalam`** on macOS, **`~/.local/share/kalam`** on Linux).

| OS | What to do |
|----|------------|
| **Windows** | Run the uninstaller (NSIS). Enable **Delete the application data** to remove both Tauri’s app data and the folders above. |
| **macOS** | Remove **Kalam** from **Applications** (or use the **.pkg** installer flow). To clear data, delete `~/.kalam` and `~/Library/Application Support/com.Kalam.Kalam` if you want a full wipe. |
| **Linux (.deb)** | `sudo apt purge <kalam-package>` removes the package and, on purge, user data for the account that ran `sudo` (when `SUDO_USER` is set). The exact package name matches the published `.deb` (often `kalam`-related—check `dpkg -l \| grep -i kalam`). **AppImage:** delete the file and remove `~/.kalam` and `~/.local/share/kalam` manually. |

**Updates:** In the app, use **Settings → About → Check for updates** (signed releases). That uses the same artifacts as GitHub Releases.

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

## Testing

The project uses **several layers**; they complement each other and are not interchangeable.

| Kind | Command | What it covers |
|------|---------|----------------|
| **Frontend unit** | `npm run test:unit` | Vitest + jsdom — small TypeScript / logic tests under `src/**/*.spec.ts`. |
| **Rust** | `cargo test` (from `src-tauri/`) | Backend unit tests, STT chunking helpers, and **STT integration tests** that read WAV fixtures. `./tasks.ps1 test` runs Vitest + `cargo test` (same as much of CI). |
| **STT integration only** | `npm run test:stt` | Same harness as Rust tests, but only the `stt_integration_tests` binary (faster when you only care about speech fixtures). |
| **Browser E2E** | `npm run test:e2e` | Cypress against the Vite dev server; **`POST /api/invoke` is mocked** (no real Tauri, no microphone, no WAV playback). Use this for onboarding/settings UI flows. |

### Sample WAV files (STT / integration tests)

Put **mono** `.wav` fixtures used by the Rust STT pipeline tests here:

**`src-tauri/tests/fixtures/`**

- The suite includes `test_english.wav` as a reference (short spoken phrase). You can add more files (e.g. other languages or lengths) and extend [`src-tauri/tests/stt_integration_tests.rs`](src-tauri/tests/stt_integration_tests.rs) to load them.
- Tests resample to **16 kHz** mono float internally where needed; 16-bit PCM mono WAVs are a good default.
- **Groq / OpenAI** checks in that file run only when `GROQ_API_KEY` or `OPENAI_API_KEY` is set; without keys those cases are skipped and the rest still pass.
- Large binaries: consider [Git LFS](https://git-lfs.com/) if fixtures grow beyond small clips.

Cypress E2E does **not** consume these WAV files today; it stubs the dev bridge. Real audio → transcript coverage is intentionally in **`cargo test`** / `npm run test:stt`.

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

### Permissions on macOS

When you first use the dictation hotkey, macOS may show a prompt such as *"Kalam would like to monitor input from your keyboard"* or *"This application can record your keystrokes."* That is **Input Monitoring**. Kalam needs it only so the **global hotkey** (e.g. Ctrl+Cmd) works in any app. We use it solely to detect your hotkey press; we do not record, log, or send your keystrokes. Audio is handled as described above (in-memory or to your chosen STT provider).

## Contributing

Issues and pull requests are welcome. See [GitHub](https://github.com/afaraha8403/kalam) to get started.

## License

Dual license (MIT NC + Commercial). See [LICENSE](LICENSE) for details.

## Acknowledgments

- [Tauri](https://tauri.app/) for the framework
- [Groq](https://groq.com/) and [OpenAI](https://openai.com/) for cloud STT
- [SenseVoice](https://github.com/FunAudioLLM/SenseVoice) and [whisper.cpp](https://github.com/ggml-org/whisper.cpp) for local STT
- Everyone who contributes and supports the project
