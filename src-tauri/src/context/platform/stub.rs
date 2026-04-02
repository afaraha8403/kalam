use crate::config::ModeContextConfig;

use super::super::CapturedContext;

pub fn capture_selected_text_impl() -> Option<String> {
    None
}

pub fn clipboard_text_truncated_for_overlay(_max_chars: usize) -> Option<String> {
    None
}

pub fn capture_post_session_impl(
    _mode_ctx: &ModeContextConfig,
    _foreground_hwnd: Option<usize>,
) -> CapturedContext {
    CapturedContext::default()
}
