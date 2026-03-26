//! Windows-only: log Win32 messages for the overlay window (for latency debugging).
//! Start via Tauri command when KALAM_LATENCY_DEBUG=1; hold the hotkey; stop after a few seconds.

use raw_window_handle::HasWindowHandle;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use tauri::Manager;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetWindowThreadProcessId, IsChild, SetWindowsHookExW, UnhookWindowsHookEx,
    HC_ACTION, HHOOK,
};

const WH_CALLWNDPROC_ID: i32 = 4;

static mut CURRENT_HOOK: HHOOK = 0 as HHOOK;

// Mirrors Win32 `CWPSTRUCT`; name matches the platform typedef.
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
struct CWPSTRUCT {
    lparam: LPARAM,
    wparam: WPARAM,
    message: u32,
    hwnd: HWND,
}

static TARGET_HWND: AtomicUsize = AtomicUsize::new(0);
static HOOK_HANDLE: Mutex<Option<HHOOK>> = Mutex::new(None);
static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

fn message_name(msg: u32) -> &'static str {
    match msg {
        0x0000 => "WM_NULL",
        0x0001 => "WM_CREATE",
        0x0002 => "WM_DESTROY",
        0x0005 => "WM_SIZE",
        0x0006 => "WM_ACTIVATE",
        0x0007 => "WM_SETFOCUS",
        0x0008 => "WM_KILLFOCUS",
        0x000F => "WM_PAINT",
        0x0010 => "WM_CLOSE",
        0x0012 => "WM_QUIT",
        0x0018 => "WM_SHOWWINDOW",
        0x0020 => "WM_SETCURSOR",
        0x0046 => "WM_WINDOWPOSCHANGING",
        0x0047 => "WM_WINDOWPOSCHANGED",
        0x007B => "WM_ENTERSIZEMOVE",
        0x007C => "WM_EXITSIZEMOVE",
        0x0085 => "WM_NCPAINT",
        0x00A0 => "WM_NCMOUSEMOVE",
        0x0113 => "WM_TIMER",
        0x0200 => "WM_MOUSEMOVE",
        0x0214 => "WM_CAPTURECHANGED",
        0x0310 => "WM_THEMECHANGED",
        0x8000 => "WM_APP",
        0xC18D => "WM_CUSTOM_0xC18D", // Chromium/WebView2 internal (e.g. timer/tick)
        _ => "WM_???",
    }
}

unsafe extern "system" fn callwnd_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let hook = CURRENT_HOOK;
    if n_code != HC_ACTION as i32 || l_param == 0 {
        return CallNextHookEx(hook, n_code, w_param, l_param);
    }
    let target = TARGET_HWND.load(Ordering::SeqCst) as HWND;
    if target == 0 {
        return CallNextHookEx(hook, n_code, w_param, l_param);
    }
    let cwp = &*(l_param as *const CWPSTRUCT);
    let hwnd = cwp.hwnd;
    if hwnd != target && IsChild(target, hwnd) == 0 {
        return CallNextHookEx(hook, n_code, w_param, l_param);
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let name = message_name(cwp.message);
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "{}\t{}\t0x{:04X}\t{:?}", ts, name, cwp.message, hwnd);
            let _ = f.flush();
        }
    }
    CallNextHookEx(hook, n_code, w_param, l_param)
}

/// Start logging Win32 messages for the overlay window. Overlay must exist.
/// Log is written to ~/.kalam/overlay-messages.log
pub fn start(app_handle: &tauri::AppHandle) -> Result<(), String> {
    const OVERLAY_LABEL: &str = "overlay";
    let overlay = app_handle
        .get_webview_window(OVERLAY_LABEL)
        .ok_or_else(|| "Overlay window not found".to_string())?;
    let win = overlay.as_ref().window();
    let handle = win
        .window_handle()
        .map_err(|e| format!("window_handle: {:?}", e))?;
    let hwnd = match handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(h) => h.hwnd.get() as HWND,
        _ => return Err("Not Win32".to_string()),
    };
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, std::ptr::null_mut()) };
    if thread_id == 0 {
        return Err("GetWindowThreadProcessId failed".to_string());
    }
    let dir = crate::config::get_kalam_dir().map_err(|e| e.to_string())?;
    let path = dir.join("overlay-messages.log");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let hook = unsafe {
        SetWindowsHookExW(
            WH_CALLWNDPROC_ID,
            Some(callwnd_proc),
            0 as _, // hMod = 0 for thread hook in same process
            thread_id,
        )
    };
    if hook == 0 {
        return Err("SetWindowsHookExW WH_CALLWNDPROC failed".to_string());
    }
    TARGET_HWND.store(hwnd as usize, Ordering::SeqCst);
    unsafe { CURRENT_HOOK = hook };
    *HOOK_HANDLE.lock().unwrap() = Some(hook);
    *LOG_FILE.lock().unwrap() = Some(file);
    log::info!(
        "Overlay message log started (thread_id={}, hwnd={:?}) -> {}",
        thread_id,
        hwnd,
        path.display()
    );
    Ok(())
}

/// Stop logging and close the log file.
pub fn stop() -> Result<(), String> {
    let mut hook_guard = HOOK_HANDLE.lock().unwrap();
    let hook = hook_guard.take();
    drop(hook_guard);
    if let Some(h) = hook {
        unsafe { UnhookWindowsHookEx(h) };
    }
    unsafe {
        CURRENT_HOOK = 0 as HHOOK;
    }
    TARGET_HWND.store(0, Ordering::SeqCst);
    *LOG_FILE.lock().unwrap() = None;
    log::info!("Overlay message log stopped");
    Ok(())
}
