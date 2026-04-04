//! Restore OS focus to the window that was foreground when dictation started.
//! `restore_foreground(id)` interprets `id` per platform:
//! - **Windows:** HWND (from `GetForegroundWindow` at dictation start).
//! - **macOS:** Unix PID (from `active_win_pos_rs` at dictation start).
//! - **Linux:** X11 window id as decimal string parsed to `usize` (from `active_win_pos_rs`).
//!
//! Injection (`enigo`) always targets the focused window; we must re-activate the correct
//! window after the user switches away during toggle-mode recording.

/// Try to bring the saved target to the foreground before pasting/typing transcribed text.
/// Returns `true` if the platform-specific call reported success (best-effort on all OSes).
pub fn restore_foreground(id: usize) -> bool {
    #[cfg(windows)]
    {
        unsafe { restore_foreground_windows(id) }
    }
    #[cfg(all(not(windows), target_os = "macos"))]
    {
        restore_foreground_macos(id)
    }
    #[cfg(all(
        not(windows),
        target_os = "linux",
        not(target_os = "android")
    ))]
    {
        restore_foreground_linux(id)
    }
    #[cfg(not(any(
        windows,
        target_os = "macos",
        all(target_os = "linux", not(target_os = "android"))
    )))]
    {
        let _ = id;
        false
    }
}

#[cfg(windows)]
unsafe fn restore_foreground_windows(id: usize) -> bool {
    use std::mem::size_of;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_MENU,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsWindow,
        SetForegroundWindow,
    };

    let target = id as HWND;
    if target == 0 {
        log::warn!("restore_foreground: null HWND");
        return false;
    }
    if IsWindow(target) == 0 {
        log::warn!("restore_foreground: HWND is not valid (window may have closed)");
        return false;
    }
    if GetForegroundWindow() == target {
        log::debug!("restore_foreground: target window already foreground");
        return true;
    }

    let fg = GetForegroundWindow();
    let fg_thread = if fg != 0 {
        GetWindowThreadProcessId(fg, null_mut())
    } else {
        0
    };
    let cur_thread = GetCurrentThreadId();

    let mut attached = false;
    if fg_thread != 0 && fg_thread != cur_thread {
        // Share input queue with the foreground thread so we may legally change focus.
        attached = AttachThreadInput(cur_thread, fg_thread, 1) != 0;
        if !attached {
            log::debug!("restore_foreground: AttachThreadInput failed; trying ALT + SetForegroundWindow anyway");
        }
    }

    // Synthetic Alt satisfies foreground-lock rules so SetForegroundWindow is more likely to succeed.
    let mut alt_inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    SendInput(
        alt_inputs.len() as u32,
        alt_inputs.as_mut_ptr(),
        size_of::<INPUT>() as i32,
    );

    let ok = SetForegroundWindow(target) != 0;
    let _top = BringWindowToTop(target);

    if attached {
        let _ = AttachThreadInput(cur_thread, fg_thread, 0);
    }

    if ok {
        log::info!("restore_foreground: focus restored to dictation target window");
    } else {
        log::warn!(
            "restore_foreground: SetForegroundWindow failed — text may inject into the wrong app"
        );
    }
    ok
}

#[cfg(target_os = "macos")]
fn restore_foreground_macos(pid: usize) -> bool {
    // PIDs on macOS fit in i32; guard so we never embed garbage into AppleScript.
    let pid_i: i32 = match i32::try_from(pid) {
        Ok(p) if p > 0 => p,
        _ => {
            log::warn!("restore_foreground: invalid macOS pid {}", pid);
            return false;
        }
    };
    let script = format!(
        "tell application \"System Events\" to set frontmost of first process whose unix id is {pid_i} to true"
    );
    match std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
    {
        Ok(out) if out.status.success() => {
            log::info!("restore_foreground: activated process pid {} via osascript", pid_i);
            true
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            log::warn!(
                "restore_foreground: osascript failed (pid {}): {}",
                pid_i,
                stderr.trim()
            );
            false
        }
        Err(e) => {
            log::warn!("restore_foreground: could not run osascript: {}", e);
            false
        }
    }
}

#[cfg(all(target_os = "linux", not(target_os = "android")))]
fn restore_foreground_linux(x11_window_id: usize) -> bool {
    let id_str = x11_window_id.to_string();
    match std::process::Command::new("xdotool")
        .args(["windowactivate", "--sync", &id_str])
        .status()
    {
        Ok(status) if status.success() => {
            log::info!(
                "restore_foreground: xdotool windowactivate succeeded for window {}",
                id_str
            );
            true
        }
        Ok(status) => {
            log::warn!(
                "restore_foreground: xdotool windowactivate failed (code {:?}) for window {} — install xdotool on X11, or expect wrong-target paste on Wayland",
                status.code(),
                id_str
            );
            false
        }
        Err(e) => {
            log::warn!(
                "restore_foreground: could not run xdotool (window {}): {}",
                id_str,
                e
            );
            false
        }
    }
}
