//! macOS-only: tune overlay `NSWindow` so it behaves like a passive utility surface (less focus churn).
//! `raw-window-handle` gives an `NSView`; we resolve `window` from the view (Tauri/Wry pattern).

use objc::runtime::Object;
use objc::{msg_send, sel, sel_impl};

/// `ns_view` is the pointer from [`raw_window_handle::AppKitWindowHandle::ns_view`].
pub fn configure_overlay_ns_window(ns_view: *mut std::ffi::c_void) {
    if ns_view.is_null() {
        return;
    }
    let view = ns_view.cast::<Object>();
    unsafe {
        let window: *mut Object = msg_send![view, window];
        if window.is_null() {
            return;
        }
        let _: () = msg_send![window, setHidesOnDeactivate: true];
        let _: () = msg_send![window, setWorksWhenModal: true];
    }
}
