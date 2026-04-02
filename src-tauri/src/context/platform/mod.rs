#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{
    capture_post_session_impl, capture_selected_text_impl, clipboard_text_truncated_for_overlay,
};

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use stub::{
    capture_post_session_impl, capture_selected_text_impl, clipboard_text_truncated_for_overlay,
};
