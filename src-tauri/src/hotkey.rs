use lazy_static::lazy_static;
use rdev::{listen, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
    let any_meta = registrations.iter().any(|r| r.target.meta);
    #[cfg(windows)]
    if any_meta {
        crate::hotkey_win::start_win_key_suppression();
    }

    {
        let mut regs = HOTKEY_REGISTRATIONS.lock().unwrap();
        *regs = registrations;
    }

    // Only start the listener thread once
    static LISTENER_STARTED: AtomicBool = AtomicBool::new(false);
    if LISTENER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        if let Err(error) = listen(move |event| {
            handle_event_multi(event);
        }) {
            log::error!("Error in rdev listener: {:?}", error);
        }
    });
}

pub fn update_registrations(registrations: Vec<HotkeyRegistration>) {
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

    let key = match event.event_type {
        EventType::KeyPress(k) => {
            update_modifiers(k, true);
            k
        }
        EventType::KeyRelease(k) => {
            update_modifiers(k, false);
            k
        }
        _ => return,
    };

    match event.event_type {
        EventType::KeyPress(_) => {
            let regs = HOTKEY_REGISTRATIONS.lock().unwrap();
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
        }
        EventType::KeyRelease(_) => {
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
        _ => (),
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
