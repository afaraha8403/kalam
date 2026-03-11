# Changelog

## [Unreleased]

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
