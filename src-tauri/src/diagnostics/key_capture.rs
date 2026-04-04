//! Timed global key capture for diagnostics (Windows: WH_KEYBOARD_LL + short message loop).

use crate::diagnostics::TestEvent;
use std::sync::{Arc, Mutex};

#[cfg(windows)]
use std::mem::zeroed;
#[cfg(windows)]
use std::ptr::null_mut;
#[cfg(windows)]
use std::sync::atomic::{AtomicIsize, Ordering};
#[cfg(windows)]
use std::sync::mpsc;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_QUIT,
};

#[cfg(windows)]
lazy_static::lazy_static! {
    static ref WIN_CAPTURE_SLOT: Mutex<Option<Arc<Mutex<Vec<CaptureRow>>>>> = Mutex::new(None);
}

#[cfg(windows)]
static WIN_CAPTURE_HHOOK: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
struct CaptureRow {
    ts: chrono::DateTime<chrono::Local>,
    event_type: String,
    key: String,
}

#[cfg(windows)]
fn vk_to_key_label(vk: u32) -> Option<&'static str> {
    Some(match vk {
        0x41..=0x5A => match vk {
            0x41 => "KeyA",
            0x42 => "KeyB",
            0x43 => "KeyC",
            0x44 => "KeyD",
            0x45 => "KeyE",
            0x46 => "KeyF",
            0x47 => "KeyG",
            0x48 => "KeyH",
            0x49 => "KeyI",
            0x4A => "KeyJ",
            0x4B => "KeyK",
            0x4C => "KeyL",
            0x4D => "KeyM",
            0x4E => "KeyN",
            0x4F => "KeyO",
            0x50 => "KeyP",
            0x51 => "KeyQ",
            0x52 => "KeyR",
            0x53 => "KeyS",
            0x54 => "KeyT",
            0x55 => "KeyU",
            0x56 => "KeyV",
            0x57 => "KeyW",
            0x58 => "KeyX",
            0x59 => "KeyY",
            0x5A => "KeyZ",
            _ => return None,
        },
        0x30..=0x39 => match vk {
            0x30 => "Num0",
            0x31 => "Num1",
            0x32 => "Num2",
            0x33 => "Num3",
            0x34 => "Num4",
            0x35 => "Num5",
            0x36 => "Num6",
            0x37 => "Num7",
            0x38 => "Num8",
            0x39 => "Num9",
            _ => return None,
        },
        0xA2 => "ControlLeft",
        0xA3 => "ControlRight",
        0xA0 => "ShiftLeft",
        0xA1 => "ShiftRight",
        0xA4 | 0xA5 => "Alt",
        0x5B => "MetaLeft",
        0x5C => "MetaRight",
        0x20 => "Space",
        0x0D => "Return",
        0x1B => "Escape",
        0x09 => "Tab",
        0x08 => "Backspace",
        0x2E => "Delete",
        0x70 => "F1",
        0x71 => "F2",
        0x72 => "F3",
        0x73 => "F4",
        0x74 => "F5",
        0x75 => "F6",
        0x76 => "F7",
        0x77 => "F8",
        0x78 => "F9",
        0x79 => "F10",
        0x7A => "F11",
        0x7B => "F12",
        _ => return None,
    })
}

#[cfg(windows)]
unsafe extern "system" fn capture_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code == HC_ACTION as i32 && l_param != 0 {
        let hook_struct = &*(l_param as *const KBDLLHOOKSTRUCT);
        let vk = hook_struct.vkCode;
        let is_press = w_param == WM_KEYDOWN as WPARAM;
        if let Some(label) = vk_to_key_label(vk) {
            if let Ok(guard) = WIN_CAPTURE_SLOT.lock() {
                if let Some(buf) = guard.as_ref() {
                    if let Ok(mut rows) = buf.lock() {
                        rows.push(CaptureRow {
                            ts: chrono::Local::now(),
                            event_type: if is_press {
                                "KeyDown".to_string()
                            } else {
                                "KeyUp".to_string()
                            },
                            key: label.to_string(),
                        });
                        const MAX: usize = 500;
                        while rows.len() > MAX {
                            rows.remove(0);
                        }
                    }
                }
            }
        }
    }
    let h = WIN_CAPTURE_HHOOK.load(Ordering::SeqCst);
    CallNextHookEx(h as HHOOK, n_code, w_param, l_param)
}

#[cfg(windows)]
enum CaptureThreadMsg {
    Ready(u32),
    Failed(String),
}

/// Capture global key events for `duration_secs` using a dedicated WH_KEYBOARD_LL hook and message loop.
#[cfg(windows)]
pub fn timed_capture(duration_secs: u64) -> Result<Vec<TestEvent>, String> {
    let events = Arc::new(Mutex::new(Vec::<CaptureRow>::new()));
    {
        let mut slot = WIN_CAPTURE_SLOT
            .lock()
            .map_err(|_| "capture mutex poisoned".to_string())?;
        *slot = Some(events.clone());
    }

    let (tx, rx) = mpsc::channel::<CaptureThreadMsg>();
    let join = std::thread::spawn(move || unsafe {
        let hmod = GetModuleHandleW(null_mut());
        if hmod == 0 {
            let _ = tx.send(CaptureThreadMsg::Failed(
                "GetModuleHandleW failed".to_string(),
            ));
            return;
        }
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(capture_keyboard_proc), hmod, 0);
        if hook == 0 {
            let _ = tx.send(CaptureThreadMsg::Failed(
                "SetWindowsHookExW failed for capture".to_string(),
            ));
            return;
        }
        WIN_CAPTURE_HHOOK.store(hook, Ordering::SeqCst);
        let tid = GetCurrentThreadId();
        if tx.send(CaptureThreadMsg::Ready(tid)).is_err() {
            UnhookWindowsHookEx(hook as HHOOK);
            WIN_CAPTURE_HHOOK.store(0, Ordering::SeqCst);
            return;
        }

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnhookWindowsHookEx(hook as HHOOK);
        WIN_CAPTURE_HHOOK.store(0, Ordering::SeqCst);
    });

    let thread_id = match rx.recv() {
        Ok(CaptureThreadMsg::Ready(tid)) => tid,
        Ok(CaptureThreadMsg::Failed(e)) => {
            let _ = join.join();
            let mut slot = WIN_CAPTURE_SLOT.lock().map_err(|_| "mutex poisoned")?;
            *slot = None;
            return Err(e);
        }
        Err(_) => {
            let _ = join.join();
            let mut slot = WIN_CAPTURE_SLOT.lock().map_err(|_| "mutex poisoned")?;
            *slot = None;
            return Err("Capture thread failed to start".to_string());
        }
    };

    std::thread::sleep(std::time::Duration::from_secs(duration_secs));

    unsafe {
        let _ = PostThreadMessageW(thread_id, WM_QUIT, 0, 0);
    }

    let _ = join.join();

    let mut slot = WIN_CAPTURE_SLOT
        .lock()
        .map_err(|_| "capture mutex poisoned".to_string())?;
    *slot = None;

    let rows = events
        .lock()
        .map_err(|_| "events mutex poisoned".to_string())?;
    Ok(rows
        .iter()
        .map(|r| TestEvent {
            timestamp: r.ts.to_rfc3339(),
            event_type: r.event_type.clone(),
            key: r.key.clone(),
        })
        .collect())
}

#[cfg(not(windows))]
pub fn timed_capture(duration_secs: u64) -> Result<Vec<TestEvent>, String> {
    use rdev::{listen, EventType};
    use std::sync::mpsc as std_mpsc;

    let events = Arc::new(Mutex::new(Vec::<TestEvent>::new()));
    let events_listen = events.clone();
    let (tx_err, rx_err) = std_mpsc::channel::<String>();

    std::thread::spawn(move || {
        if let Err(e) = listen(move |ev| {
            let (key, is_press) = match ev.event_type {
                EventType::KeyPress(k) => (k, true),
                EventType::KeyRelease(k) => (k, false),
                _ => return,
            };
            let mut g = match events_listen.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            g.push(TestEvent {
                timestamp: chrono::Local::now().to_rfc3339(),
                event_type: if is_press {
                    "KeyDown".into()
                } else {
                    "KeyUp".into()
                },
                key: format!("{key:?}"),
            });
            const MAX: usize = 500;
            while g.len() > MAX {
                g.remove(0);
            }
        }) {
            let _ = tx_err.send(format!("{e:?}"));
        }
    });

    std::thread::sleep(std::time::Duration::from_secs(duration_secs));

    if let Ok(err) = rx_err.try_recv() {
        log::warn!("[DIAGNOSTIC] rdev listen ended: {err}");
    }

    let g = events
        .lock()
        .map_err(|_| "events mutex poisoned".to_string())?;
    Ok(g.clone())
}
