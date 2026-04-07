//! Windows-only: suppress the native system context menu (minimize / maximize / close) on the
//! transparent overlay HWND. `WM_CONTEXTMENU` is still delivered when the user right-clicks the
//! client area; returning without `DefWindowProc` prevents the default menu.

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::WM_CONTEXTMENU;

/// Stable id so we never double-register the same subclass on this HWND.
const OVERLAY_SUBCLASS_ID: usize = 0x4b414d4f; // 'KAMO'

unsafe extern "system" fn overlay_suppress_system_context_menu(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    _dwrefdata: usize,
) -> LRESULT {
    if msg == WM_CONTEXTMENU {
        // Swallow default handling so comctl32 does not forward to DefWindowProc (which opens the system menu).
        return 0;
    }
    // SAFETY: forwarded to the previous window procedure via comctl32.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// Install a comctl32 window subclass on the overlay HWND. Safe to call once at startup.
pub fn install_overlay_suppress_system_context_menu(hwnd: HWND) {
    // SAFETY: `hwnd` is a live top-level window from Tauri/Wry; subclass id is unique to this app.
    let ok = unsafe {
        SetWindowSubclass(
            hwnd,
            Some(overlay_suppress_system_context_menu),
            OVERLAY_SUBCLASS_ID,
            0,
        )
    };
    if ok == 0 {
        log::warn!("SetWindowSubclass failed for overlay (system context menu may still appear)");
    }
}
