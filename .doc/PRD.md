# Kalam — Final Product Requirements Document

**Version:** 2.0 (Final)
**Date:** March 4, 2026
**Status:** Approved for Development
**Audience:** AI Agents, Developers, Contributors

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement & Market Opportunity](#2-problem-statement--market-opportunity)
3. [Product Vision, Goals & Design Principles](#3-product-vision-goals--design-principles)
4. [User Personas & Use Cases](#4-user-personas--use-cases)
5. [Functional Requirements](#5-functional-requirements)
6. [Technical Architecture](#6-technical-architecture)
7. [Audio Pipeline](#7-audio-pipeline)
8. [Speech-to-Text Engine](#8-speech-to-text-engine)
9. [Text Injection System](#9-text-injection-system)
10. [User Interface Requirements](#10-user-interface-requirements)
11. [Voice Commands & Formatting Engine](#11-voice-commands--formatting-engine)
12. [Privacy, Security & Data Handling](#12-privacy-security--data-handling)
13. [API Key Management & Monetization](#13-api-key-management--monetization)
14. [Distribution, Updates & CI/CD](#14-distribution-updates--cicd)
15. [Platform-Specific Implementation Details](#15-platform-specific-implementation-details)
16. [Non-Functional Requirements](#16-non-functional-requirements)
17. [Accessibility Requirements](#17-accessibility-requirements)
18. [Telemetry & Analytics](#18-telemetry--analytics)
19. [Competitive Analysis](#19-competitive-analysis)
20. [Risk Assessment](#20-risk-assessment)
21. [Development Roadmap](#21-development-roadmap)
22. [Success Metrics & KPIs](#22-success-metrics--kpis)
23. [Appendix](#23-appendix)

---

## 1. Executive Summary

**Kalam** is an open-source, cross-platform voice dictation application that operates as a lightweight, always-available system tray daemon. It transforms spoken language into polished text across all applications on Windows, macOS, and Linux.

**What it does:** The user presses a global hotkey (e.g., `Ctrl + Win` / `Ctrl + Cmd`), speaks into their microphone, and the transcribed text is instantly injected into whatever application is currently focused — a text editor, browser, chat app, terminal, IDE, or any other text field.

**Key Value Propositions:**

| Proposition | Detail |
|---|---|
| **Speed** | 4x faster input than traditional typing |
| **Dual-Engine** | Cloud (Groq API) for speed + Local (SenseVoice/Whisper.cpp) for privacy |
| **Lightweight** | 10-30MB idle RAM vs 800MB+ in competitors |
| **Cross-Platform** | Windows, macOS, Linux (X11 primary, Wayland best-effort) |
| **Open Source** | 100% open-source, zero mandatory operational costs via BYOK model |
| **Privacy-First** | Full offline mode, encrypted data, no mandatory cloud dependency |

**Technology Stack Summary:**

| Layer | Technology |
|---|---|
| Framework | Tauri v2 (Rust backend + OS-native WebView frontend) |
| Backend | Rust |
| Frontend | React + TypeScript |
| Audio Capture | `cpal` (Cross-Platform Audio Library) |
| Audio Resampling | `rubato` |
| Cloud STT | Groq API (whisper-large-v3-turbo) |
| Local STT | SenseVoice-Small via Sherpa-ONNX (sidecar), Whisper.cpp (alternative) |
| VAD | Silero VAD (Rust port) |
| Text Injection | `enigo` (cross-platform keystroke simulation) |
| Notifications | `tauri-plugin-notification` |
| Auto-Updates | `tauri-plugin-updater` + GitHub Pages |
| Local Database | SQLite via `tauri-plugin-sql` |

---

## 2. Problem Statement & Market Opportunity

### 2.1 Problems with Existing Solutions

Current voice dictation tools (Wispr Flow, Apple Dictation, Windows Speech Recognition) suffer from critical limitations:

1. **Privacy & Compliance Risks:** All audio is sent to third-party cloud servers (OpenAI, Meta), creating HIPAA/GDPR violations for healthcare, legal, and enterprise users. There is no way to keep data on-device.

2. **Excessive Resource Consumption:** Users report 800MB+ idle RAM usage and 3-10 second initialization delays. For a tool meant to be "invisible," this is unacceptable.

3. **Cloud Dependency:** Applications fail completely when offline or on unreliable networks. There is no local fallback.

4. **Vendor Lock-in:** Proprietary formats, subscription costs ($15-30/month), and no customization options. Users cannot bring their own API keys, choose their own STT provider, or modify the software.

5. **Limited Platform Support:** Many tools exclude Linux entirely. macOS tools don't work on Windows and vice versa.

### 2.2 Market Opportunity

| Metric | Value |
|---|---|
| Voice Dictation Market Size (2026) | $3.2B globally, 18% YoY growth |
| Knowledge Worker Typing Time | 3-4 hours/day average |
| Accessibility Users | 1.3B people with disabilities globally |
| Developer Productivity Tools | 65% of developers use voice/code dictation |

### 2.3 Target Users

- **Primary:** Developers, writers, accessibility users, customer support professionals
- **Secondary:** Legal professionals, healthcare workers, students, sales teams
- **Tertiary:** Enterprise IT departments seeking self-hosted solutions

---

## 3. Product Vision, Goals & Design Principles

### 3.1 Vision

> "Empower every computer user to speak their thoughts freely — fast, private, and accessible anywhere."

### 3.2 Strategic Goals

| Goal | Success Metric |
|---|---|
| **Speed** | < 500ms end-to-end transcription latency (cloud), < 150ms (local) |
| **Efficiency** | < 30MB idle RAM, < 3s cold start |
| **Privacy** | 100% offline-capable mode |
| **Accessibility** | WCAG 2.1 AA compliance |
| **Adoption** | 50K+ active users in Year 1 |
| **Community** | 100+ contributors, monthly releases |

### 3.3 Design Principles

These principles MUST guide every implementation decision:

1. **Invisible Until Needed:** The application runs silently in the system tray. There is NO main window on startup. The user should forget the app is running until they press the hotkey.

2. **Universal Compatibility:** Text injection must work in ANY text field in ANY application. This is non-negotiable for the core user experience.

3. **Intelligent Defaults:** The application must work out of the box with minimal configuration. A new user should be dictating within 60 seconds of installation.

4. **Privacy by Design:** Local-first architecture. Cloud features are opt-in. Audio is NEVER stored by default. All cloud transmission uses TLS 1.3.

5. **Graceful Degradation:** If cloud is unavailable, fall back to local. If the preferred mic disconnects, fall back to the default device. The app should NEVER hard-fail silently.

---

## 4. User Personas & Use Cases

### 4.1 Primary Personas

**Persona 1: "Developer Dana"**
- Full-stack developer, 32, uses VS Code, Terminal, Slack
- Pain Points: RSI from constant typing, context switching breaks flow state
- Needs: Fast code dictation, commit message generation, documentation writing
- Usage: 20-30 dictation sessions/day, technical vocabulary

**Persona 2: "Writer Walt"**
- Technical writer, 45, writes 3,000+ words/day
- Pain Points: Writer's block from typing friction, repetitive strain injuries
- Needs: Long-form dictation, automatic formatting, distraction-free workflow
- Usage: 2-3 hour continuous sessions, creative flow state critical

**Persona 3: "Accessibility Alex"**
- Software engineer with motor impairment, 28
- Pain Points: Traditional keyboard input is difficult, needs voice control
- Needs: Reliable voice commands, accurate transcription, hands-free operation
- Usage: All-day use, essential for employment

**Persona 4: "Support Sam"**
- Customer support agent, 29, handles 50+ tickets/day
- Pain Points: Repetitive typing, maintaining tone consistency
- Needs: Snippet expansion, tone adjustment, rapid response templates
- Usage: High-frequency short dictations throughout shift

### 4.2 Core Use Cases (Priority Ordered)

| ID | Use Case | Priority | Phase |
|---|---|---|---|
| UC-001 | Trigger dictation via global hotkey in any app | P0 | 1 |
| UC-002 | Real-time transcription with auto-punctuation | P0 | 1 |
| UC-003 | Automatic filler word removal | P0 | 1 |
| UC-004 | Switch between cloud (Groq) and local modes | P0 | 1 |
| UC-005 | Offline operation with local models | P0 | 2 |
| UC-006 | Voice commands for formatting ("new paragraph", "period") | P1 | 2 |
| UC-007 | Custom vocabulary / personal dictionary | P1 | 2 |
| UC-008 | Snippet shortcuts for frequently used text | P1 | 3 |
| UC-009 | Multi-language support with auto-detection | P1 | 2 |
| UC-010 | Accessibility features (voice control, screen reader support) | P1 | 3 |
| UC-011 | Transcription history with search | P1 | 2 |
| UC-012 | Context awareness (read surrounding text for better accuracy) | P2 | 3+ |

---

## 5. Functional Requirements

This section describes WHAT the application must do. Implementation details for HOW are in later sections.

### 5.1 Core Dictation Flow

The fundamental user interaction is:

```
1. User presses global hotkey (e.g., Ctrl+Win)
2. Visual/audio feedback confirms recording has started (tray icon animates)
3. User speaks into microphone
4. Audio is captured, resampled to 16kHz mono PCM
5. Voice Activity Detection (VAD) segments speech from silence
6. Audio chunks are sent to the active STT engine (cloud or local)
7. Transcribed text is post-processed (formatting rules, filler removal)
8. Text is injected into the currently focused application
9. User releases hotkey OR silence timeout triggers end of dictation
10. Visual feedback confirms dictation complete
```

### 5.2 Global Hotkey Activation

- **Default hotkey:** `Ctrl + Win` (Windows), `Ctrl + Cmd` (macOS), `Ctrl + Super` (Linux)
- **MUST** work regardless of which application currently has focus
- **MUST** be user-configurable to any key combination
- **MUST** provide visual feedback (tray icon animation) and optional audio feedback on activation
- **MUST** prevent OS default behaviors (e.g., Windows Start Menu opening on Win key release)
- **MUST** support both "hold to record" and "toggle" modes (user-configurable)

### 5.3 Audio Capture

- **MUST** capture audio at microphone's native sample rate and resample to 16kHz, 16-bit mono PCM
- **MUST** allow user to select specific audio input device in settings
- **MUST** detect and handle audio device hot-plugging (mic disconnected/reconnected)
- **MUST** support automatic gain control and noise suppression
- Audio capture latency **MUST** be < 100ms

### 5.4 Transcription

- **MUST** support at minimum two STT modes: Cloud (Groq API) and Local (SenseVoice or Whisper.cpp)
- Cloud mode **MUST** use pseudo-streaming (audio chunking with overlap) for real-time feel
- Local mode **MUST** work with zero network connectivity
- **MUST** auto-punctuate transcribed text (periods, commas, question marks)
- **MUST** remove filler words ("um", "uh", "like") automatically
- **MUST** support at minimum English out of the box, with 50+ languages via configuration

### 5.5 Text Injection

- Transcribed text **MUST** be injected into the currently focused application's text field
- **MUST** use a dual-strategy approach: keystrokes for short text, clipboard for long text
- **MUST** maintain the target application's focus during injection
- **MUST** work in all standard text input fields across all major applications
- Text injection latency **MUST** be < 50ms for short text

### 5.6 Settings & Configuration

The settings panel **MUST** provide controls for:
- Hotkey customization (visual key capture)
- Audio input device selection (with device test)
- STT mode toggle (Cloud / Local / Hybrid / Auto)
- Language selection
- Personal dictionary management (add/remove/import/export)
- Snippet shortcuts management
- Output formatting options
- Privacy settings (data retention period, telemetry opt-out)
- Notification preferences
- Auto-start on login toggle
- Text injection method selection (Keystrokes / Clipboard / Auto)

### 5.7 Transcription History

- **MUST** store transcription history in a local SQLite database
- **MUST** support full-text search
- **MUST** allow copy/paste from history
- **MUST** support export (JSON, CSV, TXT)
- **MUST** have configurable retention period (default: 30 days)
- **MUST** encrypt history at rest with AES-256

### 5.8 Error Handling & Notifications

All errors **MUST** be communicated to the user via OS-native toast notifications (via `tauri-plugin-notification`). The application **MUST NEVER** fail silently.

| Scenario | Notification Type | Message Example |
|---|---|---|
| API Timeout | Error | "Cloud service unavailable. Switched to local mode." |
| API Key Invalid | Error | "Invalid API key. Check Settings > Cloud Provider." |
| Model Downloaded | Success | "Local model ready. Offline mode available." |
| Microphone Disconnected | Warning | "Microphone disconnected. Switched to [Default Device]." |
| No Microphone Available | Error | "No microphone found. Dictation paused." |
| Update Available | Info | "Version X.Y.Z ready to install." |
| Long Processing | Info | "Processing long transcription..." |
| Dictation Complete | Success (optional) | "Text inserted successfully." |

---

## 6. Technical Architecture

### 6.1 Why Tauri v2 (Not Electron)

The framework decision is **Tauri v2**. This is final and non-negotiable. The reasons are:

| Criteria | Tauri v2 | Electron |
|---|---|---|
| Bundle Size | 15-20MB | 150-400MB |
| Idle RAM | 10-30MB | 300-800MB |
| Startup Time | < 1 second | 3-10 seconds |
| Backend Language | Rust (systems-level) | Node.js (scripting) |
| Global Hotkeys | Native plugin | Requires native modules |
| Audio Capture | Native via `cpal` | Requires native modules (fragile) |
| Text Injection | Native via `enigo` | Requires `robotjs` (often broken) |
| Security Model | Capability-based (deny by default) | Standard Node.js (allow by default) |
| Auto-Updater | Built-in with ECDSA signing | Community package |
| Mobile Support | Built-in (iOS, Android) | None |

**Architecture:**

```
Tauri = System WebView (~5MB, already on user's system) + Rust Binary (~10MB) = 15MB total
Electron = Chromium (~120MB) + Node.js (~30MB) + App Code = 150MB+ total
```

### 6.2 High-Level Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     FRONTEND (React + TypeScript)            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Settings │  │Onboarding│  │ History  │  │ Snippets │    │
│  │  Panel   │  │  Wizard  │  │  Viewer  │  │ Manager  │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
└───────┼──────────────┼──────────────┼──────────────┼────────┘
        │              │              │              │
        └──────────────┴──────┬───────┴──────────────┘
                              │
                    Tauri IPC Bridge (secure, typed)
                              │
┌─────────────────────────────┼───────────────────────────────┐
│                     RUST BACKEND                             │
│                                                              │
│  ┌─────────────────┐     ┌─────────────────────┐            │
│  │  Global Hotkey   │     │  System Tray Manager │            │
│  │  Listener        │     │  (tray icon, menu)   │            │
│  └────────┬────────┘     └──────────────────────┘            │
│           │                                                  │
│  ┌────────▼─────────────────────────────────┐                │
│  │          Audio Pipeline                   │                │
│  │  ┌─────────┐  ┌──────────┐  ┌─────────┐  │                │
│  │  │ Capture │→ │Resample  │→ │  VAD    │  │                │
│  │  │ (cpal)  │  │(rubato)  │  │(Silero) │  │                │
│  │  └─────────┘  └──────────┘  └────┬────┘  │                │
│  └──────────────────────────────────┼────────┘                │
│                                     │                        │
│              ┌──────────────────────┼──────────────┐          │
│              │                      │              │          │
│         ┌────▼────┐          ┌──────▼─────┐  ┌─────▼──────┐  │
│         │ Cloud   │          │   Local    │  │  Provider  │  │
│         │ (Groq)  │          │(SenseVoice │  │  Selector  │  │
│         │         │          │  Sidecar)  │  │  (fallback │  │
│         └────┬────┘          └──────┬─────┘  │   chain)   │  │
│              │                      │        └────────────┘  │
│              └──────────┬───────────┘                        │
│                         │                                    │
│              ┌──────────▼──────────┐                          │
│              │  Formatting Engine  │                          │
│              │  (voice commands,   │                          │
│              │   filler removal)   │                          │
│              └──────────┬──────────┘                          │
│                         │                                    │
│              ┌──────────▼──────────┐                          │
│              │   Text Injector     │                          │
│              │   (enigo / clipboard)│                          │
│              └─────────────────────┘                          │
│                                                              │
│  ┌────────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  Notification  │  │   Model      │  │  Sensitivity     │  │
│  │  Manager       │  │   Download   │  │  Detector        │  │
│  │                │  │   Manager    │  │  (privacy mode)  │  │
│  └────────────────┘  └──────────────┘  └──────────────────┘  │
└──────────────────────────────────────────────────────────────┘
         │
         │ Sidecar Process (child process, sandboxed)
         │
┌────────▼─────────────────────────────────────────────────────┐
│  LOCAL STT SIDECAR (Sherpa-ONNX + SenseVoice-Small)          │
│  - Standalone executable, compiled per target triple          │
│  - Communicates via localhost HTTP or stdin/stdout pipes       │
│  - INT8 quantized model (~200MB), downloaded on-demand        │
│  - Launched on "Local Mode" toggle, terminated on exit        │
└──────────────────────────────────────────────────────────────┘
```

### 6.3 Project Structure

```
kalam/
├── .github/
│   ├── workflows/
│   │   ├── build.yml              # CI build on PR
│   │   └── release.yml            # Release automation on tag
│   ├── FUNDING.yml                # GitHub Sponsors config
│   └── ISSUE_TEMPLATE/
├── .doc/                          # Research & planning (this folder)
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                # Tauri app entry point
│   │   ├── lib.rs                 # Library root
│   │   ├── audio/
│   │   │   ├── mod.rs             # Audio module root
│   │   │   ├── capture.rs         # Audio capture via cpal
│   │   │   ├── resample.rs        # Resampling via rubato
│   │   │   ├── vad.rs             # Voice Activity Detection (Silero)
│   │   │   └── device.rs          # Device management & hot-plug
│   │   ├── stt/
│   │   │   ├── mod.rs             # STT module root, trait definition
│   │   │   ├── provider.rs        # STTProvider trait & factory
│   │   │   ├── groq.rs            # Groq API integration
│   │   │   ├── openai.rs          # OpenAI Whisper API
│   │   │   ├── whisper_cpp.rs     # Whisper.cpp local (via whisper-rs)
│   │   │   ├── sensevoice.rs      # SenseVoice sidecar manager
│   │   │   └── selector.rs        # Provider selection & fallback logic
│   │   ├── injection/
│   │   │   ├── mod.rs             # Text injection module root
│   │   │   ├── keystroke.rs       # Keystroke injection via enigo
│   │   │   ├── clipboard.rs       # Clipboard injection with restore
│   │   │   └── auto.rs            # Auto-selection logic
│   │   ├── formatting/
│   │   │   ├── mod.rs             # Formatting engine root
│   │   │   ├── rules.rs           # Regex-based formatting rules
│   │   │   ├── filler.rs          # Filler word removal
│   │   │   └── commands.rs        # Voice command processing
│   │   ├── privacy/
│   │   │   ├── mod.rs             # Privacy module root
│   │   │   ├── sensitivity.rs     # Sensitive app detection
│   │   │   └── encryption.rs      # AES-256 encryption for local data
│   │   ├── models/
│   │   │   ├── mod.rs             # Model management root
│   │   │   └── download.rs        # Model download manager
│   │   ├── config/
│   │   │   ├── mod.rs             # Configuration management
│   │   │   ├── settings.rs        # User settings struct
│   │   │   └── hotkey.rs          # Hotkey configuration
│   │   ├── history/
│   │   │   ├── mod.rs             # History module root
│   │   │   └── db.rs              # SQLite history database
│   │   ├── update/
│   │   │   └── mod.rs             # Auto-update logic
│   │   └── tray/
│   │       └── mod.rs             # System tray setup & management
│   ├── binaries/                  # Sidecar executables (per platform)
│   ├── capabilities/
│   │   └── default.json           # Tauri capability permissions
│   ├── icons/                     # App icons (all platforms)
│   ├── Cargo.toml                 # Rust dependencies
│   └── tauri.conf.json            # Tauri configuration
├── src/                           # React frontend
│   ├── App.tsx                    # Root React component
│   ├── main.tsx                   # React entry point
│   ├── pages/
│   │   ├── Settings.tsx           # Settings panel
│   │   ├── History.tsx            # Transcription history viewer
│   │   ├── Snippets.tsx           # Snippet manager
│   │   └── Onboarding.tsx         # First-run wizard
│   ├── components/                # Reusable UI components
│   ├── hooks/                     # Custom React hooks
│   ├── stores/                    # State management
│   └── styles/                    # CSS/Tailwind styles
├── docs/                          # User-facing documentation
│   ├── INSTALL.md
│   ├── USAGE.md
│   └── API.md
├── website/                       # Landing page (GitHub Pages)
├── package.json
├── tsconfig.json
├── PRD.md                         # THIS DOCUMENT
└── README.md
```

### 6.4 Tauri Configuration

The `tauri.conf.json` must include:

```jsonc
{
  "productName": "Kalam",
  "version": "0.1.0",
  "identifier": "com.kalam.voice",
  "build": {
    "frontendDist": "../dist"
  },
  "app": {
    "withGlobalTauri": true,
    "trayIcon": {
      "iconPath": "icons/tray-icon.png",
      "iconAsTemplate": true
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "externalBin": ["binaries/sensevoice-server"],
    "resources": ["models/*"]
  },
  "plugins": {
    "updater": {
      "pubkey": "<GENERATED_PUBLIC_KEY>",
      "endpoints": [
        "https://<org>.github.io/kalam/latest.json"
      ],
      "windows": {
        "installMode": "passive"
      }
    }
  }
}
```

The `capabilities/default.json` must explicitly whitelist ONLY the needed permissions:

```json
{
  "permissions": [
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "notification:default",
    "shell:allow-execute",
    "updater:default",
    "sql:default"
  ]
}
```

### 6.5 Security Architecture

| Threat | Mitigation |
|---|---|
| Audio interception in transit | Local processing option; TLS 1.3 for all cloud calls |
| Audio interception at rest | Audio is NEVER stored by default; in-memory only |
| Keylogging | Secure IPC only; no keystroke logging; enigo writes but never reads |
| Malicious updates | ECDSA signatures on all binaries; public key embedded at compile time |
| Unauthorized IPC access | Tauri capability-based permission system (deny by default) |
| Data leakage | Zero-data retention option; local-only mode; AES-256 for stored history |
| Sidecar tampering | Sidecar binaries whitelisted in capabilities; argument validation |
| Supply chain attacks | Audited Rust crate dependencies; minimal npm surface area |

---

## 7. Audio Pipeline

### 7.1 Audio Capture (cpal)

The audio pipeline is the critical path from microphone to STT engine. Every millisecond matters.

**Implementation Requirements:**

1. Use the `cpal` crate for cross-platform audio I/O
2. Capture audio at the device's native sample rate (typically 44.1kHz or 48kHz)
3. Write captured samples to a lock-free ring buffer (e.g., `ringbuf` crate) to avoid blocking the audio thread
4. The audio callback thread **MUST NOT** perform any blocking operations (no allocations, no mutex locks, no I/O)

**Platform-specific audio backends:**
- Windows: WASAPI (via cpal)
- macOS: CoreAudio (via cpal)
- Linux: ALSA / PulseAudio / JACK (via cpal, auto-detected)

**Audio format for STT engines:**
- Output: 16kHz, 16-bit, mono PCM
- All STT engines (Groq Whisper, SenseVoice, Whisper.cpp) require this format
- Resampling from native rate to 16kHz is performed via the `rubato` crate

### 7.2 Audio Device Management

The application **MUST** handle microphone hot-plugging gracefully:

```rust
pub struct AudioDeviceManager {
    current_device: Arc<Mutex<Option<Device>>>,
    fallback_device: Option<Device>,
    host: Host,
}
```

**Behavior:**

| Event | Action | User Notification |
|---|---|---|
| Device disconnected | Auto-switch to system default device | Toast: "Microphone disconnected. Switched to [Default]." |
| No devices available | Pause dictation, wait for reconnect | Toast: "No microphone found. Dictation paused." |
| Device reconnected | Optionally auto-switch back (user setting) | Toast: "Microphone [Name] reconnected." |
| New device plugged in | Add to device list in settings | No notification (silent) |

**Implementation:** Poll device availability every 2 seconds in a background thread. On device loss, call `host.default_input_device()` as fallback.

### 7.3 Voice Activity Detection (VAD)

VAD is used to segment continuous audio into speech chunks for the STT engine. This is critical for the pseudo-streaming experience over REST APIs.

**Engine:** Silero VAD (Rust port or via ONNX runtime)

**Configuration:**

```rust
pub struct VADConfig {
    /// Probability threshold for speech detection (0.0 - 1.0)
    pub speech_threshold: f32,         // Default: 0.5

    /// Seconds of silence before triggering chunk submission
    pub silence_timeout_sec: f64,      // Default: 1.5

    /// Minimum speech duration to process (filters coughs, etc.)
    pub min_speech_duration_sec: f64,  // Default: 0.25

    /// Maximum chunk duration before forcing submission
    pub max_chunk_duration_sec: f64,   // Default: 30.0

    /// Padding added before/after speech segments
    pub speech_padding_sec: f64,       // Default: 0.3
}
```

**Preset Modes (user-selectable in settings):**

| Mode | Silence Timeout | Best For |
|---|---|---|
| Fast | 0.8s | Quick commands, short dictations |
| Balanced | 1.5s | General dictation (default) |
| Accurate | 2.5s | Long-form writing, minimal interruption |

### 7.4 Audio Chunking & Overlap

For cloud STT (Groq API), which uses REST, not WebSocket streaming, the application must implement pseudo-streaming:

1. **VAD-based chunking:** Slice audio at natural conversational pauses detected by VAD
2. **Hard fallback:** If user speaks continuously for > `max_chunk_duration_sec` (30s), force a chunk boundary
3. **Overlap:** Consecutive chunks share 10-20% overlap (0.5-1s) to prevent word truncation at boundaries
4. **Prompt chaining:** When submitting chunk N to the API, include the transcription of chunk N-1 in the `prompt` parameter for context continuity

**Overlap math:** If Chunk 1 spans `[t0, t1]`, Chunk 2 captures from `[t1 - overlap, t2]` where `overlap = (t1 - t0) * 0.15`.

---

## 8. Speech-to-Text Engine

### 8.1 Provider-Agnostic Interface

ALL STT providers **MUST** implement a common trait. This enables swapping providers, fallback chains, and future extensibility:

```rust
#[async_trait]
pub trait STTProvider: Send + Sync {
    /// Transcribe a single audio chunk
    async fn transcribe(&self, audio: AudioChunk) -> Result<TranscriptionResult>;

    /// Transcribe with streaming (if supported)
    async fn transcribe_stream(&self, stream: AudioStream) -> Result<TranscriptionStream>;

    /// Whether this provider supports streaming
    fn supports_streaming(&self) -> bool;

    /// Whether this provider requires internet
    fn requires_internet(&self) -> bool;

    /// Expected latency for planning
    fn estimated_latency(&self) -> Duration;

    /// Human-readable provider name
    fn name(&self) -> &str;
}

pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub words: Vec<WordInfo>,
}

pub struct WordInfo {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub confidence: f32,
}
```

### 8.2 Provider Factory

```rust
pub struct STTProviderFactory;

impl STTProviderFactory {
    pub fn create(config: &ProviderConfig) -> Result<Box<dyn STTProvider>> {
        match config.provider_type {
            ProviderType::Groq => Ok(Box::new(GroqProvider::new(&config.api_key)?)),
            ProviderType::OpenAI => Ok(Box::new(OpenAIProvider::new(&config.api_key)?)),
            ProviderType::WhisperCpp => Ok(Box::new(WhisperCppProvider::new(&config.model_path)?)),
            ProviderType::SenseVoice => Ok(Box::new(SenseVoiceProvider::new(&config.model_path)?)),
            ProviderType::AssemblyAI => Ok(Box::new(AssemblyAIProvider::new(&config.api_key)?)),
            // Future providers added here
        }
    }
}
```

### 8.3 Provider Selection & Fallback

```rust
pub struct ProviderSelector {
    providers: Vec<Box<dyn STTProvider>>,
    config: STTConfig,
}

impl ProviderSelector {
    pub async fn transcribe(&self, audio: AudioChunk) -> Result<TranscriptionResult> {
        // 1. If sensitivity detector says ForceLocal, use local only
        // 2. If user explicitly set "Local Only", use local
        // 3. If user set "Cloud", try cloud first
        // 4. If user set "Auto/Hybrid", try cloud → fall back to local
        // 5. On cloud failure, fall back to local and notify user
    }
}
```

### 8.4 Cloud Provider: Groq API (Default)

**Endpoint:** `POST https://api.groq.com/openai/v1/audio/transcriptions`

**Models:**

| Model | Price | Speed | Use Case |
|---|---|---|---|
| `whisper-large-v3-turbo` (default) | $0.04/hour | ~300ms | Real-time dictation |
| `whisper-large-v3` | $0.111/hour | ~500ms | Maximum accuracy |
| `distil-whisper-large-v3` | $0.04/hour | ~200ms | Speed-optimized |

**Integration:**

```rust
pub struct GroqProvider {
    api_key: String,
    client: reqwest::Client,
    model: String,  // Default: "whisper-large-v3-turbo"
}

impl GroqProvider {
    pub async fn transcribe(&self, audio: &[u8]) -> Result<TranscriptionResult> {
        let form = reqwest::multipart::Form::new()
            .part("file", Part::bytes(audio.to_vec())
                .file_name("audio.wav")
                .mime_str("audio/wav")?)
            .text("model", self.model.clone())
            .text("language", "auto")
            .text("response_format", "verbose_json");

        let response = self.client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        // Parse and return TranscriptionResult
    }
}
```

**Prompt chaining:** For each chunk after the first, include the previous chunk's transcription in the `prompt` field. This tells Whisper what came before and prevents:
- Hallucinated punctuation at chunk boundaries
- Inconsistent capitalization
- Lost context for domain-specific terms

### 8.5 Local Provider: SenseVoice (Primary) via Sherpa-ONNX Sidecar

**Why SenseVoice over Whisper.cpp:**

| Feature | Whisper-Small | SenseVoice-Small |
|---|---|---|
| Architecture | Autoregressive (token-by-token) | Non-autoregressive (encoder-only) |
| Speed (10s audio) | ~350ms+ | ~70ms |
| Languages | 99 | 50+ |
| Built-in Features | ASR only | ASR + VAD + Emotion + Audio Events |
| Punctuation | Needs post-processing | Native via ITN (use_itn=true) |
| Model Size (INT8) | ~466MB (small) | ~200MB |

**Sidecar Architecture:**

1. A standalone Sherpa-ONNX inference server is compiled as a native executable for each target platform
2. The executable is placed in `src-tauri/binaries/` with platform-specific naming (e.g., `sensevoice-server-x86_64-pc-windows-msvc.exe`)
3. Tauri's `Command::new_sidecar("sensevoice-server").spawn()` launches it as a child process
4. Communication is via localhost HTTP (e.g., `http://127.0.0.1:10095`) or stdin/stdout pipes
5. The sidecar and its arguments **MUST** be whitelisted in `capabilities/default.json`
6. When the user switches to cloud mode, the sidecar process is terminated to free resources

**SenseVoice Configuration:**

```rust
pub struct SenseVoiceConfig {
    pub use_itn: bool,          // true — enables punctuation & text normalization
    pub language: String,       // "auto" for auto-detection
    pub ban_emo_unk: bool,      // false — allow emotion tokens
    pub model_path: PathBuf,    // Path to INT8 .onnx model file
}
```

**Important:** SenseVoice handles punctuation NATIVELY via ITN (Inverse Text Normalization). No additional punctuation model is needed. ITN converts:
- "two thousand twenty six" → "2026"
- "five dollars" → "$5"
- Automatically inserts periods, commas, question marks

### 8.6 Local Provider: Whisper.cpp (Alternative)

Whisper.cpp is offered as an alternative local provider, especially for English-only users who prefer Whisper's accuracy profile.

**Integration via `whisper-rs` crate:**

```rust
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams};

pub struct WhisperCppProvider {
    ctx: WhisperContext,
}

impl WhisperCppProvider {
    pub fn new(model_path: &str) -> Result<Self> {
        let ctx = WhisperContext::new_with_params(
            model_path,
            WhisperContextParameters::default()
        )?;
        Ok(Self { ctx })
    }

    pub fn transcribe(&self, audio: &[f32]) -> Result<String> {
        let mut params = FullParams::new(Default::default());
        params.set_language(Some("auto"));
        params.set_translate(false);

        let mut state = self.ctx.create_state()?;
        state.full(params, audio)?;

        let num_segments = state.full_n_segments()?;
        let mut result = String::new();
        for i in 0..num_segments {
            result.push_str(&state.full_get_segment_text(i)?);
        }
        Ok(result)
    }
}
```

**Available Whisper.cpp models:**

| Model | Disk | RAM | Real-time Factor | Quality |
|---|---|---|---|---|
| tiny | 75MB | 273MB | Real-time | Basic |
| base | 142MB | 388MB | Real-time | Good |
| small | 466MB | 852MB | Real-time | Very Good |
| medium | 1.5GB | 2.1GB | 0.5x real-time | Excellent |
| large-v3 | 2.9GB | 3.9GB | 0.3x real-time | Best |

**Recommendation:** Ship Whisper.cpp base (142MB) as the default local Whisper option. Users can download larger models on-demand.

### 8.7 Future Cloud Providers (Phase 3+)

The provider-agnostic architecture allows adding these later:

| Provider | Latency | Price/Hour | Languages | Free Tier |
|---|---|---|---|---|
| OpenAI Whisper | 1-2s | $0.36 | 99 | None |
| Google Cloud STT | 2-5s | $1.44 | 125 | 60min/mo |
| Azure Speech | 1-3s | $1.00 | 100 | 5hr/mo |
| AssemblyAI | 500ms | $0.37 | 50+ | 100hr one-time |
| Deepgram | 300ms | $0.26 | 30+ | $200 credit |
| AWS Transcribe | 2-4s | $1.44 | 100 | 60min/mo |

---

## 9. Text Injection System

### 9.1 Dual-Strategy Architecture

Text injection is one of the most critical and fragile parts of the system. The application **MUST** use a dual-strategy approach:

```rust
pub enum InjectionMethod {
    Keystrokes,  // Character-by-character via enigo (best for short text)
    Clipboard,   // Save clipboard → set text → Ctrl+V → restore clipboard
    Auto,        // Intelligent selection based on text length (DEFAULT)
}

pub struct InjectionConfig {
    pub method: InjectionMethod,          // Default: Auto
    pub keystroke_delay_ms: u64,          // Default: 10ms between keystrokes
    pub clipboard_threshold: usize,       // Default: 50 characters
    pub retry_attempts: u32,              // Default: 3
    pub retry_delay_ms: u64,              // Default: 100ms
}
```

### 9.2 Keystroke Injection (Primary, Short Text)

- Uses the `enigo` crate (v0.2.1+)
- Platform support:
  - Windows: Win32 `SendInput` API — full support
  - macOS: `CGEvent` API — full support
  - Linux X11: `XTest` extension — full support
  - Linux Wayland: Experimental (via dbus/virtual-keyboard protocol)
- Good for text < 50 characters
- Preserves target application formatting context
- Latency: < 50ms for short strings

### 9.3 Clipboard Injection (Fallback, Long Text)

For text longer than `clipboard_threshold` characters, keystroke injection is too slow and fragile. Use clipboard injection:

```rust
impl TextInjector {
    async fn inject_via_clipboard(&self, text: &str) -> Result<()> {
        // 1. Save current clipboard contents
        let old_clipboard = clipboard::get_text()?;

        // 2. Set transcribed text to clipboard
        clipboard::set_text(text)?;

        // 3. Simulate Ctrl+V (or Cmd+V on macOS)
        enigo.key_down(Key::Control);
        enigo.key_click(Key::V);
        enigo.key_up(Key::Control);

        // 4. Wait for paste to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 5. Restore original clipboard contents
        clipboard::set_text(&old_clipboard)?;

        Ok(())
    }
}
```

### 9.4 Auto Mode Logic

```rust
fn select_method(text: &str, config: &InjectionConfig) -> InjectionMethod {
    if text.len() > config.clipboard_threshold {
        InjectionMethod::Clipboard
    } else {
        InjectionMethod::Keystrokes
    }
}
```

### 9.5 Retry Mechanism

Both methods **MUST** implement retry logic (3 attempts by default) because:
- A notification may steal focus during injection
- The user may click away
- Some applications have input throttling

On final failure, show an error notification and offer to copy text to clipboard.

### 9.6 Platform Limitations

| Platform | Keystrokes | Clipboard | Notes |
|---|---|---|---|
| Windows | Full | Full | Works in all apps |
| macOS | Full | Full | Requires Accessibility permission |
| Linux X11 | Full | Full | Primary target |
| Linux Wayland | Limited | Full | Keystrokes depend on compositor |

---

## 10. User Interface Requirements

### 10.1 System Tray (Primary Interface)

The system tray is the ONLY persistent UI element. There is no main application window.

**Tray Icon States:**

| State | Icon Appearance | Meaning |
|---|---|---|
| Idle | Static icon | App is running, ready for hotkey |
| Recording | Animated/pulsing icon | Microphone is active, listening |
| Processing | Spinner animation | Transcription in progress |
| Error | Red indicator | Something is wrong (check notifications) |

**Right-Click Context Menu:**

```
┌───────────────────────┐
│ Kalam                 │
├───────────────────────┤
│ ● Cloud Mode   ▸     │  → Groq (active), OpenAI, Local
│ ○ Local Mode         │
│ ○ Hybrid Mode        │
├───────────────────────┤
│ 🎤 Audio: MacBook Mic │
├───────────────────────┤
│ Settings...           │
│ History...            │
│ Snippets...           │
├───────────────────────┤
│ Check for Updates     │
│ About                 │
│ Quit                  │
└───────────────────────┘
```

### 10.2 First-Run Onboarding Wizard

On first launch, the application **MUST** present a guided setup wizard:

**Step 1: Welcome**
- Brief value proposition
- Privacy commitment statement

**Step 2: Permissions**
- Request microphone access
- Request accessibility permissions (macOS)
- Test microphone with visual audio level meter

**Step 3: Mode Selection**
- Cloud Mode: Prompt for API key (link to Groq signup)
- Local Mode: Trigger model download (200MB for SenseVoice, or 142MB for Whisper base)
- Hybrid Mode: Both (recommended)

**Step 4: Hotkey Configuration**
- Visual key capture UI
- Default: Ctrl+Win / Ctrl+Cmd
- Option to choose "hold to record" or "toggle" mode

**Step 5: Quick Tutorial**
- 30-second interactive demo
- User tries dictating "Hello, this is a test."
- Confirm text appears in a sample text field

**Step 6: Done**
- App minimizes to system tray
- "You're ready! Press [hotkey] anytime to start dictating."

### 10.3 Settings Panel

A React-based window opened from the tray menu. Organized into tabs:

**Tab: General**
- Hotkey configuration (visual key capture)
- Start on login toggle
- Recording mode (Hold / Toggle)
- Language selection dropdown
- VAD sensitivity preset (Fast / Balanced / Accurate)

**Tab: Audio**
- Input device selection (dropdown with test button)
- Automatic gain control toggle
- Noise suppression toggle

**Tab: STT Provider**
- Mode selection (Cloud / Local / Hybrid)
- Cloud provider dropdown (Groq, OpenAI, custom)
- API key input with validation button
- Local model selection with download manager

**Tab: Text Injection**
- Injection method (Auto / Keystrokes / Clipboard)
- Keystroke delay slider
- Clipboard threshold slider

**Tab: Formatting**
- Voice commands enable/disable
- Filler word removal toggle
- Custom formatting rules (regex editor)
- Personal dictionary management

**Tab: Snippets**
- Snippet list (trigger phrase → expanded text)
- Add/Edit/Delete
- Import/Export (JSON)

**Tab: Privacy**
- History retention period (days)
- Telemetry opt-in/opt-out
- Sensitive app patterns editor
- Clear all data button

**Tab: About**
- Version info
- Update check button
- Links: GitHub, documentation, sponsors

### 10.4 Local Model Download Manager

When the user enables local mode for the first time, they need to download the model:

```
┌─────────────────────────────────────────┐
│ SenseVoice Small (Recommended)          │
│ Size: 200 MB │ Status: Not Installed    │
│                                         │
│ [████████████████████░░░░] 78%          │
│ 156 MB / 200 MB — 2 min remaining      │
│                                         │
│ [Pause] [Cancel]                        │
├─────────────────────────────────────────┤
│ Whisper Base (Alternative)              │
│ Size: 142 MB │ Status: Not Installed    │
│                                         │
│ [Download] [Learn More]                 │
├─────────────────────────────────────────┤
│ Whisper Small (High Quality)            │
│ Size: 466 MB │ Status: Not Installed    │
│                                         │
│ [Download] [Learn More]                 │
└─────────────────────────────────────────┘
```

**Download Manager Requirements:**
- Progress tracking with visual indicator
- Pause/resume support
- Background download capability (user can close settings)
- Auto-retry on failure (3 attempts)
- SHA-256 checksum verification after download
- Download sources: HuggingFace (primary), GitHub Releases (mirror)
- Storage: Platform-appropriate app data directory
  - Windows: `%APPDATA%/com.kalam.voice/models/`
  - macOS: `~/Library/Application Support/com.kalam.voice/models/`
  - Linux: `~/.local/share/com.kalam.voice/models/`

---

## 11. Voice Commands & Formatting Engine

### 11.1 Architecture

Voice commands are processed CLIENT-SIDE using regex pattern matching. This is deliberately NOT sent to an LLM because:
- Regex processing takes < 10ms (vs 100-500ms for LLM)
- Commands are deterministic (no ambiguity)
- Works offline
- More reliable

### 11.2 Processing Pipeline

```
Raw STT Output → Formatting Engine → Text Injector
```

The formatting engine runs as a post-processing step AFTER the STT engine returns text and BEFORE text injection.

### 11.3 Implementation

```rust
pub struct FormattingEngine {
    rules: Vec<FormattingRule>,
}

pub struct FormattingRule {
    pub pattern: Regex,
    pub replacement: Replacement,
    pub enabled: bool,
}

pub enum Replacement {
    Text(String),                           // Simple text substitution
    Function(Box<dyn Fn(&Captures) -> String>),  // Dynamic replacement
    Action(Action),                         // Side-effect action
}

pub enum Action {
    DeleteLast,          // "delete that" — delete last injected text
    DeleteLastSentence,  // "scratch that" — delete last sentence
    Undo,                // "undo" — Ctrl+Z
}
```

### 11.4 Default Voice Commands

**Punctuation:**

| Voice Command | Output |
|---|---|
| "period" / "full stop" | `.` |
| "comma" | `,` |
| "question mark" | `?` |
| "exclamation mark" / "exclamation point" | `!` |
| "colon" | `:` |
| "semicolon" | `;` |
| "dash" / "hyphen" | `-` |
| "open quote" | `"` |
| "close quote" | `"` |
| "open parenthesis" | `(` |
| "close parenthesis" | `)` |

**Formatting:**

| Voice Command | Output |
|---|---|
| "new line" | `\n` |
| "new paragraph" | `\n\n` |
| "tab" | `\t` |
| "capitalize [word]" | Capitalizes the next word |
| "all caps [word]" | UPPERCASES the next word |

**Editing:**

| Voice Command | Action |
|---|---|
| "delete that" | Deletes the last injected text |
| "scratch that" | Deletes the last sentence |
| "undo" | Simulates Ctrl+Z / Cmd+Z |

**Mode:**

| Voice Command | Action |
|---|---|
| "switch to local mode" | Switches STT to local |
| "switch to cloud mode" | Switches STT to cloud |

### 11.5 Extensibility

- Users can add custom regex rules via the Settings > Formatting tab
- Rules are stored as JSON and can be imported/exported
- Pre-built rule packs for specific domains (legal, medical, coding) planned for Phase 3

### 11.6 Filler Word Removal

A separate, always-on formatting pass removes common filler words:
- "um", "uh", "er", "ah", "like" (when used as filler), "you know", "I mean", "basically", "actually" (when used as filler)
- This is applied via regex with word boundary matching
- User-configurable: can be disabled in settings

---

## 12. Privacy, Security & Data Handling

### 12.1 Data Classification

| Data Type | Storage Location | Retention | Encryption | Cloud Transmission |
|---|---|---|---|---|
| Audio recordings | NEVER stored | Real-time only, in-memory | N/A | TLS 1.3 (cloud mode only) |
| Transcribed text (history) | Local SQLite | User-configurable (default: 30 days) | AES-256 at rest | Never |
| Personal dictionary | Local file | Persistent | AES-256 at rest | Never |
| Snippet data | Local file | Persistent | None (non-sensitive) | Never |
| User settings | Local file | Persistent | None (non-sensitive) | Never |
| API keys | Local encrypted store | Persistent | AES-256 at rest | TLS 1.3 (to API only) |
| Telemetry (opt-in) | PostHog Cloud | 90 days | Anonymized | TLS 1.3 |

### 12.2 Privacy Modes

**Cloud Mode:** Audio is sent to the configured cloud STT provider (e.g., Groq). Encrypted via TLS 1.3 in transit. Groq states zero data retention after processing.

**Local Mode:** ALL processing happens on-device. Zero network transmission. Complete air-gap privacy.

**Hybrid Mode:** Automatically switches between cloud and local based on:
1. Network availability (offline → local)
2. Sensitive application detection (see below)
3. User-defined rules

### 12.3 Sensitive Application Detection (Hybrid Mode)

When hybrid mode is active, the application detects sensitive foreground applications and AUTOMATICALLY forces local mode:

```rust
pub struct SensitivityDetector {
    patterns: Vec<SensitivityPattern>,
}

pub struct SensitivityPattern {
    pub pattern_type: PatternType,  // ProcessName, WindowTitle, BundleId
    pub pattern: String,            // Regex pattern
    pub action: PrivacyAction,      // ForceLocal, Block, RequireConfirmation
}
```

**Default Patterns (shipped with app):**

| Pattern | Type | Action | Description |
|---|---|---|---|
| `(?i)(1password\|bitwarden\|keepass\|lastpass\|dashlane)` | ProcessName | ForceLocal | Password managers |
| `(?i)(bank\|credit union\|payment\|transfer)` | WindowTitle | ForceLocal | Banking websites |
| `(?i)(ssh\|terminal\|iterm\|alacritty\|kitty)` | ProcessName | RequireConfirmation | Terminals (user decides) |
| `(?i)(metamask\|electrum\|exodus\|ledger\|trezor)` | ProcessName | ForceLocal | Crypto wallets |

**User controls:**
- Enable/disable auto-detection (default: enabled)
- Add custom patterns
- Override default patterns
- Choose default action per pattern

**Detection method:** Read the active window's process name and window title using OS APIs:
- macOS: Accessibility API (`AXUIElement`)
- Windows: UI Automation API
- Linux: `wmctrl` or AT-SPI2

### 12.4 Compliance Posture

| Regulation | Compliance Path |
|---|---|
| **GDPR** | Data export, right to deletion, consent management, opt-in telemetry |
| **HIPAA** | Local mode for healthcare users; BAA possible with Groq if applicable |
| **CCPA** | Data transparency, opt-out mechanisms for telemetry |

---

## 13. API Key Management & Monetization

### 13.1 BYOK-First Architecture

The default model is **Bring Your Own Key (BYOK)**. Users provide their own Groq API key. This means:
- Zero API costs for the Kalam project
- Users pay Groq directly ($0.04/hour — roughly $1.20/month for 1hr/day usage)
- No rate limiting from Kalam's side
- Full user control

### 13.2 API Key Source Hierarchy

```rust
pub enum ApiKeySource {
    /// User provides their own Groq/OpenAI/etc. key
    UserProvided { key: String },

    /// Kalam-hosted shared key for paid supporters
    Hosted {
        endpoint: String,
        rate_limit: RateLimit,
    },

    /// Enterprise self-hosted endpoint
    SelfHosted { endpoint: String },
}
```

### 13.3 Pricing Tiers

| Tier | Cost | Features | Target |
|---|---|---|---|
| **Free** | $0 | BYOK only, unlimited local usage | Individual users, developers |
| **Supporter** | $5/month (via GitHub Sponsors) | Kalam-hosted key, 10 hours cloud/month | Casual users wanting convenience |
| **Pro** | $15/month (via GitHub Sponsors) | Kalam-hosted key, 50 hours cloud/month | Power users, professionals |
| **Enterprise** | Custom | Self-hosted, unlimited, SLA, admin dashboard | Organizations |

### 13.4 Payment Infrastructure

- **Primary:** GitHub Sponsors (zero transaction fees, integrated with GitHub)
- **Alternative:** Stripe (for enterprise invoicing, custom billing)
- **Entitlement Validation:** GitHub Sponsors API to verify tier membership

### 13.5 Operational Cost Analysis

| Item | Annual Cost | Notes |
|---|---|---|
| Apple Developer Account | $99 | Required for macOS notarization (needed at v1.0) |
| Windows Code Signing | $0-200 | Self-signed for beta; certificate for v1.0 |
| Domain/Hosting | $0 | GitHub Pages for website, updates, docs |
| CI/CD | $0 | GitHub Actions (free for open source) |
| **Total** | **$99-299/year** | Within $5K/month donation target |

### 13.6 Sustainability Model

- 50K users with BYOK = $0 cloud cost to Kalam
- Target: $5,000/month via GitHub Sponsors for operational costs + maintainer compensation
- Enterprise tier provides additional revenue stream

---

## 14. Distribution, Updates & CI/CD

### 14.1 Installer Formats

| Platform | Primary Format | Secondary | Bundle Size |
|---|---|---|---|
| Windows x64 | `.msi` | `.exe` (NSIS portable) | ~15MB |
| Windows ARM64 | `.msi` | `.exe` | ~15MB |
| macOS Intel | `.dmg` | `.app` bundle | ~15MB |
| macOS Apple Silicon | `.dmg` | `.app` bundle | ~15MB |
| Linux x64 | `.AppImage` | `.deb` | ~15MB |
| Linux ARM64 | `.AppImage` | `.deb` | ~15MB |

### 14.2 Distribution Channels

| Channel | Priority | Timeline | Installation |
|---|---|---|---|
| GitHub Releases | P0 | Launch | Direct download |
| Official Website | P0 | Launch | Download portal |
| Homebrew (macOS) | P1 | Month 2 | `brew install kalam-voice` |
| Chocolatey (Windows) | P1 | Month 2 | `choco install kalam-voice` |
| winget (Windows) | P1 | Month 2 | `winget install KalamVoice` |
| AUR (Arch Linux) | P1 | Month 2 | `yay -S kalam-voice` |
| Flathub (Linux) | P2 | Month 3 | `flatpak install flathub com.kalam.voice` |
| Microsoft Store | P2 | Month 4 | Store listing |
| Mac App Store | P3 | Month 6 | If feasible with sandboxing |

### 14.3 CI/CD Pipeline (GitHub Actions)

**Trigger:** Git tag matching `v*.*.*` (semantic versioning)

**Pipeline Steps:**

1. **Environment Provisioning:** Check out repository, install OS-specific build dependencies (webkit2gtk, libasound2-dev for Linux, etc.)

2. **Dependency Caching:** Cache Rust `cargo` dependencies via `swatinem/rust-cache` and Node.js modules. This reduces build time from 60+ minutes to under 10 minutes on subsequent builds.

3. **Matrix Builds:** Run in parallel across:
   - `windows-latest` → x86_64-pc-windows-msvc → `.msi`, `.exe`
   - `macos-latest` → x86_64-apple-darwin + aarch64-apple-darwin → `.dmg`, `.app`
   - `ubuntu-22.04` → x86_64-unknown-linux-gnu → `.AppImage`, `.deb`

4. **Frontend Build:** `pnpm install && pnpm run build` (generates static web assets)

5. **Backend Build:** `tauri build` (compiles Rust binary, injects frontend, bundles sidecars, generates `.sig` files)

6. **Code Signing:**
   - macOS: Extract .p12 certificate from GitHub Secrets → create temporary keychain → `codesign` → submit to Apple notary service
   - Windows: Use Azure Key Vault credentials or certificate from Secrets → `signtool.exe`
   - Linux: GPG signing

7. **Artifact Upload:** Upload all installers + `.sig` files to GitHub Release (draft)

8. **Manifest Generation:** Custom script generates `latest.json` for the Tauri updater:
   - Parse GitHub API for asset download URLs
   - Read `.sig` file contents
   - Construct `latest.json` with version, URLs, signatures
   - Deploy to GitHub Pages

9. **Publish Release:** Convert draft release to published

### 14.4 Auto-Update Mechanism

**Architecture:** `tauri-plugin-updater` + GitHub Pages (zero server cost)

**Update Flow:**

```
1. App checks GitHub Pages for latest.json (daily, background)
2. Compares remote version against local version
3. If update available:
   a. Download update binary silently in background
   b. Verify ECDSA signature against embedded public key
   c. Show OS notification: "Update v1.2.0 ready to install"
   d. User clicks notification or defers (max 7 days for non-critical)
   e. On install:
      - Windows: "passive" mode (progress bar, no interaction, UAC auto-elevation)
      - macOS/Linux: User-initiated restart
   f. On failure: Automatic rollback to previous version
```

**Update Categories:**

| Type | User Prompt | Max Defer |
|---|---|---|
| Critical (security, crash fix) | Immediate notification | 24 hours |
| Feature (new functionality) | Standard notification | 7 days |
| Patch (bug fix, performance) | Weekly digest option | 14 days |

**Windows UAC Handling:**
- Install mode is set to `"passive"` in `tauri.conf.json`
- The installer requests UAC elevation automatically
- A before-exit hook saves user state before the required app restart
- This preserves the "silent background" design principle

**ECDSA Key Management:**
- Generate keypair: `tauri signer generate`
- Public key: embedded in `tauri.conf.json`, compiled into binary (safe to be public)
- Private key: stored ONLY as GitHub Secret (`TAURI_SIGNING_PRIVATE_KEY`)
- Every build artifact gets a `.sig` file signed with the private key

### 14.5 `latest.json` Schema

```json
{
  "version": "v1.2.0",
  "notes": "## What's New\n- Improved transcription accuracy\n- Fixed Windows hotkey issue",
  "pub_date": "2026-03-04T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<contents of .sig file>",
      "url": "https://github.com/kalam-voice/kalam/releases/download/v1.2.0/Kalam-Voice-1.2.0-x64.msi"
    },
    "darwin-aarch64": {
      "signature": "<contents of .sig file>",
      "url": "https://github.com/kalam-voice/kalam/releases/download/v1.2.0/Kalam-Voice-1.2.0-arm64.dmg"
    },
    "darwin-x86_64": {
      "signature": "<contents of .sig file>",
      "url": "https://github.com/kalam-voice/kalam/releases/download/v1.2.0/Kalam-Voice-1.2.0-x64.dmg"
    },
    "linux-x86_64": {
      "signature": "<contents of .sig file>",
      "url": "https://github.com/kalam-voice/kalam/releases/download/v1.2.0/Kalam-Voice-1.2.0-x64.AppImage"
    }
  }
}
```

---

## 15. Platform-Specific Implementation Details

### 15.1 Windows

**Global Hotkey — Windows Key Interception:**

The Windows key (`LWin`/`RWin`) opens the Start Menu by default when released. If the user's hotkey is `Ctrl+Win`, releasing the keys after dictation will open the Start Menu, stealing focus and causing the transcribed text to be typed into the Start Menu search bar.

**Solution:** Use `windows-sys` or `winapi` crate to install a `WH_KEYBOARD_LL` (Low-Level Keyboard Hook) in the OS event queue. When the Windows key is detected as part of the dictation hotkey, inject a "dummy" keystroke (`vkE8`, an unassigned virtual key code) immediately before the Windows key is released. This tells Windows that a key combination was pressed (not just the Windows key alone), which suppresses the Start Menu.

```rust
// Pseudo-code for Windows key suppression
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

fn suppress_start_menu() {
    // When Win key release detected during dictation:
    // Inject vkE8 (dummy key) to cancel Start Menu activation
    let input = INPUT {
        type_: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0xE8,  // Unassigned virtual key
                // ... rest of struct
            }
        }
    };
    SendInput(1, &input, size_of::<INPUT>() as i32);
}
```

**Code Signing:** Authenticode certificate via Azure Key Vault (for v1.0; self-signed for beta)

**Installer:** `.msi` (primary, for proper Windows installation) + `.exe` (NSIS, for portable option)

### 15.2 macOS

**Required Permissions (Entitlements):**
- **Microphone:** `NSMicrophoneUsageDescription` — required for audio capture
- **Accessibility:** `NSAppleEventsUsageDescription` — required for text injection via `enigo`
- **Input Monitoring:** May be required depending on hotkey implementation

**The onboarding wizard MUST guide the user through granting these permissions with clear instructions and screenshots.**

**Code Signing & Notarization:**
- Apple Developer ID certificate ($99/year)
- Every `.dmg` must be submitted to Apple's notary service during CI/CD
- Without notarization, macOS Gatekeeper will block installation

**Sidecar Sandboxing:** The SenseVoice sidecar executable runs sandboxed as a child process

**Installer:** `.dmg` with drag-to-Applications experience

### 15.3 Linux

**Audio Subsystems:** `cpal` auto-detects and supports:
- ALSA (direct hardware access)
- PulseAudio (most common desktop audio server)
- JACK (professional audio)
- PipeWire (modern replacement for PulseAudio, compatible)

**Display Server Support:**

| Display Server | Global Hotkeys | Text Injection | Status |
|---|---|---|---|
| **X11** | Full support | Full support | **Primary target, fully supported** |
| **Wayland** | Compositor-dependent | Limited | **Best-effort, experimental** |

**Wayland Challenges:**
- Global hotkeys are NOT standardized in Wayland. Each compositor has its own protocol:
  - KDE: `kglobalaccel`
  - GNOME: `mutter` private API (very limited)
  - Sway: IPC protocol
  - Hyprland: socket IPC
- Text injection requires the `virtual-keyboard` Wayland protocol (works on wlroots-based compositors, limited on GNOME)

**Wayland Implementation:**

```rust
pub enum DisplayServer {
    X11,
    Wayland { compositor: String },
}

pub fn detect_display_server() -> DisplayServer {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        let compositor = detect_compositor(); // Read $XDG_CURRENT_DESKTOP or process list
        DisplayServer::Wayland { compositor }
    } else {
        DisplayServer::X11
    }
}

// If Wayland with unsupported compositor: fall back to tray-icon-only activation
// (user clicks tray icon to start recording instead of global hotkey)
```

**User-facing Wayland guidance:** Provide a clear compatibility matrix in documentation. For unsupported Wayland compositors, offer tray-icon activation as the fallback.

**Packaging:**
- `.AppImage` — universal, runs on any distro
- `.deb` — for Ubuntu/Debian
- AUR package — for Arch Linux
- Flatpak — for Flathub

**Desktop Integration:** System tray via AppIndicator (for GNOME/Unity/KDE compatibility)

---

## 16. Non-Functional Requirements

### 16.1 Performance Requirements

| Metric | Target | Critical? |
|---|---|---|
| Idle RAM usage | < 30MB | YES |
| Cold start time | < 3 seconds | YES |
| End-to-end latency (cloud) | < 500ms | YES |
| End-to-end latency (local) | < 150ms | YES |
| Audio capture latency | < 50ms | YES |
| Text injection latency (short text) | < 50ms | YES |
| Bundle size (without models) | < 20MB | NO (target) |
| Battery impact (idle) | < 2% per hour | NO (target) |

### 16.2 Compatibility Requirements

| Platform | Minimum Version | Architectures |
|---|---|---|
| Windows | Windows 10 (1903+) | x64, ARM64 |
| macOS | macOS 11 (Big Sur) | Intel (x64), Apple Silicon (ARM64) |
| Linux | Ubuntu 20.04, Fedora 34, Arch (rolling) | x64, ARM64 |

### 16.3 Reliability Requirements

- **Uptime:** 99.9% availability when the OS is running (the app should essentially never crash)
- **Error Recovery:** Automatic restart on crash with session state persistence
- **Data Integrity:** SHA-256 checksums on downloaded models; ECDSA on updates
- **Graceful Degradation:** Cloud unavailable → local fallback. Mic lost → fallback to default. STT error → retry with backoff.

---

## 17. Accessibility Requirements

Kalam MUST be accessible to users with disabilities. This is both a moral imperative (Persona: Accessibility Alex) and a competitive differentiator.

**WCAG 2.1 AA Compliance:**
- All UI elements must be keyboard-navigable
- All UI elements must have proper ARIA labels for screen readers
- Minimum contrast ratio of 4.5:1 for text
- Focus indicators must be visible

**Screen Reader Support:**
- Windows: NVDA, JAWS
- macOS: VoiceOver
- Linux: Orca

**Visual:**
- High contrast mode support
- Configurable font sizes in all UI panels
- Tray icon state changes must be communicated non-visually (screen reader announcements + optional audio cues)

**Motor Impairment:**
- Voice control for all UI elements (dictate into settings fields)
- Single-key hotkey activation option (for users who cannot perform key combinations)
- Configurable hold duration for "hold to record" mode

---

## 18. Telemetry & Analytics

### 18.1 Telemetry Is Opt-In ONLY

Telemetry is NEVER enabled by default. The user must explicitly opt in during onboarding or in Settings > Privacy.

### 18.2 Infrastructure: PostHog Cloud (Open Source Tier)

- **Pricing:** Free for < 1M events/month
- **GDPR compliant** — no cookie banner required for basic analytics
- **Self-hostable** — if the community prefers, PostHog can be self-hosted
- **Open Source** — aligns with project values

### 18.3 What IS Collected (if opted in)

```rust
pub struct TelemetryEvent {
    pub event_type: String,       // e.g., "dictation_completed"
    pub duration_ms: u64,         // Transcription processing time
    pub mode: String,             // "cloud" | "local"
    pub error: Option<String>,    // Error type (NOT content), e.g., "api_timeout"
    pub platform: String,         // "win" | "mac" | "linux"
    pub app_version: String,      // e.g., "1.2.0"
}
```

Collected metrics:
- Usage frequency (sessions/day)
- Transcription duration and latency
- Mode usage distribution (cloud vs local)
- Error rates and types (NOT error content)
- Feature usage (voice commands, snippets, history)
- Platform distribution

### 18.4 What is NEVER Collected

- Audio recordings
- Transcribed text content
- Personal dictionary entries
- API keys
- File paths or file content
- Window titles or process names
- User identity (all data is anonymized)

---

## 19. Competitive Analysis

### 19.1 Feature Comparison

| Feature | Wispr Flow | Apple Dictation | Windows Speech | **Kalam** |
|---|---|---|---|---|
| Price | $15/month | Free (Apple only) | Free (Windows only) | **Free / BYOK** |
| Open Source | No | No | No | **Yes** |
| Cross-Platform | macOS + Windows | macOS/iOS only | Windows only | **Win + Mac + Linux** |
| Offline Mode | No | Limited | Limited | **Yes (full)** |
| Idle RAM | 800MB+ | N/A (system) | N/A (system) | **10-30MB** |
| Provider Choice | No | No | No | **Yes (Groq, OpenAI, local)** |
| Local Model | No | Partial | Partial | **Yes (SenseVoice, Whisper.cpp)** |
| Custom Hotkeys | Limited | No | No | **Yes (any combination)** |
| API / Extensibility | No | No | No | **Yes** |
| Voice Commands | Yes | Basic | Basic | **Yes (extensible regex)** |
| Personal Dictionary | No | No | No | **Yes** |
| Snippets | No | No | No | **Yes** |

### 19.2 Differentiation

1. **Privacy Leadership:** The only fully offline-capable voice dictation tool with open-source transparency
2. **Resource Efficiency:** 25x lower memory footprint than Wispr Flow
3. **Provider Agnostic:** Not locked into any single cloud vendor
4. **Developer Friendly:** Open source, extensible, API access
5. **Zero Cost:** BYOK means the software is truly free

---

## 20. Risk Assessment

### 20.1 Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Windows key interception fails on some configurations | Medium | High | Extensive testing matrix; fallback hotkeys; document known issues |
| Audio latency on Linux (PulseAudio/PipeWire quirks) | Medium | Medium | Support multiple backends; user-selectable in settings |
| Local model performance insufficient on old hardware | Low | High | Provide hardware requirements; offer cloud fallback; tiny model option |
| Wayland global hotkeys broken on some compositors | High | Medium | X11 primary target; tray-icon fallback for Wayland; clear docs |
| Cross-platform text injection edge cases | Medium | High | Dual strategy (keystroke + clipboard); retry mechanism; user override |
| Sidecar process management (crashes, zombie processes) | Medium | Medium | Robust process supervision; kill on app exit; health checks |

### 20.2 Business Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Low community adoption | Medium | High | Marketing (Product Hunt, HN, Reddit); demo videos; partnerships |
| Groq API pricing changes / deprecation | Low | Medium | Provider-agnostic architecture; easy to add new providers |
| Contributor burnout / bus factor | Medium | High | Clear governance; modular architecture; funded maintainership |
| Enterprise adoption barriers (compliance) | Low | Medium | Local mode for HIPAA; security audits for v1.0+ |
| Insufficient GitHub Sponsors revenue | Medium | Medium | BYOK = zero costs; focus on enterprise tier for revenue |

### 20.3 Legal Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Patent infringement (voice dictation patents) | Low | High | Prior art research; open-source licensing review |
| Data privacy violations (GDPR/HIPAA) | Low | High | Privacy-first design; local mode; no default data retention |
| License compatibility (Rust crate licenses) | Low | Medium | License audit of all dependencies before v1.0 |

---

## 21. Development Roadmap

### Phase 1: MVP (Months 1-3)

**Goal:** Working dictation app with cloud STT on Windows and macOS

**Deliverables:**
- [ ] Tauri v2 project scaffolding with React + TypeScript frontend
- [ ] System tray integration (icon, context menu, background running)
- [ ] Global hotkey registration and activation
- [ ] Audio capture pipeline (cpal → rubato resampling → 16kHz mono PCM)
- [ ] Groq API integration (whisper-large-v3-turbo)
- [ ] Basic audio chunking with VAD (Silero)
- [ ] Text injection via enigo (keystroke mode)
- [ ] Clipboard fallback for long text
- [ ] Basic settings panel (hotkey config, API key input, device selection)
- [ ] First-run onboarding wizard
- [ ] OS-native error notifications (tauri-plugin-notification)
- [ ] Windows and macOS builds
- [ ] Basic README and installation docs

**Alpha Release Criteria:**
- User can press hotkey, speak, and see text appear in a text editor
- Cloud mode works with user's Groq API key
- Works on Windows 10+ and macOS 11+

### Phase 2: Enhanced Features (Months 4-6)

**Goal:** Local STT, Linux support, production features

**Deliverables:**
- [ ] SenseVoice local model integration (Sherpa-ONNX sidecar)
- [ ] Whisper.cpp local model integration (via whisper-rs)
- [ ] Model download manager with progress UI
- [ ] Provider-agnostic STT interface with fallback chain
- [ ] Hybrid mode with auto cloud/local switching
- [ ] Voice Activity Detection tuning (Fast/Balanced/Accurate presets)
- [ ] Pseudo-streaming with audio chunking + overlap + prompt chaining
- [ ] Linux support (X11 fully supported)
- [ ] Personal dictionary management
- [ ] Basic voice commands (punctuation, new paragraph)
- [ ] Filler word removal
- [ ] Transcription history with SQLite storage and search
- [ ] OpenAI Whisper API as alternative cloud provider
- [ ] CI/CD pipeline (GitHub Actions, matrix builds)
- [ ] OTA updates via Tauri updater + GitHub Pages
- [ ] Audio device hot-plug detection

**Public Beta Release Criteria:**
- Cloud and local modes both functional
- Works on Windows, macOS, and Linux (X11)
- Auto-updates working
- History, voice commands, and personal dictionary functional

### Phase 3: Polish & Scale (Months 7-9)

**Goal:** Production-ready v1.0

**Deliverables:**
- [ ] Advanced voice commands (editing: "delete that", "scratch that")
- [ ] Snippet system (trigger phrases → expanded text)
- [ ] Multi-language support (20+ languages, auto-detection)
- [ ] Sensitive app detection for hybrid mode
- [ ] Accessibility features (screen reader support, high contrast, keyboard nav)
- [ ] Code signing (Windows Authenticode, macOS notarization)
- [ ] Wayland experimental support (best-effort)
- [ ] Telemetry (PostHog, opt-in only)
- [ ] Performance optimization (profiling, memory audit)
- [ ] Comprehensive documentation (install guide, user manual, API docs)
- [ ] Landing page website (GitHub Pages)
- [ ] Community building (Discord, Contributing Guide, Code of Conduct)

**v1.0 Stable Release Criteria:**
- All P0 and P1 features complete
- Code-signed installers for all platforms
- < 30MB idle RAM on all platforms
- < 500ms end-to-end cloud latency
- < 150ms end-to-end local latency
- Comprehensive docs and user guides
- Auto-updates working and tested

### Phase 4: Expansion (Months 10-12)

**Goal:** Growth, enterprise, and ecosystem

**Deliverables:**
- [ ] 100+ language support
- [ ] Additional cloud providers (AssemblyAI, Deepgram, Google, Azure)
- [ ] Context awareness (read surrounding text for accuracy — P2 feature)
- [ ] Enterprise features (SSO, admin dashboard, policy enforcement)
- [ ] Plugin/extension system architecture
- [ ] Package manager distribution (Homebrew, Chocolatey, winget, AUR, Flathub)
- [ ] Microsoft Store listing
- [ ] Advanced analytics dashboard (for project health, opt-in)
- [ ] Mobile companion app research & feasibility study
- [ ] Security audit (third-party)
- [ ] Bug bounty program

**v1.5 Feature Release**

---

## 22. Success Metrics & KPIs

### 22.1 User Adoption

| Metric | 6 Months | 12 Months |
|---|---|---|
| Total downloads | 50,000 | 200,000 |
| Monthly active users | 15,000 | 60,000 |
| GitHub stars | 5,000 | 15,000 |
| Contributors | 25 | 75 |

### 22.2 Technical Performance

| Metric | Target |
|---|---|
| Average transcription latency (cloud) | < 400ms |
| Average transcription latency (local) | < 100ms |
| Word Error Rate (WER) | < 5% |
| App crash rate | < 0.1% of sessions |
| Update adoption rate | > 80% within 2 weeks |

### 22.3 Business

| Metric | Target |
|---|---|
| GitHub Sponsors revenue | $5,000/month by Month 12 |
| Enterprise inquiries | 50+ by Month 12 |
| Enterprise conversions | 10+ by Month 12 |
| Self-sustaining operations | Month 12 |

### 22.4 Community

| Metric | Target |
|---|---|
| Discord members | 5,000+ |
| Monthly releases | 1-2 |
| Average issue response time | < 48 hours |
| PR review turnaround | < 72 hours |
| NPS score | > 50 |

---

## 23. Appendix

### 23.1 Glossary

| Term | Definition |
|---|---|
| **ASR** | Automatic Speech Recognition |
| **BYOK** | Bring Your Own Key — user provides their own API credentials |
| **ECDSA** | Elliptic Curve Digital Signature Algorithm — used for update signing |
| **IPC** | Inter-Process Communication — Tauri's secure bridge between frontend and backend |
| **ITN** | Inverse Text Normalization — converts spoken numbers/symbols to written form |
| **LPU** | Language Processing Unit — Groq's custom silicon for fast inference |
| **ONNX** | Open Neural Network Exchange — portable ML model format |
| **OTA** | Over-The-Air updates |
| **PCM** | Pulse-Code Modulation — raw audio format |
| **Sidecar** | A standalone executable bundled with and managed by the Tauri app |
| **STT** | Speech-to-Text |
| **VAD** | Voice Activity Detection — distinguishes speech from silence |
| **WER** | Word Error Rate — standard metric for transcription accuracy |
| **WebView** | The OS-native browser engine used by Tauri for rendering the frontend |

### 23.2 Key Library References

| Library | Purpose | URL |
|---|---|---|
| Tauri v2 | Desktop framework | https://v2.tauri.app/ |
| cpal | Cross-platform audio I/O | https://github.com/RustAudio/cpal |
| rubato | Audio resampling | https://github.com/HEnquist/rubato |
| enigo | Cross-platform input simulation | https://github.com/enigo-rs/enigo |
| whisper-rs | Whisper.cpp Rust bindings | https://github.com/tazz4843/whisper-rs |
| Sherpa-ONNX | ONNX inference runtime for SenseVoice | https://github.com/k2-fsa/sherpa-onnx |
| SenseVoice | Local STT model | https://github.com/FunAudioLLM/SenseVoice |
| Silero VAD | Voice Activity Detection | https://github.com/snakers4/silero-vad |
| Groq API | Cloud STT | https://console.groq.com/docs/speech-to-text |
| tauri-plugin-notification | OS notifications | https://v2.tauri.app/plugin/notification/ |
| tauri-plugin-updater | Auto-updates | https://v2.tauri.app/plugin/updater/ |
| tauri-plugin-global-shortcut | Global hotkeys | https://v2.tauri.app/plugin/global-shortcut/ |
| tauri-plugin-sql | SQLite database | https://v2.tauri.app/plugin/sql/ |
| PostHog | Analytics (opt-in) | https://posthog.com/ |

### 23.3 Rust Dependency Summary (Cargo.toml)

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-global-shortcut = "2"
tauri-plugin-notification = "2"
tauri-plugin-updater = "2"
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
tauri-plugin-shell = "2"

# Audio
cpal = "0.15"
rubato = "0.14"

# Text injection
enigo = "0.2"

# STT
reqwest = { version = "0.12", features = ["multipart", "json"] }
whisper-rs = "0.12"

# General
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1"
ring = "0.17"          # Encryption (AES-256)
directories = "5"       # Platform-appropriate paths
log = "0.4"
env_logger = "0.11"

# Platform-specific
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.52", features = ["Win32_UI_Input_KeyboardAndMouse"] }
```

### 23.4 Frontend Dependencies (package.json)

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-global-shortcut": "^2.0.0",
    "@tauri-apps/plugin-notification": "^2.0.0",
    "@tauri-apps/plugin-updater": "^2.0.0",
    "@tauri-apps/plugin-sql": "^2.0.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "react-router-dom": "^6.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "typescript": "^5.5.0",
    "vite": "^5.4.0",
    "@vitejs/plugin-react": "^4.3.0",
    "tailwindcss": "^3.4.0"
  }
}
```

### 23.5 Discovery & Marketing Channels

| Channel | Strategy | Timeline |
|---|---|---|
| Product Hunt | Launch campaign | Month 1 |
| Hacker News | "Show HN" post | Month 1 |
| Reddit | r/productivity, r/rust, r/opensource, r/accessibility | Month 1-2 |
| YouTube | Demo videos, tutorials | Month 2 |
| AlternativeTo.net | Listing as Wispr Flow alternative | Month 1 |
| Awesome Lists | awesome-rust, awesome-productivity | Month 1 |
| Dev.to / Blog | Technical deep-dives | Ongoing |

### 23.6 Document History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-03-03 | Initial PRD proposal |
| 1.1 | 2026-03-04 | Addressed all gaps (text injection, notifications, API keys, hybrid mode, model download, telemetry, Wayland, VAD, UAC, voice commands) |
| **2.0** | **2026-03-04** | **Final consolidated PRD incorporating all research, gap analysis, framework comparison, STT provider research, and distribution strategy** |

---

## Quick Reference for AI Agents

**If you are an AI agent building this project, here is the priority order:**

1. **Scaffold the Tauri v2 project** with React + TypeScript frontend
2. **Implement system tray** with background running (no main window)
3. **Implement global hotkey** with Windows key suppression
4. **Build audio capture pipeline** (cpal → rubato → ring buffer)
5. **Integrate Groq API** for cloud transcription
6. **Build text injection** (enigo keystrokes + clipboard fallback)
7. **Build settings panel** (React, minimal: hotkey, API key, device)
8. **Add VAD** for audio chunking
9. **Add local STT** (SenseVoice sidecar OR Whisper.cpp)
10. **Add voice commands** and filler word removal
11. **Add transcription history** (SQLite)
12. **Set up CI/CD** (GitHub Actions)
13. **Add auto-updates** (tauri-plugin-updater + GitHub Pages)
14. **Add Linux support**
15. **Polish UI, accessibility, docs**

**Critical constraints to always remember:**
- Bundle size < 20MB (without models)
- Idle RAM < 30MB
- NEVER store audio to disk by default
- NEVER enable telemetry by default
- ALWAYS provide local/offline fallback
- ALWAYS use TLS 1.3 for cloud calls
- ALWAYS verify ECDSA signatures for updates
- The app MUST be invisible in the system tray until the hotkey is pressed

---

*This is the final, consolidated PRD for Kalam. It supersedes all previous documents in `.doc/`. All technical decisions have been validated through research. This document should be the single source of truth for development.*
