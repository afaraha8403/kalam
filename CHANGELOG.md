# Changelog

## [Unreleased]
- **Feature:** Task detail panel: wider side panel (520px), markdown toolbar (bold, italic, list, link) with Edit/Preview tabs, priority field (None/Low/Medium/High) with list badges, subtask reorder (move up/down), lighter placeholders, due/reminder layout tweaks, Save and Cancel side-by-side in footer.
- **Feature:** Browser dev: when the app is opened in a normal browser (e.g. http://localhost:5173/), onboarding is skipped and a default config is used so the main UI can be debugged without Tauri. In Tauri, `?skipOnboarding=1` forces skipping onboarding for testing.
- **Feature:** Windows hotkey low-latency: default hotkey path is the `WH_KEYBOARD_LL` hook (same thread as key delivery); set `KALAM_USE_RDEV=1` to use rdev (~500 ms gap accepted). Findings and env vars documented in `.doc/latency-profiling-windows.md`.
- **Feature:** Latency test: optional willhook-based hotkey listener. When `KALAM_USE_WILLHOOK=1` (Windows), the app uses the willhook crate instead of rdev for global keyboard listening to compare OS_key_down→T0 latency (rdev showed ~500 ms gap). Rdev remains the default; see `.doc/latency-profiling-windows.md` and inline comments in `hotkey.rs`.
- **Feature:** Latency debug "before T0" test: when `KALAM_LATENCY_DEBUG=1`, a Windows `WH_KEYBOARD_LL` hook logs each key-down as `OS_key_down_0x{VK}` to `~/.kalam/latency-trace.log`. Compare that timestamp to T0 (rdev callback) to confirm whether delay is before our process (key → OS) or inside rdev.
- **Fix:** Hold/command hotkey "didn't hold long enough" false positive: press time is now recorded in the sync key-down callback (rdev thread) and copied into app state at the start of the async task, so hold duration is measured from actual key press, not from when the async task ran. Previously a delayed async task could make a long hold appear short and trigger cancel.
- **Feature:** Settings → About: when an update is available, show a "Download and install" button; download runs in-app with progress, then the app restarts to complete install.
- **Feature:** Latency debugging: when `KALAM_LATENCY_DEBUG=1` (or `true`), a unified trace is written to `~/.kalam/latency-trace.log` with microsecond timestamps. Rust trace points T0–T5 (hotkey callback, spawn, resize, emit, play_sound, nudge) and frontend T6–T9 (script load, overlay-state received, Svelte tick, requestAnimationFrame) plus a `trace_latency` Tauri command so the overlay can report JS-side timestamps for correlation.
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
