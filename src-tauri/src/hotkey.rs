use lazy_static::lazy_static;
use rdev::{listen, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static! {
    static ref CTRL_PRESSED: AtomicBool = AtomicBool::new(false);
    static ref ALT_PRESSED: AtomicBool = AtomicBool::new(false);
    static ref SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);
    static ref META_PRESSED: AtomicBool = AtomicBool::new(false);
    static ref MAIN_KEY_PRESSED: AtomicBool = AtomicBool::new(false);
}

#[derive(Debug, Clone, PartialEq)]
pub struct RdevHotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub main_key: Key,
}

pub fn parse_rdev_hotkey(hotkey_str: &str) -> anyhow::Result<RdevHotkey> {
    let parts: Vec<&str> = hotkey_str.split('+').map(|s| s.trim()).collect();

    let mut hotkey = RdevHotkey {
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
        main_key: Key::Unknown(0),
    };

    let mut key_code = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => hotkey.ctrl = true,
            "alt" => hotkey.alt = true,
            "shift" => hotkey.shift = true,
            "super" | "win" | "command" | "cmd" | "meta" => hotkey.meta = true,
            key => {
                key_code = Some(parse_rdev_key_code(key)?);
            }
        }
    }

    hotkey.main_key = key_code.ok_or_else(|| anyhow::anyhow!("No key code found"))?;
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

pub fn start_listener(
    target_hotkey: RdevHotkey,
    on_press: impl Fn() + Send + Sync + 'static,
    on_release: impl Fn() + Send + Sync + 'static,
) {
    let press_cb = Arc::new(on_press);
    let release_cb = Arc::new(on_release);

    std::thread::spawn(move || {
        if let Err(error) = listen(move |event| {
            handle_event(event, &target_hotkey, press_cb.clone(), release_cb.clone());
        }) {
            log::error!("Error in rdev listener: {:?}", error);
        }
    });
}

fn handle_event(
    event: Event,
    target: &RdevHotkey,
    on_press: Arc<impl Fn()>,
    on_release: Arc<impl Fn()>,
) {
    match event.event_type {
        EventType::KeyPress(key) => {
            update_modifiers(key, true);

            if key == target.main_key {
                MAIN_KEY_PRESSED.store(true, Ordering::SeqCst);

                // Check if modifiers match target
                if modifiers_match(target) {
                    on_press();
                }
            }
        }
        EventType::KeyRelease(key) => {
            update_modifiers(key, false);

            if key == target.main_key {
                if MAIN_KEY_PRESSED.load(Ordering::SeqCst) {
                    on_release();
                }
                MAIN_KEY_PRESSED.store(false, Ordering::SeqCst);
            }
        }
        _ => (),
    }
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
