//! Windows: UI Automation for focused control text, `arboard` for clipboard, `enigo` for Ctrl+C selection shim.

use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use uiautomation::patterns::{UITextPattern, UIValuePattern};
use uiautomation::types::Handle;
use uiautomation::{UIAutomation, UIElement};

use crate::config::ModeContextConfig;

use super::super::{
    truncate_str, CapturedContext, MAX_APP_TEXT_CHARS, MAX_CLIPBOARD_CHARS, MAX_SELECTION_CHARS,
};

pub fn capture_selected_text_impl() -> Option<String> {
    let mut clipboard = Clipboard::new().ok()?;
    let old = clipboard.get_text().ok();
    let old_str = old.as_deref().unwrap_or("");

    let mut enigo = Enigo::new(&Settings::default()).ok()?;
    enigo.key(Key::Control, Direction::Press).ok()?;
    enigo.key(Key::Unicode('c'), Direction::Click).ok()?;
    enigo.key(Key::Control, Direction::Release).ok()?;

    std::thread::sleep(Duration::from_millis(120));

    let new = clipboard.get_text().ok()?;
    if let Err(e) = if old_str.is_empty() {
        clipboard.clear()
    } else {
        clipboard.set_text(old_str)
    } {
        log::warn!(
            "context: failed to restore clipboard after selection shim: {}",
            e
        );
    }

    if new == old_str || new.trim().is_empty() {
        return None;
    }
    Some(truncate_str(&new, MAX_SELECTION_CHARS))
}

/// Clipboard text for overlay “context preview” panel (Phase 6); truncated to limit PII in IPC.
pub fn clipboard_text_truncated_for_overlay(max_chars: usize) -> Option<String> {
    let mut clipboard = Clipboard::new().ok()?;
    let t = clipboard.get_text().ok()?;
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    Some(super::super::truncate_str(t, max_chars))
}

pub fn capture_post_session_impl(
    mode_ctx: &ModeContextConfig,
    foreground_hwnd: Option<usize>,
) -> CapturedContext {
    let mut out = CapturedContext::default();
    if mode_ctx.read_clipboard {
        if let Ok(mut cb) = Clipboard::new() {
            if let Ok(t) = cb.get_text() {
                let t = t.trim();
                if !t.is_empty() {
                    out.clipboard_text = Some(truncate_str(t, MAX_CLIPBOARD_CHARS));
                }
            }
        }
    }
    if mode_ctx.include_system_info {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let user = whoami::username();
        out.system_info = Some(format!("Date/time: {}, User: {}", now, user));
    }
    if mode_ctx.read_app {
        let (name, text) = read_app_uia_bounded(foreground_hwnd);
        out.app_name = name;
        out.app_text = text;
    }
    out
}

fn read_app_uia_bounded(foreground_hwnd: Option<usize>) -> (Option<String>, Option<String>) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(read_app_uia_inner(foreground_hwnd));
    });
    match rx.recv_timeout(Duration::from_millis(500)) {
        Ok(v) => v,
        Err(_) => {
            log::warn!("context: UIA capture timed out (500ms)");
            (None, None)
        }
    }
}

fn read_app_uia_inner(foreground_hwnd: Option<usize>) -> (Option<String>, Option<String>) {
    let automation = match UIAutomation::new() {
        Ok(a) => a,
        Err(e) => {
            log::debug!("context: UIAutomation::new failed: {:?}", e);
            return (None, None);
        }
    };

    let mut window_label: Option<String> = None;
    if let Some(h) = foreground_hwnd.filter(|&x| x != 0) {
        if let Ok(el) = automation.element_from_handle(Handle::from(h as isize)) {
            window_label = el.get_name().ok().filter(|s| !s.trim().is_empty());
            if let Some(t) = try_extract_text_from_element(&el) {
                return (window_label, Some(truncate_str(&t, MAX_APP_TEXT_CHARS)));
            }
        }
    }

    let focused = match automation.get_focused_element() {
        Ok(e) => e,
        Err(e) => {
            log::debug!("context: get_focused_element failed: {:?}", e);
            return (window_label, None);
        }
    };

    if let Some(t) = walk_focused_chain(&automation, focused) {
        return (window_label, Some(truncate_str(&t, MAX_APP_TEXT_CHARS)));
    }

    (window_label, None)
}

fn try_extract_text_from_element(el: &UIElement) -> Option<String> {
    if let Ok(tp) = el.get_pattern::<UITextPattern>() {
        if let Some(t) = text_from_text_pattern(&tp) {
            return Some(t);
        }
    }
    if let Ok(vp) = el.get_pattern::<UIValuePattern>() {
        if let Ok(v) = vp.get_value() {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn text_from_text_pattern(tp: &UITextPattern) -> Option<String> {
    if let Ok(dr) = tp.get_document_range() {
        if let Ok(t) = dr.get_text(-1) {
            let t = t.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    if let Ok(ranges) = tp.get_visible_ranges() {
        let mut acc = String::new();
        for r in ranges {
            if let Ok(t) = r.get_text(-1) {
                acc.push_str(&t);
            }
        }
        let acc = acc.trim().to_string();
        if !acc.is_empty() {
            return Some(acc);
        }
    }
    None
}

fn walk_focused_chain(automation: &UIAutomation, focused: UIElement) -> Option<String> {
    let walker = automation.get_raw_view_walker().ok()?;
    let mut current = focused;
    for _ in 0..16 {
        if let Some(t) = try_extract_text_from_element(&current) {
            return Some(t);
        }
        match walker.get_parent(&current) {
            Ok(p) => current = p,
            Err(_) => break,
        }
    }
    None
}
