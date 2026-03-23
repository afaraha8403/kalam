# Windows profiling for input and overlay latency

When in-app traces (e.g. `KALAM_LATENCY_DEBUG=1` and `latency-trace.log`) don't explain a perceived delay, use OS-level tools to see where time is spent.

## Finding the process (especially in dev mode)

- **Process name**: The running app is `**kalam-voice.exe`** (from the Cargo package name). In dev mode it lives under `src-tauri\target\debug\kalam-voice.exe` and is launched by `npm run tauri dev`.
- **Window titles**: The main window title is **"Kalam"**; the overlay (small pill) window title is **"Kalam Overlay"**. The main window may be hidden (only tray visible) until you open it.
- **In Spy++**: Use **Search → Find Window** (Ctrl+F), then drag the finder (crosshair) over the Kalam main window or the overlay pill. Spy++ will highlight the window in its tree. Alternatively, in the **Windows** tree expand until you find a window whose caption is "Kalam" or "Kalam Overlay".
- **Task Manager**: Look for **kalam-voice.exe**; you may also see **node.exe** (Vite dev server) and the browser if you have the frontend devtools open.

## Spy++ (Visual Studio)

1. Install: part of Visual Studio (Tools → Spy++) or [Build Tools](https://visualstudio.microsoft.com/downloads/) with "C++ desktop development".
2. Run **Spy++** (64-bit: `spyxx_amd64.exe`).
3. **Find the window**: Search → Find Window → drag the finder over the Kalam window or overlay pill. In the tree, **expand** that window (click the [+] next to it) so you see its **child windows**.
4. **Message log**: Right‑click the window → **Messages** → **Log Messages** (or Messages menu → Log Messages).
   - **Message scope**: Choose **"Window and descendants"** (or "Window and its children") so messages to the top-level window and all child windows (including the WebView2 host) are logged. Logging only the top-level window often shows nothing because Tauri/WebView2 deliver many messages to child HWNDs.
   - **Which window**: You can also try selecting a **child** of "Kalam" or "Kalam Overlay" in the tree (e.g. the first child with a class like "Chrome_WidgetWin_1" or similar) and log that HWND and its descendants.
   - **Messages to log**: In the dialog, open the "Messages to log" list and either select **"All Messages"** or add at least: **WM_PAINT**, **WM_SIZE**, **WM_TIMER**, **WM_ERASEBKGND**. Then click **OK** and press **Start Logging** (or "Log").
   - Trigger the overlay (hold hotkey so the pill appears/expands) and watch the log. You should see WM_PAINT and similar when the overlay updates.
5. **Why you won't see WM_KEYDOWN for the hotkey**: Kalam uses a **low-level keyboard hook** (`WH_KEYBOARD_LL` on Windows; rdev on Linux/macOS), not window messages, for the dictation hotkey. The key is delivered to our callback; it is **never** posted to the app's windows. So an empty log when you *only* press the hotkey is expected. To see messages, trigger something that affects the window (e.g. show the main window, move the overlay, or hold the hotkey so the overlay expands and repaints).
6. **If the log is still empty**: Try **Messages → Log Messages** and in the "Window" / scope dropdown choose **"All windows in the same thread"** or **"All windows in the process"**, then Start Logging. That logs every message to every window in the process so you can see which HWND actually receives traffic.

## WinUI / Windows SDK tooling

- **Windows Performance Recorder (WPR)** and **Windows Performance Analyzer (WPA)** can record input and UI threads. Capture a trace while triggering dictation and inspect input → process → thread timing.
- For **WebView2**, Chromium’s own throttling of background windows is not visible in Spy++; it only shows Win32 message delivery. The in-app trace (T4 → T7) measures when the script runs in the overlay after the emit.

## In-app overlay message log (no Spy++ needed)

The app can log Win32 messages for the overlay window itself (runs inside the process):

1. Start Kalam (e.g. `npm run tauri dev`).
2. Run the helper script: `.\scripts\run-overlay-message-log.ps1` and follow the prompts.
3. Or manually: open the main window → DevTools (F12 or right‑click → Inspect) → Console → run:
   `await __TAURI_INTERNALS__.invoke('start_overlay_message_log_for_seconds', { seconds: 15 })`
4. Hold the dictation hotkey for ~15 seconds. Log is written to `~/.kalam/overlay-messages.log` (tab‑separated: timestamp, message name, hex msg, hwnd).

Commands: `start_overlay_message_log`, `stop_overlay_message_log`, `start_overlay_message_log_for_seconds` (auto‑stops after N seconds).

## Interpreting results

- **Spy++**: If `WM_KEYDOWN` (or equivalent) is logged long after you press the key, the delay is in the OS or input stack (drivers, accessibility hooks, etc.).
- **Spy++**: If the message arrives quickly but the overlay updates late, the delay is in Tauri/WebView (our T4→T7 gap in the trace).
- **Trace**: If T0 (our key callback) is late relative to the physical press, the delay is between the OS and our rdev listener (e.g. another hook consuming the key first, or rdev thread starvation).

## Trace result: delay is before T0 (rdev)

With `KALAM_LATENCY_DEBUG=1`, the app logs **OS_key_down_0x{VK}** (from a `WH_KEYBOARD_LL` hook in our process) and **T0** (when the rdev listener callback runs). Comparing timestamps in `~/.kalam/latency-trace.log`:

- **OS_key_down (Win, 0x5B)** and **T0** on the same key press showed **~505 ms** gap (e.g. OS_key_down at 1773326006306784 µs, T0 at 1773326006812096 µs).
- So the OS delivers the key to our process quickly (low-level hook fires), but **rdev’s callback (T0) runs ~500 ms later**. The delay is **before T0**: between the Windows input path and rdev’s listener, not in our overlay/resize/sound pipeline (T0→T9 is ~40 ms).

**Conclusion:** The perceived ~1 s delay is largely **before** our code: the OS sees the key quickly, but rdev’s global listener is slow to run. Possible causes include how rdev receives key events on Windows (thread scheduling, hook order, or internal processing). Testing an alternative input library (e.g. willhook, which also uses Windows low-level hooks) with `KALAM_USE_WILLHOOK=1` lets us see if T0 moves closer to OS_key_down.

### rdev vs willhook: same ~500 ms gap (thread scheduling)

Traces with **rdev** and **willhook** (separate runs, same machine) both showed **~505–510 ms** between **OS_key_down** (Win) and **T0**. So the delay is **not** specific to rdev; it appears to be **thread scheduling**: the OS delivers the key quickly to our process (the latency-debug `WH_KEYBOARD_LL` hook sees it immediately), but the thread that runs hotkey matching (rdev callback or willhook `recv` loop) runs ~500 ms later.

### How to test willhook (alternative listener)

1. Set both env vars and start the app:  
   `$env:KALAM_LATENCY_DEBUG="1"; $env:KALAM_USE_WILLHOOK="1"; npm run dev`
2. Use the dictation hotkey and check `~/.kalam/latency-trace.log`: compare **OS_key_down_0x5B** (or your main key) to **T0**. (Result: gap is similar to rdev, ~500 ms.)
3. To use rdev again, omit `KALAM_USE_WILLHOOK` or set `KALAM_USE_WILLHOOK=0`.

### Windows default: same-thread hook (low latency)

On Windows, the **default** hotkey path is the `WH_KEYBOARD_LL` hook (hotkey runs in the same thread as key delivery), so OS_key_down and T0 are typically within a few hundred microseconds. To use rdev instead (e.g. for comparison), set `KALAM_USE_RDEV=1` before starting. With rdev on Windows, the listener thread is set to `THREAD_PRIORITY_ABOVE_NORMAL`; see below for measured impact.

---

## Findings and accepted behaviour (verbose)

This section summarises the latency investigation and the current, accepted behaviour. It is intended for future follow-up.

### What we measured

1. **OS_key_down vs T0**
   - With `KALAM_LATENCY_DEBUG=1`, the app writes **OS_key_down_0x{VK}** from a `WH_KEYBOARD_LL` hook (same process, dedicated thread) and **T0** when the hotkey callback runs (wherever that is).
   - All timestamps are in **microseconds** in `~/.kalam/latency-trace.log` (tab‑separated: `timestamp\tlabel` or `timestamp\tlabel\tjs_timestamp`).

2. **Rdev listener (Windows)**
   - With **rdev** as the hotkey listener (e.g. `KALAM_USE_RDEV=1`), the gap between **OS_key_down (Win, 0x5B)** and **T0** was consistently **~500–510 ms** (e.g. OS_key_down 1773326006306784 µs, T0 1773326006812096 µs → ~505 ms). So the OS delivers the key to our process quickly (the debug hook sees it), but the **rdev callback runs hundreds of milliseconds later**.

3. **Willhook listener (Windows)**
   - With **willhook** (`KALAM_USE_WILLHOOK=1`) instead of rdev, the **same ~505–510 ms** gap was observed. So the delay is **not** specific to rdev; it appears to be **thread scheduling**: the thread that runs hotkey matching (rdev or willhook) is scheduled long after the key was seen by another thread in the same process.

4. **Same-thread hook (Windows)**
   - When hotkey logic runs in the **same** thread as the key event (the `WH_KEYBOARD_LL` hook that logs OS_key_down), **OS_key_down** and **T0** were **~390 µs** apart (e.g. OS_key_down 1773329184284866 µs, T0 1773329184285256 µs). So the delay is **not** in our overlay/resize/sound pipeline (T0→T9 is ~40 ms); it is **before T0**, in which thread runs the hotkey callback.

5. **Rdev + raised thread priority (Windows)**
   - With rdev and **THREAD_PRIORITY_ABOVE_NORMAL** on the rdev listener thread (`KALAM_USE_RDEV=1`), the **OS_key_down → T0** gap remained **~503 ms** (e.g. OS_key_down 1773337432598828 µs, T0 1773337433101660 µs). Raising the listener thread priority did **not** materially reduce the delay.

### Conclusion

- The **~500 ms** delay is due to **thread scheduling**: the OS delivers the key quickly (our debug hook proves it), but the **separate** thread that runs rdev (or willhook) is scheduled much later. Running hotkey logic in the **same** thread as the key event (the hook) removes that delay (~0.4 ms).
- **Windows default** is therefore the **`WH_KEYBOARD_LL` hook** as the hotkey path (same thread as key delivery). **Linux and macOS** continue to use rdev.
- When **rdev is used on Windows** (opt‑out via `KALAM_USE_RDEV=1`), we **accept the ~500 ms gap for now**. The rdev thread is still set to `THREAD_PRIORITY_ABOVE_NORMAL` for consistency; it did not improve the gap in our tests. Further work (e.g. different input API, or moving all hotkey work into a hook and deferring only heavy work) could be revisited later.

### Trace labels (reference)

- **OS_key_down_0x{VK}** – Key down seen in the latency-debug `WH_KEYBOARD_LL` hook (only when `KALAM_LATENCY_DEBUG=1`).
- **T0** – Hotkey press callback entered (before spawn).
- **T1** – Async task started after spawn.
- **T2_before_resize / T2_after_resize** – Overlay resize.
- **T3_before_play_sound / T3_after_play_sound** – Start sound.
- **T4_before_emit** – Before emitting overlay state.
- **T5_after_nudge** – After overlay nudge.
- **T6–T9** – Frontend (script load, overlay-state received, tick, requestAnimationFrame).
- **listening_emit_run_N** – Nth time we emitted Listening (1 = first after process start).

### Env vars (reference)

- **KALAM_LATENCY_DEBUG=1** – Write OS_key_down and T0–T5 (and related) to `~/.kalam/latency-trace.log`. Use to compare OS_key_down → T0.
- **KALAM_USE_RDEV=1** – On Windows, use rdev instead of the hook for hotkeys (accept ~500 ms gap).
- **KALAM_USE_WILLHOOK=1** – On Windows, use willhook instead of rdev (optional; same ~500 ms gap observed).
- **KALAM_SAME_THREAD_HOTKEY=1** – On Windows, force the hook path (same as default; kept for backward compatibility).

