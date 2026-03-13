use lazy_static::lazy_static;
use rdev::{listen, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Windows: KALAM_USE_WILLHOOK=1 uses willhook instead of rdev (optional; ~500 ms gap unchanged). See .doc/latency-profiling-windows.md.
#[cfg(windows)]
fn use_willhook_listener() -> bool {
    std::env::var("KALAM_USE_WILLHOOK").as_deref() == Ok("1")
        || std::env::var("KALAM_USE_WILLHOOK").as_deref() == Ok("true")
}

#[cfg(not(windows))]
fn use_willhook_listener() -> bool {
    false
}

/// Windows: KALAM_USE_RDEV=1 uses rdev for hotkeys (~500 ms gap accepted). Default is hook. See .doc/latency-profiling-windows.md.
#[cfg(windows)]
fn force_rdev() -> bool {
    std::env::var("KALAM_USE_RDEV").as_deref() == Ok("1")
        || std::env::var("KALAM_USE_RDEV").as_deref() == Ok("true")
}

#[cfg(not(windows))]
fn force_rdev() -> bool {
    false
}

#[cfg(windows)]
fn use_same_thread_hotkey() -> bool {
    std::env::var("KALAM_SAME_THREAD_HOTKEY").as_deref() == Ok("1")
        || std::env::var("KALAM_SAME_THREAD_HOTKEY").as_deref() == Ok("true")
}

#[cfg(windows)]
fn use_hook_hotkey() -> bool {
    use_same_thread_hotkey() || !force_rdev()
}

#[cfg(not(windows))]
fn use_hook_hotkey() -> bool {
    false
}

#[cfg(not(windows))]
fn use_same_thread_hotkey() -> bool {
    false
}

/// Windows: set rdev thread to ABOVE_NORMAL priority when rdev is used (measured: no material improvement to ~500 ms gap).
#[cfg(windows)]
fn set_rdev_thread_priority_above_normal() {
    use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL};
    let thread = unsafe { GetCurrentThread() };
    if unsafe { SetThreadPriority(thread, THREAD_PRIORITY_ABOVE_NORMAL) } == 0 {
        log::warn!("SetThreadPriority(ABOVE_NORMAL) failed for rdev thread");
    }
}

/// Called from WH_KEYBOARD_LL hook (Windows). Dispatches key so T0 runs in hook thread (low latency).
#[cfg(windows)]
pub(crate) fn dispatch_key_from_win_hook(vk_code: u32, is_press: bool) {
    if let Some(key) = vk_code_to_key(vk_code) {
        update_modifiers(key, is_press);
        apply_key_event(key, is_press);
    }
}

/// Map Windows VK to rdev Key for hook hotkey matching.
#[cfg(windows)]
fn vk_code_to_key(vk: u32) -> Option<Key> {
    Some(match vk {
        0x41 => Key::KeyA,
        0x42 => Key::KeyB,
        0x43 => Key::KeyC,
        0x44 => Key::KeyD,
        0x45 => Key::KeyE,
        0x46 => Key::KeyF,
        0x47 => Key::KeyG,
        0x48 => Key::KeyH,
        0x49 => Key::KeyI,
        0x4A => Key::KeyJ,
        0x4B => Key::KeyK,
        0x4C => Key::KeyL,
        0x4D => Key::KeyM,
        0x4E => Key::KeyN,
        0x4F => Key::KeyO,
        0x50 => Key::KeyP,
        0x51 => Key::KeyQ,
        0x52 => Key::KeyR,
        0x53 => Key::KeyS,
        0x54 => Key::KeyT,
        0x55 => Key::KeyU,
        0x56 => Key::KeyV,
        0x57 => Key::KeyW,
        0x58 => Key::KeyX,
        0x59 => Key::KeyY,
        0x5A => Key::KeyZ,
        0x30 => Key::Num0,
        0x31 => Key::Num1,
        0x32 => Key::Num2,
        0x33 => Key::Num3,
        0x34 => Key::Num4,
        0x35 => Key::Num5,
        0x36 => Key::Num6,
        0x37 => Key::Num7,
        0x38 => Key::Num8,
        0x39 => Key::Num9,
        0xA2 => Key::ControlLeft,
        0xA3 => Key::ControlRight,
        0xA0 => Key::ShiftLeft,
        0xA1 => Key::ShiftRight,
        0xA4 | 0xA5 => Key::Alt,
        0x5B => Key::MetaLeft,
        0x5C => Key::MetaRight,
        0x20 => Key::Space,
        0x0D => Key::Return,
        0x1B => Key::Escape,
        0x09 => Key::Tab,
        0x08 => Key::Backspace,
        0x2E => Key::Delete,
        0x2D => Key::Insert,
        0x24 => Key::Home,
        0x21 => Key::PageUp,
        0x22 => Key::PageDown,
        0x25 => Key::LeftArrow,
        0x26 => Key::UpArrow,
        0x27 => Key::RightArrow,
        0x28 => Key::DownArrow,
        0x70 => Key::F1,
        0x71 => Key::F2,
        0x72 => Key::F3,
        0x73 => Key::F4,
        0x74 => Key::F5,
        0x75 => Key::F6,
        0x76 => Key::F7,
        0x77 => Key::F8,
        0x78 => Key::F9,
        0x79 => Key::F10,
        0x7A => Key::F11,
        0x7B => Key::F12,
        _ => return None,
    })
}

/// Sync tracked modifier state with OS physical state. Prevents ghost triggers when
/// KeyRelease events were dropped (e.g. after Win+L or UAC). Windows-only.
#[cfg(windows)]
fn verify_modifiers_physical_state() {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    // Virtual key codes: Left/Right Control, Shift, Menu(Alt), Win
    const VK_LCONTROL: i32 = 0xA2;
    const VK_RCONTROL: i32 = 0xA3;
    const VK_LSHIFT: i32 = 0xA0;
    const VK_RSHIFT: i32 = 0xA1;
    const VK_LMENU: i32 = 0xA4;
    const VK_RMENU: i32 = 0xA5;
    const VK_LWIN: i32 = 0x5B;
    const VK_RWIN: i32 = 0x5C;
    // GetAsyncKeyState returns i16; MSB set (negative) means key is currently down
    let ctrl = unsafe { GetAsyncKeyState(VK_LCONTROL) < 0 || GetAsyncKeyState(VK_RCONTROL) < 0 };
    let shift = unsafe { GetAsyncKeyState(VK_LSHIFT) < 0 || GetAsyncKeyState(VK_RSHIFT) < 0 };
    let alt = unsafe { GetAsyncKeyState(VK_LMENU) < 0 || GetAsyncKeyState(VK_RMENU) < 0 };
    let meta = unsafe { GetAsyncKeyState(VK_LWIN) < 0 || GetAsyncKeyState(VK_RWIN) < 0 };
    CTRL_PRESSED.store(ctrl, Ordering::SeqCst);
    SHIFT_PRESSED.store(shift, Ordering::SeqCst);
    ALT_PRESSED.store(alt, Ordering::SeqCst);
    META_PRESSED.store(meta, Ordering::SeqCst);
}

#[cfg(not(windows))]
fn verify_modifiers_physical_state() {}

lazy_static! {
    static ref CTRL_PRESSED: AtomicBool = AtomicBool::new(false);
    static ref ALT_PRESSED: AtomicBool = AtomicBool::new(false);
    static ref SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);
    static ref META_PRESSED: AtomicBool = AtomicBool::new(false);
    /// Set when dictation hotkey is active (used on Windows to suppress Start Menu on Win key release).
    pub(crate) static ref HOTKEY_ACTIVE: AtomicBool = AtomicBool::new(false);
    /// When true, global hotkey callbacks are not fired (e.g. while user is setting a hotkey in UI).
    static ref HOTKEYS_PAUSED: AtomicBool = AtomicBool::new(false);

    /// Global registry of hotkeys that can be updated dynamically
    pub static ref HOTKEY_REGISTRATIONS: Arc<Mutex<Vec<HotkeyRegistration>>> = Arc::new(Mutex::new(Vec::new()));
}

/// Pause or resume global hotkey handling. Used when the user is capturing a new hotkey in settings.
pub fn set_hotkeys_paused(paused: bool) {
    HOTKEYS_PAUSED.store(paused, Ordering::SeqCst);
}

/// One hotkey registration with its own active state and callbacks.
pub struct HotkeyRegistration {
    pub target: RdevHotkey,
    pub active: Arc<AtomicBool>,
    pub on_press: Arc<dyn Fn() + Send + Sync>,
    pub on_release: Arc<dyn Fn() + Send + Sync>,
    /// When set, called when a modifier-only hotkey is cancelled (e.g. user pressed another key).
    pub on_cancel: Option<Arc<dyn Fn() + Send + Sync>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RdevHotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub main_key: Option<Key>,
}

pub fn parse_rdev_hotkey(hotkey_str: &str) -> anyhow::Result<RdevHotkey> {
    let parts: Vec<&str> = hotkey_str.split('+').map(|s| s.trim()).collect();

    let mut hotkey = RdevHotkey {
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
        main_key: None,
    };

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => hotkey.ctrl = true,
            "alt" => hotkey.alt = true,
            "shift" => hotkey.shift = true,
            "super" | "win" | "command" | "cmd" | "meta" => hotkey.meta = true,
            key => {
                hotkey.main_key = Some(parse_rdev_key_code(key)?);
            }
        }
    }

    let has_any_modifier = hotkey.ctrl || hotkey.alt || hotkey.shift || hotkey.meta;
    if hotkey.main_key.is_none() && !has_any_modifier {
        return Err(anyhow::anyhow!(
            "Hotkey must have at least one key or modifier"
        ));
    }

    Ok(hotkey)
}

fn parse_rdev_key_code(key: &str) -> anyhow::Result<Key> {
    match key.to_lowercase().as_str() {
        "a" => Ok(Key::KeyA),
        "b" => Ok(Key::KeyB),
        "c" => Ok(Key::KeyC),
        "d" => Ok(Key::KeyD),
        "e" => Ok(Key::KeyE),
        "f" => Ok(Key::KeyF),
        "g" => Ok(Key::KeyG),
        "h" => Ok(Key::KeyH),
        "i" => Ok(Key::KeyI),
        "j" => Ok(Key::KeyJ),
        "k" => Ok(Key::KeyK),
        "l" => Ok(Key::KeyL),
        "m" => Ok(Key::KeyM),
        "n" => Ok(Key::KeyN),
        "o" => Ok(Key::KeyO),
        "p" => Ok(Key::KeyP),
        "q" => Ok(Key::KeyQ),
        "r" => Ok(Key::KeyR),
        "s" => Ok(Key::KeyS),
        "t" => Ok(Key::KeyT),
        "u" => Ok(Key::KeyU),
        "v" => Ok(Key::KeyV),
        "w" => Ok(Key::KeyW),
        "x" => Ok(Key::KeyX),
        "y" => Ok(Key::KeyY),
        "z" => Ok(Key::KeyZ),
        "0" => Ok(Key::Num0),
        "1" => Ok(Key::Num1),
        "2" => Ok(Key::Num2),
        "3" => Ok(Key::Num3),
        "4" => Ok(Key::Num4),
        "5" => Ok(Key::Num5),
        "6" => Ok(Key::Num6),
        "7" => Ok(Key::Num7),
        "8" => Ok(Key::Num8),
        "9" => Ok(Key::Num9),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        "enter" => Ok(Key::Return),
        "space" => Ok(Key::Space),
        "escape" | "esc" => Ok(Key::Escape),
        "tab" => Ok(Key::Tab),
        "backspace" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "insert" | "ins" => Ok(Key::Insert),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" | "page_up" => Ok(Key::PageUp),
        "pagedown" | "page_down" => Ok(Key::PageDown),
        "up" => Ok(Key::UpArrow),
        "down" => Ok(Key::DownArrow),
        "left" => Ok(Key::LeftArrow),
        "right" => Ok(Key::RightArrow),
        _ => Err(anyhow::anyhow!("Unknown key code: {}", key)),
    }
}

/// Start the global key listener with multiple hotkey registrations.
/// The first registration (index 0) is the dictation hotkey; when it is active, HOTKEY_ACTIVE is set (for Windows Win-key suppression).
pub fn start_listener(registrations: Vec<HotkeyRegistration>) {
    #[allow(unused_variables)]
    let any_meta = registrations.iter().any(|r| r.target.meta);
    #[cfg(windows)]
    if any_meta {
        crate::hotkey_win::start_win_key_suppression();
    }
    #[cfg(windows)]
    crate::hotkey_win::start_latency_debug_keyboard_hook(use_hook_hotkey());

    {
        let mut regs = HOTKEY_REGISTRATIONS.lock().unwrap();
        *regs = registrations;
    }

    // Only start the listener thread once
    static LISTENER_STARTED: AtomicBool = AtomicBool::new(false);
    if LISTENER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    if use_hook_hotkey() {
        log::info!("Hotkey: WH_KEYBOARD_LL hook (Windows). KALAM_USE_RDEV=1 for rdev.");
        return;
    }

    if use_willhook_listener() {
        #[cfg(windows)]
        start_willhook_listener();
    } else {
        // rdev (Linux, macOS, or Windows with KALAM_USE_RDEV=1). Windows: thread priority ABOVE_NORMAL (see latency-profiling-windows.md).
        std::thread::spawn(move || {
            #[cfg(windows)]
            set_rdev_thread_priority_above_normal();
            if let Err(error) = listen(move |event| {
                handle_event_multi(event);
            }) {
                log::error!("Error in rdev listener: {:?}", error);
            }
        });
    }
}

pub fn update_registrations(registrations: Vec<HotkeyRegistration>) {
    #[allow(unused_variables)]
    let any_meta = registrations.iter().any(|r| r.target.meta);
    #[cfg(windows)]
    if any_meta {
        crate::hotkey_win::start_win_key_suppression();
    }

    let mut regs = HOTKEY_REGISTRATIONS.lock().unwrap();
    *regs = registrations;
}

fn handle_event_multi(event: Event) {
    if HOTKEYS_PAUSED.load(Ordering::SeqCst) {
        if let EventType::KeyPress(k) = event.event_type {
            update_modifiers(k, true);
        }
        if let EventType::KeyRelease(k) = event.event_type {
            update_modifiers(k, false);
        }
        return;
    }

    let (key, is_press) = match event.event_type {
        EventType::KeyPress(k) => {
            update_modifiers(k, true);
            (k, true)
        }
        EventType::KeyRelease(k) => {
            update_modifiers(k, false);
            (k, false)
        }
        _ => return,
    };

    apply_key_event(key, is_press);
}

/// Hotkey match and callback dispatch. Used by rdev, willhook (Windows), and WH_KEYBOARD_LL hook.
fn apply_key_event(key: Key, is_press: bool) {
    if is_press {
        verify_modifiers_physical_state();

        let regs = HOTKEY_REGISTRATIONS.lock().unwrap();
        for (idx, reg) in regs.iter().enumerate() {
            if reg.active.load(Ordering::SeqCst)
                && reg.target.main_key.is_none()
                && !is_modifier_key(key)
            {
                reg.active.store(false, Ordering::SeqCst);
                if idx == 0 {
                    HOTKEY_ACTIVE.store(false, Ordering::SeqCst);
                }
                if let Some(on_cancel) = &reg.on_cancel {
                    on_cancel();
                }
                return;
            }
        }
        for (idx, reg) in regs.iter().enumerate() {
            let activated = match reg.target.main_key {
                Some(main_key) => {
                    key == main_key
                        && modifiers_match(&reg.target)
                        && !reg.active.load(Ordering::SeqCst)
                }
                None => {
                    is_modifier_key(key)
                        && modifiers_match(&reg.target)
                        && !reg.active.load(Ordering::SeqCst)
                }
            };
            if activated {
                reg.active.store(true, Ordering::SeqCst);
                if idx == 0 {
                    HOTKEY_ACTIVE.store(true, Ordering::SeqCst);
                }
                (reg.on_press)();
                break;
            }
        }
    } else {
        let regs = HOTKEY_REGISTRATIONS.lock().unwrap();
        for (idx, reg) in regs.iter().enumerate() {
            if !reg.active.load(Ordering::SeqCst) {
                continue;
            }
            let release = match reg.target.main_key {
                Some(main_key) => key == main_key,
                None => !modifiers_match(&reg.target),
            };
            if release {
                reg.active.store(false, Ordering::SeqCst);
                if idx == 0 {
                    HOTKEY_ACTIVE.store(false, Ordering::SeqCst);
                }
                (reg.on_release)();
                break;
            }
        }
    }
}

fn is_modifier_key(key: Key) -> bool {
    matches!(
        key,
        Key::ControlLeft
            | Key::ControlRight
            | Key::Alt
            | Key::AltGr
            | Key::ShiftLeft
            | Key::ShiftRight
            | Key::MetaLeft
            | Key::MetaRight
    )
}

fn update_modifiers(key: Key, pressed: bool) {
    match key {
        Key::ControlLeft | Key::ControlRight => CTRL_PRESSED.store(pressed, Ordering::SeqCst),
        Key::Alt | Key::AltGr => ALT_PRESSED.store(pressed, Ordering::SeqCst),
        Key::ShiftLeft | Key::ShiftRight => SHIFT_PRESSED.store(pressed, Ordering::SeqCst),
        Key::MetaLeft | Key::MetaRight => META_PRESSED.store(pressed, Ordering::SeqCst),
        _ => (),
    }
}

fn modifiers_match(target: &RdevHotkey) -> bool {
    target.ctrl == CTRL_PRESSED.load(Ordering::SeqCst)
        && target.alt == ALT_PRESSED.load(Ordering::SeqCst)
        && target.shift == SHIFT_PRESSED.load(Ordering::SeqCst)
        && target.meta == META_PRESSED.load(Ordering::SeqCst)
}

/// Windows: KALAM_USE_WILLHOOK=1. Optional; ~500 ms gap same as rdev. See .doc/latency-profiling-windows.md.
#[cfg(windows)]
fn start_willhook_listener() {
    use willhook::event::{InputEvent, KeyPress};
    use willhook::keyboard_hook;

    std::thread::spawn(|| {
        let hook = match keyboard_hook() {
            Some(h) => h,
            None => {
                log::error!("willhook keyboard_hook returned None");
                return;
            }
        };
        log::info!("Hotkey: willhook listener (KALAM_USE_WILLHOOK=1).");
        loop {
            match hook.recv() {
                Ok(InputEvent::Keyboard(ke)) => {
                    let is_press = matches!(ke.pressed, KeyPress::Down(_));
                    if let Some(wh_key) = ke.key {
                        if let Some(k) = willhook_key_to_rdev(wh_key) {
                            apply_key_event(k, is_press);
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
}

/// Map willhook KeyboardKey to rdev Key (when KALAM_USE_WILLHOOK=1).
#[cfg(windows)]
fn willhook_key_to_rdev(key: willhook::event::KeyboardKey) -> Option<Key> {
    use willhook::event::KeyboardKey as WK;
    Some(match key {
        WK::A => Key::KeyA,
        WK::B => Key::KeyB,
        WK::C => Key::KeyC,
        WK::D => Key::KeyD,
        WK::E => Key::KeyE,
        WK::F => Key::KeyF,
        WK::G => Key::KeyG,
        WK::H => Key::KeyH,
        WK::I => Key::KeyI,
        WK::J => Key::KeyJ,
        WK::K => Key::KeyK,
        WK::L => Key::KeyL,
        WK::M => Key::KeyM,
        WK::N => Key::KeyN,
        WK::O => Key::KeyO,
        WK::P => Key::KeyP,
        WK::Q => Key::KeyQ,
        WK::R => Key::KeyR,
        WK::S => Key::KeyS,
        WK::T => Key::KeyT,
        WK::U => Key::KeyU,
        WK::V => Key::KeyV,
        WK::W => Key::KeyW,
        WK::X => Key::KeyX,
        WK::Y => Key::KeyY,
        WK::Z => Key::KeyZ,
        WK::Number0 => Key::Num0,
        WK::Number1 => Key::Num1,
        WK::Number2 => Key::Num2,
        WK::Number3 => Key::Num3,
        WK::Number4 => Key::Num4,
        WK::Number5 => Key::Num5,
        WK::Number6 => Key::Num6,
        WK::Number7 => Key::Num7,
        WK::Number8 => Key::Num8,
        WK::Number9 => Key::Num9,
        WK::LeftControl => Key::ControlLeft,
        WK::RightControl => Key::ControlRight,
        WK::LeftAlt | WK::RightAlt => Key::Alt,
        WK::LeftShift => Key::ShiftLeft,
        WK::RightShift => Key::ShiftRight,
        WK::LeftWindows => Key::MetaLeft,
        WK::RightWindows => Key::MetaRight,
        WK::Space => Key::Space,
        WK::Enter => Key::Return,
        WK::Escape => Key::Escape,
        WK::Tab => Key::Tab,
        WK::BackSpace => Key::Backspace,
        WK::Delete => Key::Delete,
        WK::Insert => Key::Insert,
        WK::Home => Key::Home,
        WK::PageUp => Key::PageUp,
        WK::PageDown => Key::PageDown,
        WK::ArrowLeft => Key::LeftArrow,
        WK::ArrowRight => Key::RightArrow,
        WK::ArrowUp => Key::UpArrow,
        WK::ArrowDown => Key::DownArrow,
        WK::F1 => Key::F1,
        WK::F2 => Key::F2,
        WK::F3 => Key::F3,
        WK::F4 => Key::F4,
        WK::F5 => Key::F5,
        WK::F6 => Key::F6,
        WK::F7 => Key::F7,
        WK::F8 => Key::F8,
        WK::F9 => Key::F9,
        WK::F10 => Key::F10,
        WK::F11 => Key::F11,
        WK::F12 => Key::F12,
        WK::Other(_) | WK::InvalidKeyCodeReceived => return None,
        _ => return None,
    })
}
