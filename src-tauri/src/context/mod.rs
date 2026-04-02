//! Screen / clipboard context for Phase 4 LLM prompts (Pro-gated at call sites when billing ships).
//!
//! Selection is captured at hotkey time (see `capture_selected_text`); app + clipboard + system
//! are captured after recording so they match the post-dictation state.

mod platform;

use crate::config::ModeContextConfig;
use serde::{Deserialize, Serialize};

/// Max chars sent to the LLM per context bucket (avoid huge prompts / token waste).
pub const MAX_APP_TEXT_CHARS: usize = 2000;
pub const MAX_CLIPBOARD_CHARS: usize = 1000;
pub const MAX_SELECTION_CHARS: usize = 2000;

#[derive(Debug, Default, Clone)]
pub struct CapturedContext {
    /// Friendly label (e.g. window title or process basename).
    pub app_name: Option<String>,
    pub app_text: Option<String>,
    pub selected_text: Option<String>,
    pub clipboard_text: Option<String>,
    pub system_info: Option<String>,
}

/// `Ctrl+C` shim while the target app still has focus — run from a blocking pool thread, not the UI thread.
pub fn capture_selected_text() -> Option<String> {
    platform::capture_selected_text_impl()
}

/// Read app / clipboard / system per `ModeContextConfig` (caller must enforce global toggle + sensitive-app off).
pub fn capture_context_post_session(
    mode_ctx: &ModeContextConfig,
    foreground_hwnd: Option<usize>,
) -> CapturedContext {
    if !mode_ctx.enabled {
        return CapturedContext::default();
    }
    platform::capture_post_session_impl(mode_ctx, foreground_hwnd)
}

/// Payload for `get_context_previews` (Phase 6 full overlay). Fetched on demand so `overlay-state` stays small.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPreviewsForOverlay {
    pub app_label: Option<String>,
    pub clipboard_preview: Option<String>,
    /// Avoids Ctrl+C from the overlay webview; filled when session already captured selection.
    pub selection_preview: Option<String>,
}

/// Best-effort foreground app + clipboard for the full overlay panel.
pub fn context_previews_for_overlay() -> ContextPreviewsForOverlay {
    let app_label = crate::config::privacy::get_foreground_app().map(|(proc, title)| {
        let t = title.trim();
        if !t.is_empty() {
            format!("{} — {}", proc, t)
        } else {
            proc
        }
    });
    let clipboard_preview = platform::clipboard_text_truncated_for_overlay(100);
    ContextPreviewsForOverlay {
        app_label,
        clipboard_preview,
        selection_preview: None,
    }
}

pub fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect::<String>() + "…"
}

fn nonempty_trim(opt: &Option<String>) -> bool {
    opt.as_ref().is_some_and(|s| !s.trim().is_empty())
}

/// App / clipboard / system blocks for the **system** prompt (selection is user-message only).
pub fn context_has_system_prompt_extras(ctx: &CapturedContext) -> bool {
    nonempty_trim(&ctx.app_name)
        || nonempty_trim(&ctx.app_text)
        || nonempty_trim(&ctx.clipboard_text)
        || nonempty_trim(&ctx.system_info)
}

/// Any context that requires the LLM path (system extras and/or highlighted selection).
pub fn context_has_llm_payload(ctx: &CapturedContext) -> bool {
    context_has_system_prompt_extras(ctx) || nonempty_trim(&ctx.selected_text)
}

/// Extra system instructions when context is present (keeps mode/polish instructions unchanged).
pub fn context_system_section(ctx: &CapturedContext) -> Option<String> {
    if !context_has_system_prompt_extras(ctx) {
        return None;
    }
    let mut parts: Vec<String> = vec!["## Context from the user's environment".to_string()];
    if let Some(ref name) = ctx.app_name {
        if !name.trim().is_empty() {
            parts.push(format!("Active app / window: {}", name.trim()));
        }
    }
    if let Some(ref t) = ctx.app_text {
        if !t.trim().is_empty() {
            parts.push(format!(
                "App content (excerpt):\n\"\"\"\n{}\n\"\"\"",
                t.trim()
            ));
        }
    }
    if let Some(ref t) = ctx.clipboard_text {
        if !t.trim().is_empty() {
            parts.push(format!("Clipboard:\n\"\"\"\n{}\n\"\"\"", t.trim()));
        }
    }
    // Selection is attached to the user message (transcript) in `lib.rs`, not here.
    if let Some(ref s) = ctx.system_info {
        if !s.trim().is_empty() {
            parts.push(format!("System: {}", s.trim()));
        }
    }
    parts.push(
        "Use this context to make the output more relevant and accurate. Do not invent facts."
            .to_string(),
    );
    parts.push(
        "Do not repeat long context verbatim unless the user's speech clearly references it."
            .to_string(),
    );
    Some(parts.join("\n\n"))
}
