//! Single source of truth for "which non-Kalam app the user is working in".
//!
//! - **`ExternalForegroundCache`**: updated on a timer and on demand (`get_context_previews`);
//!   holds platform injection id (HWND / PID / X11 wid), process name, window title, exe path.
//! - **Dictation start** snapshots injection target from live foreground, or falls back to this
//!   cache when the user starts from Kalam (overlay / main window).

use crate::config::privacy::{get_foreground_app, is_kalam_process, resolve_external_foreground_app};
#[cfg(windows)]
use crate::config::privacy::get_active_win_foreground_details;
use crate::AppState;

/// Last observed external (non-Kalam) foreground. All UI and idle logic should read from here
/// after `refresh_external_foreground_cache`, not from parallel `(String, String)` tuples.
#[derive(Clone, Debug)]
pub struct ExternalForegroundCache {
    /// Windows: HWND; macOS: PID; Linux: X11 window id.
    pub injection_id: Option<usize>,
    pub process_name: String,
    pub window_title: String,
    pub exe_path: Option<String>,
    pub last_updated: std::time::Instant,
}

impl Default for ExternalForegroundCache {
    fn default() -> Self {
        Self {
            injection_id: None,
            process_name: String::new(),
            window_title: String::new(),
            exe_path: None,
            last_updated: std::time::Instant::now(),
        }
    }
}

impl ExternalForegroundCache {
    pub fn as_name_title(&self) -> Option<(String, String)> {
        if self.process_name.trim().is_empty() {
            None
        } else {
            Some((self.process_name.clone(), self.window_title.clone()))
        }
    }
}

/// Resolve process short name + full exe path for a Windows foreground HWND.
#[cfg(windows)]
pub(crate) fn get_foreground_exe_info(hwnd: usize) -> Option<(String, String)> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd as isize, &mut pid) };
    if pid == 0 {
        return None;
    }

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 {
        return None;
    }

    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    unsafe { CloseHandle(handle) };

    if ok == 0 {
        return None;
    }

    let path = String::from_utf16_lossy(&buf[..size as usize]);
    let path = path.trim_end_matches('\0').to_string();
    let short = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())?;
    Some((short, path))
}

/// `active_win_pos_rs` encodes HWND via `Debug` (e.g. `HWND(0x…)`). Parse and cast to `windows-sys` HWND.
#[cfg(windows)]
fn parse_hwnd_from_active_win_window_id(s: &str) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::Foundation::HWND;
    let s = s.trim();
    if let Some(open) = s.find('(') {
        if let Some(close) = s.rfind(')') {
            if close > open {
                let inner = s[open + 1..close].trim();
                let addr = if let Some(hex) = inner.strip_prefix("0x").or_else(|| inner.strip_prefix("0X"))
                {
                    usize::from_str_radix(hex, 16).ok()?
                } else {
                    inner.parse::<usize>().ok()?
                };
                return Some(addr as HWND);
            }
        }
    }
    s.parse::<usize>().ok().map(|a| a as HWND)
}

#[cfg(windows)]
fn hwnd_pid(hwnd: windows_sys::Win32::Foundation::HWND) -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    pid
}

#[cfg(windows)]
fn hwnd_matches_active_pid(hwnd: windows_sys::Win32::Foundation::HWND, active_pid: u32) -> bool {
    hwnd != 0 && hwnd_pid(hwnd) == active_pid && active_pid != 0
}

#[cfg(windows)]
struct EnumBestHwndCtx {
    target_pid: u32,
    title_hint: String,
    best: Option<(usize, i64)>,
}

#[cfg(windows)]
unsafe extern "system" fn enum_best_visible_toplevel(
    hwnd: windows_sys::Win32::Foundation::HWND,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::BOOL {
    use windows_sys::Win32::Foundation::{RECT, TRUE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    };

    if IsWindowVisible(hwnd) == 0 {
        return TRUE;
    }
    let ctx = unsafe { &mut *(lparam as *mut EnumBestHwndCtx) };
    let mut wpid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut wpid) };
    if wpid != ctx.target_pid {
        return TRUE;
    }

    let len = unsafe { GetWindowTextLengthW(hwnd) };
    let title = if len > 0 {
        let mut buf = vec![0u16; len as usize + 1];
        let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1) };
        if n > 0 {
            String::from_utf16_lossy(&buf[..n as usize])
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let area = if unsafe { GetWindowRect(hwnd, &mut rect) } != 0 {
        let w = (rect.right - rect.left).max(0) as i64;
        let h = (rect.bottom - rect.top).max(0) as i64;
        w.saturating_mul(h)
    } else {
        0
    };
    // Drop tiny chrome (tooltips, 1×1 hosts) so we prefer the real app surface.
    if area < 400 {
        return TRUE;
    }

    let hint = ctx.title_hint.to_lowercase();
    let tlow = title.to_lowercase();
    let mut score: i64 = area;
    if !hint.is_empty() && tlow.contains(&hint) {
        score = score.saturating_add(1_000_000_000);
    } else if !title.trim().is_empty() {
        score = score.saturating_add(10_000);
    }

    let hwnd_usize = hwnd as usize;
    match ctx.best {
        None => ctx.best = Some((hwnd_usize, score)),
        Some((_, s)) if score > s => ctx.best = Some((hwnd_usize, score)),
        _ => {}
    }
    TRUE
}

/// Pick an HWND for injection: trust parsed `window_id` if its PID matches active_win, else GF if it
/// matches, else best visible top-level for that PID (handles UWP / frame host vs naive GF).
#[cfg(windows)]
fn resolve_windows_injection_hwnd(
    details: &crate::config::privacy::ActiveWinForegroundDetails,
) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetForegroundWindow};

    let active_pid = details.process_id as u32;

    if let Some(h) = parse_hwnd_from_active_win_window_id(&details.window_id) {
        if hwnd_matches_active_pid(h, active_pid) {
            return Some(h);
        }
    }

    let gf = unsafe { GetForegroundWindow() };
    if hwnd_matches_active_pid(gf, active_pid) {
        return Some(gf);
    }

    let mut ctx = EnumBestHwndCtx {
        target_pid: active_pid,
        title_hint: details.title.clone(),
        best: None,
    };
    let lparam: LPARAM = &mut ctx as *mut EnumBestHwndCtx as isize;
    unsafe {
        EnumWindows(Some(enum_best_visible_toplevel), lparam);
    }
    ctx.best.map(|(addr, _)| addr as HWND)
}

/// Query OS foreground and refresh the cache when it is not Kalam. Safe to call from any thread
/// that can lock `AppState` (non-async lock only here).
#[cfg(windows)]
pub fn refresh_external_foreground_cache(state: &AppState) {
    let Some(details) = get_active_win_foreground_details() else {
        return;
    };
    if details.process_name.trim().is_empty() {
        return;
    }
    if is_kalam_process(&details.process_name) {
        return;
    }

    let injection_id = resolve_windows_injection_hwnd(&details).map(|h| h as usize);

    let exe_path = if !details.process_path.as_os_str().is_empty() {
        Some(details.process_path.to_string_lossy().to_string())
    } else if let Some(id) = injection_id {
        get_foreground_exe_info(id).map(|(_, p)| p)
    } else {
        None
    };

    if let Ok(mut g) = state.external_foreground_cache.lock() {
        *g = Some(ExternalForegroundCache {
            injection_id,
            process_name: details.process_name,
            window_title: details.title,
            exe_path,
            last_updated: std::time::Instant::now(),
        });
    }
}

#[cfg(not(windows))]
pub fn refresh_external_foreground_cache(state: &AppState) {
    let Some((proc, title)) = get_foreground_app() else {
        return;
    };
    if is_kalam_process(&proc) {
        return;
    }
    let window = match active_win_pos_rs::get_active_window() {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut injection_id: Option<usize> = None;
    let mut exe_path: Option<String> = None;
    #[cfg(target_os = "macos")]
    {
        injection_id = Some(window.process_id as usize);
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(wid) = window.window_id.parse::<u32>() {
            injection_id = Some(wid as usize);
        }
    }
    let pid = window.process_id;
    {
        use sysinfo::{Pid, ProcessesToUpdate, System};
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid as u32)]));
        if let Some(p) = sys.process(Pid::from_u32(pid as u32)) {
            if let Some(exe) = p.exe() {
                exe_path = Some(exe.to_string_lossy().to_string());
            }
        }
    }
    if let Ok(mut g) = state.external_foreground_cache.lock() {
        *g = Some(ExternalForegroundCache {
            injection_id,
            process_name: proc,
            window_title: title,
            exe_path,
            last_updated: std::time::Instant::now(),
        });
    }
}

/// Refresh cache, then return live external foreground or last cached external (Kalam-as-live → cache).
pub fn capture_and_resolve_external_foreground(state: &AppState) -> Option<(String, String)> {
    refresh_external_foreground_cache(state);
    let live = get_foreground_app();
    let cached_pair = state
        .external_foreground_cache
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|c| c.as_name_title()));
    resolve_external_foreground_app(live, cached_pair)
}

/// Read cache for overlay previews (caller should call `refresh_external_foreground_cache` first).
pub fn cached_name_title(state: &AppState) -> Option<(String, String)> {
    state
        .external_foreground_cache
        .lock()
        .ok()
        .and_then(|g| {
            if let Some(c) = g.as_ref() {
                if c.last_updated.elapsed() > std::time::Duration::from_secs(2) {
                    log::warn!("External foreground cache is stale ({}s old)", c.last_updated.elapsed().as_secs());
                }
                c.as_name_title()
            } else {
                None
            }
        })
}
