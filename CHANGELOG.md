# Changelog

## [Unreleased]
- **Fix:** Overlay pill delay (~1 s): pre-resize/collapse the overlay window from Rust before emitting state events, eliminating the JS→Rust IPC roundtrip that required the unfocused WebView2 to process a resize command. Also add a Web Worker keepalive to prevent WebView2 from throttling the overlay's JS event loop when unfocused, and enable `--disable-background-timer-throttling` browser arg for both windows.
- **Fix:** Start-of-dictation latency: hotkey press/release/cancel now run on Tauri's async runtime instead of a dedicated hotkey runtime, avoiding cross-runtime lock contention that could add ~1 s delay.
- **Fix:** Foreground process name is resolved in a background task after storing the HWND (aligned with v0.0.1-beta.5 behavior) so the start path isn't blocked by OpenProcess/QueryFullProcessImageName.
- **Fix:** Start-of-dictation latency: show overlay pill and start sound immediately, then position overlay and capture foreground window, so heavy work no longer blocks visible/audible feedback.
- **Fix:** Start sound no longer blocks the hot path: WAV read and playback run in spawn_blocking + thread so the dictation flow isn't delayed by disk or audio init.
- **Change:** UI font: body and headings use Google Sans for consistency; logo text keeps Syne (unchanged).
- **Fix:** Dictation cloud API keys are now stored per STT provider, so switching between Groq/OpenAI uses the correct key and Settings shows provider-specific configured state.
- **Fix:** Hold/command hotkey handling now guards startup with a dedicated `Starting` audio state to prevent short-press cancel races that could leave dictation in inconsistent states.
- **Fix:** Dictation stop latency reduced by replacing fixed 300ms recorder shutdown sleeps with a bounded adaptive drain.
- **Change:** Hybrid sensitive-app detection now resolves the active process name via direct PID lookup on Windows instead of full `sysinfo` refresh scans.
- **Change:** Settings page: tab menu (General, Dictation, etc.) is sticky so it stays at the top when scrolling.
- **Fix:** In-app log export: use native Save dialog so log/CSV files are actually saved (blob download does not work in Tauri webview); refresh log-empty state on Settings load; show clear feedback when there are no entries or save fails.
- **Fix:** Prevent multiple instances; re-opening the app focuses the existing window (Issue #9).
- **Fix:** Overlay pill in dormant state: collapsed window size increased to 52×14 so pill border is not clipped; hover expansion and "Press … to dictate" text delayed 60ms so overlay window resizes before content shows; removed border-radius from overlay root so the pill is not clipped by the root's capsule shape on hover (fixes cut-off edges, #7).

## [0.0.1-beta.8]
- **Fix:** Release workflow: global `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24`, correct tauri-action input `assetNamePattern`, Windows build uses `--bundles nsis` only to avoid MSI prerelease version error.
- **Change:** CI workflow: global `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` for Node 24 opt-in on all jobs.
- **Change:** tasks.ps1: on Windows, `build` and `build-signed` use `--bundles nsis` so local builds succeed with prerelease versions (aligns with release workflow).

## [0.0.1-beta.5]
- **Change:** CI: Node 22, npm ci, frontend unit tests; build script embeds Windows .exe icon; AppConfig version handling improved; STT handles empty results and prompt echoes.
- **Change:** Overlay and app optimization: Overlay component, Home, Snippets, History, Notes, Reminders, Tasks, lib.rs and docs.
- **Feature:** Sidebar collapse: toggle for expanded vs icon-only view; state persisted in AppConfig.
- **Feature:** Settings: model download progress and sidecar installation status.
- **Fix:** Overlay background on macOS to prevent white flashes during initialization.
- **Fix:** Injection: force clipboard for TSF-heavy apps (e.g. Win11 Notepad).
- **Change:** Version bump to 0.0.1-beta.5.
- **Change:** Docs: footer and navigation links; business page.
- **Change:** CI: macOS runners (macos-13 for x86_64), frontend build before clippy, Ubuntu deps (libxdo-dev, pkg-config).
- **Feature:** Overlay pill resizes the OS window: idle/collapsed uses 80×80 so the rest of the screen is click-through; expanded (recording or hover) uses 300×120. Preserves hover tooltip and drag.
- **Change:** License: allow commercial use for organizations with up to 2 team members without a separate license; above 5 or resale/embedding still requires a written commercial license. Business page updated to state small-team allowance.
- **Change:** GitHub page (docs): condense top nav into "More" dropdown (Workspace, Compare, Documentation, For business); keep Features, How it Works, FAQ, GitHub in bar.
- **Fix:** macOS crash on keyboard input (onboarding email field): switch rdev to fufesou/rdev fork to avoid HIToolbox main-thread assertion (dispatch_assert_queue_fail).
- **Fix:** Overlay window on macOS showing white box: enable `macOSPrivateApi` in Tauri config/Cargo and set window background color to transparent in Rust to ensure the OS allows transparent windows without white flashes.
- **Change:** Changelog added; Cursor rule to maintain CHANGELOG.md (version-grouped, used for releases).
