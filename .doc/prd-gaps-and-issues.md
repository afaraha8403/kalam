# PRD Analysis: Gaps, Issues, and Open Questions

**Date:** March 3, 2026  
**Related Document:** `prd-proposal.md` (Version 1.0)

After reviewing the Product Requirements Document (PRD) for Kalam, the following gaps, potential issues, and open questions have been identified. Addressing these will ensure a more robust technical specification and smoother development phases.

---

## 1. Identified Gaps & Missing Requirements

### 1.1 Context Awareness
- **Gap:** Competitors like Wispr Flow read the surrounding text in the active application to provide context to the transcription engine, drastically improving accuracy and formatting. The PRD does not mention reading screen context before dictation.
- **Impact:** Without context, transcription of domain-specific terms or continuing a sentence seamlessly may suffer.

### 1.2 Text Injection Mechanics
- **Gap:** The PRD specifies OS-level simulated keystrokes (via `enigo`/`mouce`) to bypass the clipboard. 
- **Issue:** Simulating keystrokes for long transcriptions can be visibly slow and is highly prone to interruption if the user clicks away or a notification steals focus during injection.
- **Recommendation:** Define a fallback or alternative injection method (e.g., saving current clipboard, copying text, pasting via OS shortcut, then restoring the old clipboard).

### 1.3 Audio Device Management (Hot-plugging)
- **Gap:** While "Audio device selection" is in the settings, the PRD doesn't define the system's behavior when a microphone is plugged in or unplugged during operation or mid-dictation.
- **Recommendation:** Define auto-fallback to default devices and user notifications for device loss.

### 1.4 User Feedback / Error Handling
- **Gap:** The application is designed to be "invisible" (runs in the system tray). If an error occurs (e.g., Groq API timeout, local model crash, network failure), it is unclear how the user is notified.
- **Recommendation:** Specify OS-level toast notifications or a floating non-intrusive status overlay for errors and state changes.

### 1.5 Local Model Acquisition
- **Gap:** The PRD states the bundle size is "< 20MB (without local models)". 
- **Issue:** It does not specify the user experience for acquiring the local models (~200MB). 
- **Recommendation:** Define an on-demand download manager with progress indicators within the Settings panel.

---

## 2. Technical & Architectural Issues

### 2.1 API Key Management & Funding
- **Issue:** Section 10.3 targets "Cloud API costs per user/month <$1", implying Kalam hosts the Groq API key. However, if adoption targets (50k+ active users) are met, this scales to $50k/month, while donation targets are only $5k/month.
- **Recommendation:** Explicitly define a "Bring Your Own Key" (BYOK) architecture for power users, or clarify the funding model for the default cloud service.

### 2.2 Auto-Punctuation in Local Mode
- **Issue:** While cloud models (Whisper) are excellent at auto-punctuation, local acoustic models (like SenseVoice) may require a secondary text-processing step (like an LLM or dedicated punctuation model) to achieve comparable readability. 
- **Recommendation:** Investigate and document whether SenseVoice handles punctuation natively to the required standard, or if a lightweight punctuation model must also be bundled.

### 2.3 "Hybrid Mode" Sensitivity Heuristics
- **Issue:** Section 8.2 mentions Hybrid Mode will "Auto-switch based on connectivity and sensitivity."
- **Recommendation:** Define exactly how the app determines "sensitivity" (e.g., does it detect the active window's process name, like a password manager or banking app, to force local mode?).

### 2.4 Auto-Update Constraints
- **Issue:** Over-the-air (OTA) updates for system-level applications (especially on Windows) often trigger User Account Control (UAC) prompts. This breaks the "silent background" design principle.
- **Recommendation:** Specify how the Tauri updater will handle UAC elevations gracefully without disrupting the user workflow.

---

## 3. Open Questions for Stakeholders

1. **Monetization & Apple Developer Account:** Who will fund the $99/year Apple Developer account required for macOS code signing and notarization, as well as the Azure Key Vault for Windows Authenticode?
2. **VAD Thresholds:** What are the designated Voice Activity Detection (VAD) silence thresholds to trigger the end of a dictation chunk? (e.g., 1.5 seconds of silence = submit).
3. **Telemetry Infrastructure:** If opt-in telemetry is implemented (Section 8.1), what backend infrastructure will be used to collect and anonymize this data safely for an open-source project?
4. **Linux Wayland Support:** Text injection and global hotkeys are notoriously difficult on Linux Wayland compared to X11. Will Wayland be officially supported at launch, or will the initial Linux release target X11 only?
5. **Formatting Processing:** Will formatting commands (e.g., "new paragraph") be handled client-side via Regex/string manipulation, or processed by the LLM/STT engine?