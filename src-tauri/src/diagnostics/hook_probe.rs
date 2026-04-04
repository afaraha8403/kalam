//! One-shot WH_KEYBOARD_LL install probe (Windows). Does not start a message pump.

#[cfg(windows)]
use std::ptr::null_mut;
#[cfg(windows)]
use std::sync::atomic::{AtomicIsize, Ordering};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{GetLastError, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, WH_KEYBOARD_LL,
};

#[cfg(windows)]
static PROBE_HHOOK: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
unsafe extern "system" fn probe_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    let h = PROBE_HHOOK.load(Ordering::SeqCst);
    CallNextHookEx(h as HHOOK, n_code, w_param, l_param)
}

/// Verify `SetWindowsHookExW(WH_KEYBOARD_LL, …)` succeeds, then unhook immediately.
#[cfg(windows)]
pub fn probe_wh_keyboard_ll() -> crate::diagnostics::HookInstallationResult {
    use crate::diagnostics::HookInstallationResult;

    let thread_id = unsafe { GetCurrentThreadId() };

    let hmod = unsafe { GetModuleHandleW(null_mut()) };
    if hmod == 0 {
        let error = unsafe { GetLastError() };
        let msg = format!("GetModuleHandleW failed (error {error})");
        log::warn!("[DIAGNOSTIC] {msg}");
        return HookInstallationResult {
            success: false,
            hook_handle: "0".to_string(),
            error_code: error,
            error_message: msg.clone(),
            thread_id,
            recommendations: hook_failure_recommendations(error),
        };
    }

    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(probe_keyboard_proc), hmod, 0) };

    if hook == 0 {
        let error = unsafe { GetLastError() };
        let msg = format!("SetWindowsHookExW(WH_KEYBOARD_LL) failed (error {error})");
        log::warn!("[DIAGNOSTIC] {msg}");
        return HookInstallationResult {
            success: false,
            hook_handle: "0".to_string(),
            error_code: error,
            error_message: detailed_hook_message(error, msg),
            thread_id,
            recommendations: hook_failure_recommendations(error),
        };
    }

    PROBE_HHOOK.store(hook, Ordering::SeqCst);
    unsafe {
        UnhookWindowsHookEx(hook as HHOOK);
    }
    PROBE_HHOOK.store(0, Ordering::SeqCst);

    let success_msg = format!("WH_KEYBOARD_LL install probe succeeded (handle 0x{hook:X})");
    log::info!("[DIAGNOSTIC] {success_msg}");

    HookInstallationResult {
        success: true,
        hook_handle: format!("0x{hook:X}"),
        error_code: 0,
        error_message: success_msg,
        thread_id,
        recommendations: vec![
            "Hook installation is allowed for this process.".to_string(),
            "If dictation hotkeys still fail, check config, other hooks, or security software."
                .to_string(),
        ],
    }
}

#[cfg(windows)]
fn detailed_hook_message(error: u32, base: String) -> String {
    match error {
        5 => format!("{base} — access denied (try running as Administrator or check antivirus)."),
        126 => format!("{base} — module not found (system DLL issue)."),
        127 => format!("{base} — procedure not found."),
        _ => base,
    }
}

#[cfg(windows)]
fn hook_failure_recommendations(error: u32) -> Vec<String> {
    match error {
        5 => vec![
            "Try running Kalam as Administrator.".to_string(),
            "Check whether antivirus or endpoint software blocks low-level keyboard hooks."
                .to_string(),
        ],
        126 | 127 => vec![
            "Run DISM /Online /Cleanup-Image /RestoreHealth and reboot.".to_string(),
            "Reinstall the Visual C++ Redistributable (x64).".to_string(),
        ],
        _ => vec![
            "Check Windows Event Viewer for related errors.".to_string(),
            "Reboot and try again.".to_string(),
        ],
    }
}

#[cfg(not(windows))]
pub fn probe_wh_keyboard_ll() -> crate::diagnostics::HookInstallationResult {
    use crate::diagnostics::HookInstallationResult;
    HookInstallationResult {
        success: true,
        hook_handle: "N/A".to_string(),
        error_code: 0,
        error_message: "WH_KEYBOARD_LL is Windows-only; this build uses rdev or platform APIs."
            .to_string(),
        thread_id: 0,
        recommendations: vec![
            "No low-level Windows hook is used on this OS.".to_string(),
            "If hotkeys fail here, check desktop environment permissions and shortcuts."
                .to_string(),
        ],
    }
}
