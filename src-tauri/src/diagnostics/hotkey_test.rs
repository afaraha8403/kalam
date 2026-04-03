use crate::diagnostics::key_capture;
use crate::diagnostics::{
    HookInstallationResult, HotkeyMatchResult, KeyCaptureResult, TestEvent,
};
use crate::hotkey::RdevHotkey;
use std::collections::HashSet;

/// Probe whether a low-level keyboard hook can be installed (Windows), or document N/A elsewhere.
pub fn test_hook_installation() -> Result<HookInstallationResult, String> {
    Ok(crate::diagnostics::hook_probe::probe_wh_keyboard_ll())
}

/// Record global key events for `duration_secs` (dedicated capture path; does not replace the app listener).
pub fn test_key_capture(duration_secs: u64) -> Result<KeyCaptureResult, String> {
    let capped = duration_secs.clamp(1, 120);
    log::info!("[DIAGNOSTIC] Key capture for {capped}s — press modifiers and letter keys as needed.");

    let test_events = key_capture::timed_capture(capped)?;
    let keys_captured = test_events.len();

    let modifier_keys_seen: Vec<String> = test_events
        .iter()
        .filter(|e| {
            e.key.contains("Control")
                || e.key.contains("Shift")
                || e.key.contains("Alt")
                || e.key.contains("Meta")
        })
        .map(|e| e.key.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let mut errors = Vec::new();
    if keys_captured == 0 {
        errors.push("No keys were captured.".to_string());
        errors.push("If you pressed keys, another app or policy may be blocking input capture.".to_string());
    }

    let success = keys_captured > 0;

    Ok(KeyCaptureResult {
        success,
        keys_captured,
        duration_secs: capped,
        modifier_keys_seen,
        errors,
    })
}

/// Parse `hotkey_str` with the same parser as Kalam, then capture keys for 10s and check whether the combo appeared.
pub fn test_hotkey_matching(hotkey_str: &str) -> Result<HotkeyMatchResult, String> {
    let parse_result = crate::hotkey::parse_rdev_hotkey(hotkey_str);

    let (target, parsed_ok, parse_error) = match parse_result {
        Ok(t) => (t, true, None),
        Err(e) => {
            return Ok(HotkeyMatchResult {
                success: false,
                hotkey_str: hotkey_str.to_string(),
                parsed_successfully: false,
                parse_error: Some(e.to_string()),
                test_events: vec![],
                would_trigger: false,
                errors: vec![format!("Invalid hotkey string: {e}")],
            });
        }
    };

    log::info!("[DIAGNOSTIC] Hotkey match test — press the combination within 10 seconds.");

    let test_events = key_capture::timed_capture(10)?;
    let would_trigger = events_satisfy_target(&test_events, &target);

    let errors = if would_trigger {
        vec![]
    } else {
        vec!["Expected combination was not detected in the capture window.".to_string()]
    };

    Ok(HotkeyMatchResult {
        success: would_trigger,
        hotkey_str: hotkey_str.to_string(),
        parsed_successfully: parsed_ok,
        parse_error,
        test_events,
        would_trigger,
        errors,
    })
}

/// Whether captured events include the modifiers (and optional main key) required by `target`.
fn events_satisfy_target(events: &[TestEvent], target: &RdevHotkey) -> bool {
    let mut saw_ctrl = false;
    let mut saw_alt = false;
    let mut saw_shift = false;
    let mut saw_meta = false;
    let mut saw_main = false;
    let main_label = target.main_key.map(|k| format!("{k:?}"));

    for e in events {
        if e.event_type != "KeyDown" {
            continue;
        }
        let k = e.key.as_str();
        if k.contains("Control") {
            saw_ctrl = true;
        }
        if k.contains("Alt") {
            saw_alt = true;
        }
        if k.contains("Shift") {
            saw_shift = true;
        }
        if k.contains("Meta") {
            saw_meta = true;
        }
        if let Some(ref ml) = main_label {
            if e.key == *ml {
                saw_main = true;
            }
        }
    }

    let mods_ok = (!target.ctrl || saw_ctrl)
        && (!target.alt || saw_alt)
        && (!target.shift || saw_shift)
        && (!target.meta || saw_meta);

    if target.main_key.is_some() {
        mods_ok && saw_main
    } else {
        mods_ok
    }
}
