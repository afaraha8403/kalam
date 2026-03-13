---
name: Comprehensive Latency Debugging Plan
overview: Implement a unified, high-precision tracing system across Rust and Svelte to pinpoint the exact source of the 1-second delay, supplemented by external profiling if necessary.
todos: []
isProject: false
---

# Comprehensive Latency Debugging Plan

To find the exact source of the ~1 second delay, we need to trace the entire pipeline from the physical keypress to the pixels rendering on the screen. We will implement a unified high-precision logging system that correlates Rust (backend) and JavaScript (frontend) events in a single chronological timeline.

## Phase 1: Unified Tracing System

- **Enhance Rust Logger**: Upgrade the existing `latency_debug_write` to include microsecond precision and ensure it flushes immediately to a dedicated file (e.g., `.kalam/latency-trace.log`).
- **Bridge JS to Rust**: Create a new Tauri command (`#[tauri::command] fn trace_latency(event: String, js_timestamp: f64)`) so the Svelte frontend can send its exact timing data back to Rust. Rust will calculate the delta between the JS timestamp and the Rust timestamp to account for IPC transit time.

## Phase 2: Backend (Rust) Instrumentation

We will inject trace points at every critical junction in `src-tauri/src/lib.rs` and related modules:

- **T0**: Inside the raw `rdev` OS hook callback (the absolute earliest we detect the key).
- **T1**: Inside the Tauri async task spawned by the hotkey.
- **T2**: Right before and after the overlay window is pre-resized.
- **T3**: Right before and after `play_sound` is invoked.
- **T4**: Right before `app.emit_to` sends the `Listening` event to the frontend.
- **T5**: Right after `nudge_overlay_renderer` finishes.

## Phase 3: Frontend (Svelte/JS) Instrumentation

We will inject trace points in the Overlay window (`src/components/Overlay.svelte` and `src/app.html`):

- **T6**: At the very top of the JS execution context (to see if the WebView was suspended/reloading).
- **T7**: The exact millisecond the `listen('overlay-state')` callback fires.
- **T8**: Inside Svelte's `$effect` or `afterUpdate` to track when Svelte applies the state change.
- **T9**: Using `requestAnimationFrame` to log when the browser actually paints the new UI to the screen.

## Phase 4: External Profiling (If logs don't reveal the culprit)

If the logs show that the delay happens *between* T4 (Rust emits) and T7 (JS receives), or *before* T0 (OS to Rust), we will use external tools:

1. **Chromium DevTools**: Temporarily make the overlay window focusable/inspectable so we can use the DevTools "Performance" tab to record a CPU profile of the WebView2 renderer during the hotkey press.
2. **Spy++ / WinUI**: Monitor Windows message queues to see if the OS is delaying the delivery of the keyboard hook to our application.

## Execution

Once you approve this plan, I will write the code to implement Phases 1-3. We will then run the app, trigger the hotkey, and analyze the resulting `latency-trace.log` file to find the exact bottleneck.