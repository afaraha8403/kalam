# Kalam

**Kalam** (كلام — *speech* in Arabic) is an open-source, cross-platform voice dictation application that transforms spoken language into polished text across all applications on Windows, macOS, and Linux. A free, privacy-friendly alternative to Whisperflow.

![License](https://img.shields.io/badge/license-Dual%20(MIT%20NC%20%2B%20Commercial)-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)

## Features

- ⚡ **4x Faster** than typing
- 🔒 **Privacy-First** with local mode support
- 🌍 **Cross-Platform** (Windows, macOS, Linux)
- ☁️ **Dual-Engine** (Cloud + Local STT)
- 🎯 **Global Hotkey** activation
- 📝 **Transcription History** with search
- 🎭 **Voice Commands** for formatting
- 📎 **Snippets** for frequently used text

## Quick Start

1. **Download** the latest release for your platform from [Releases](https://github.com/kalam-voice/kalam/releases)
2. **Install** and launch the app
3. **Configure** your API key (for cloud mode) or download a local model
4. **Press** `Ctrl+Win` (or `Ctrl+Cmd` on macOS) to start dictating

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) 1.75+

### Setup

```bash
# Clone the repository
git clone https://github.com/kalam-voice/kalam.git
cd kalam

# Install dependencies
npm install

# Run in development mode
npm run tauri:dev
```

### Building

```bash
# Build for production
npm run tauri:build
```

## Architecture

- **Framework**: [Tauri v2](https://v2.tauri.app/) (Rust backend + WebView frontend)
- **Frontend**: Svelte + TypeScript
- **Audio Capture**: `cpal` (Cross-Platform Audio Library)
- **Text Injection**: `enigo` (keystroke simulation)
- **Cloud STT**: Groq API (Whisper)
- **Local STT**: SenseVoice / Whisper.cpp

## API Keys

Kalam uses a BYOK (Bring Your Own Key) model:

1. Sign up at [Groq Console](https://console.groq.com)
2. Get your free API key
3. Enter it in Settings → STT Provider

Local mode works without any API key!

## Privacy

- **Audio** is never stored to disk (in-memory only)
- **Cloud mode** sends audio to Groq via TLS 1.3 (zero retention)
- **Local mode** processes everything on-device
- **History** is stored locally in SQLite with AES-256 encryption
- **Telemetry** is opt-in only (disabled by default)

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Tauri](https://tauri.app/) for the amazing framework
- [Groq](https://groq.com/) for fast STT API
- [SenseVoice](https://github.com/FunAudioLLM/SenseVoice) for local STT
- All our contributors and supporters!

---

<p align="center">
  Made with ❤️ by the Kalam team
</p>
