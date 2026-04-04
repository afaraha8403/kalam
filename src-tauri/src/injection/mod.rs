#![allow(dead_code)]

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::time::Duration;
use tokio::time::sleep;

use crate::config::{FormattingConfig, InjectionMethod};

/// How long to wait after sending Ctrl+V before restoring the clipboard.
/// Must be long enough for the target app to read the clipboard.
/// 100ms was too short for XAML/Electron/heavy apps — raised to 350ms
/// with a verification loop that re-checks the clipboard to confirm the
/// paste was consumed before restoring.
const PASTE_SETTLE_MS: u64 = 350;

/// Max number of extra polling rounds to wait for the target app to consume the paste.
const PASTE_VERIFY_ROUNDS: u32 = 3;

/// Interval between verification polls.
const PASTE_VERIFY_INTERVAL_MS: u64 = 100;

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }

    /// Inject text using default config (for backward compatibility).
    pub async fn inject(&self, text: &str) -> anyhow::Result<()> {
        self.inject_with_config(text, &FormattingConfig::default())
            .await
    }

    /// Inject text using the provided formatting config (method, threshold, retries).
    pub async fn inject_with_config(
        &self,
        text: &str,
        config: &FormattingConfig,
    ) -> anyhow::Result<()> {
        let method = match &config.injection_method {
            InjectionMethod::Auto => {
                #[cfg(target_os = "macos")]
                {
                    // Automatic on macOS: Accessibility API first, clipboard fallback in dispatch.
                    InjectionMethod::AccessibilityAPI
                }
                #[cfg(not(target_os = "macos"))]
                {
                    if text.len() > config.clipboard_threshold {
                        InjectionMethod::Clipboard
                    } else {
                        InjectionMethod::Keystrokes
                    }
                }
            }
            other => other.clone(),
        };

        log::debug!("Injecting {} chars via {:?}", text.len(), method);

        let mut last_err = None;
        for attempt in 1..=config.retry_attempts {
            let result = match method {
                InjectionMethod::Keystrokes => {
                    self.inject_via_keystrokes(text, config.keystroke_delay_ms)
                        .await
                }
                InjectionMethod::Clipboard => self.inject_via_clipboard(text).await,
                InjectionMethod::AccessibilityAPI => {
                    #[cfg(target_os = "macos")]
                    {
                        match self.inject_via_accessibility_api(text).await {
                            Ok(()) => Ok(()),
                            Err(ax_err) => {
                                log::info!(
                                    "AX API injection failed, falling back to clipboard: {}",
                                    ax_err
                                );
                                self.inject_via_clipboard(text).await
                            }
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        // Config may sync from macOS; paste is the portable fallback.
                        self.inject_via_clipboard(text).await
                    }
                }
                InjectionMethod::Auto => unreachable!(),
            };

            match result {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_err = Some(e);
                    if attempt < config.retry_attempts {
                        log::warn!(
                            "Injection attempt {} failed, retrying in {}ms: {}",
                            attempt,
                            config.retry_delay_ms,
                            last_err.as_ref().unwrap()
                        );
                        sleep(Duration::from_millis(config.retry_delay_ms)).await;
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Injection failed")))
    }

    async fn inject_via_keystrokes(&self, text: &str, delay_ms: u64) -> anyhow::Result<()> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Failed to init enigo: {:?}", e))?;

        if text.is_empty() {
            return Ok(());
        }

        // delay_ms == 0: one batch (fast). delay_ms > 0: pause between each Unicode scalar
        // so "ms per character" matches Settings copy and helps flaky targets.
        if delay_ms == 0 {
            enigo
                .text(text)
                .map_err(|e| anyhow::anyhow!("Failed to type text: {:?}", e))?;
            return Ok(());
        }

        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            let mut buf = [0u8; 4];
            let fragment = ch.encode_utf8(&mut buf);
            enigo
                .text(fragment)
                .map_err(|e| anyhow::anyhow!("Failed to type text: {:?}", e))?;
            if chars.peek().is_some() {
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
        Ok(())
    }

    async fn inject_via_clipboard(&self, text: &str) -> anyhow::Result<()> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| anyhow::anyhow!("Failed to open clipboard: {}", e))?;

        let old_text = clipboard.get_text().unwrap_or_default();

        clipboard
            .set_text(text)
            .map_err(|e| anyhow::anyhow!("Failed to set clipboard: {}", e))?;

        // Verify the clipboard actually holds our text before pasting.
        // Some apps or clipboard managers can race us.
        let clip_check = clipboard.get_text().unwrap_or_default();
        if clip_check != text {
            log::warn!(
                "Clipboard verification failed: set {} chars but read back {} chars",
                text.len(),
                clip_check.len()
            );
        }

        {
            let mut enigo = Enigo::new(&Settings::default())
                .map_err(|e| anyhow::anyhow!("Failed to init enigo: {:?}", e))?;

            #[cfg(target_os = "macos")]
            let modifier = Key::Meta;
            #[cfg(not(target_os = "macos"))]
            let modifier = Key::Control;

            enigo
                .key(modifier, Direction::Press)
                .map_err(|e| anyhow::anyhow!("Failed to press modifier: {:?}", e))?;
            enigo
                .key(Key::Unicode('v'), Direction::Click)
                .map_err(|e| anyhow::anyhow!("Failed to press v: {:?}", e))?;
            enigo
                .key(modifier, Direction::Release)
                .map_err(|e| anyhow::anyhow!("Failed to release modifier: {:?}", e))?;
        }

        // Wait for the target app to process the paste. The initial delay covers
        // the vast majority of apps; the verification loop handles slow ones
        // (XAML, Electron, apps under load) without penalising fast apps.
        sleep(Duration::from_millis(PASTE_SETTLE_MS)).await;

        // Before restoring, verify the target likely consumed the paste by
        // checking that the clipboard still holds our text.  If a clipboard
        // manager already changed it, skip restoration to avoid overwriting.
        let still_ours = clipboard.get_text().map(|c| c == text).unwrap_or(false);

        if still_ours {
            // Extra safety: poll briefly in case the app is still mid-paste.
            // We only restore once we're confident the app has had time to read.
            for round in 0..PASTE_VERIFY_ROUNDS {
                sleep(Duration::from_millis(PASTE_VERIFY_INTERVAL_MS)).await;
                let current = clipboard.get_text().unwrap_or_default();
                if current != text {
                    // Clipboard was modified externally (target app or manager consumed it).
                    log::debug!("Clipboard consumed by app after {} extra polls", round + 1);
                    break;
                }
            }
            if let Err(e) = clipboard.set_text(&old_text) {
                log::warn!("Failed to restore clipboard after paste: {}", e);
            }
        } else {
            log::debug!("Clipboard already changed after paste, skipping restore");
        }

        Ok(())
    }

    /// macOS: set `AXSelectedText` on the focused accessibility element (requires trusted accessibility client).
    #[cfg(target_os = "macos")]
    async fn inject_via_accessibility_api(&self, text: &str) -> anyhow::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let owned = text.to_string();
        tokio::task::spawn_blocking(move || macos_ax_inject_selected_text(&owned))
            .await
            .map_err(|e| anyhow::anyhow!("AX injection task join error: {}", e))?
    }

    /// Insert at caret / replace selection via Accessibility; clipboard is not used.
    #[cfg(target_os = "macos")]
    fn macos_ax_inject_selected_text(text: &str) -> anyhow::Result<()> {
        use core_foundation::base::TCFType;
        use core_foundation::string::CFString;
        use core_foundation_sys::base::{CFRelease, CFTypeRef};
        use std::ffi::c_void;
        use std::ptr;

        const KAX_ERROR_SUCCESS: i32 = 0;

        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXUIElementCreateSystemWide() -> *const c_void;
            fn AXUIElementCopyAttributeValue(
                element: *const c_void,
                attribute: core_foundation_sys::string::CFStringRef,
                value: *mut CFTypeRef,
            ) -> i32;
            fn AXUIElementSetAttributeValue(
                element: *const c_void,
                attribute: core_foundation_sys::string::CFStringRef,
                value: CFTypeRef,
            ) -> i32;
        }

        // CFString constants match `kAXFocusedUIElementAttribute` / `kAXSelectedTextAttribute`.
        let attr_focused = CFString::new("AXFocusedUIElement");
        let attr_selected = CFString::new("AXSelectedText");
        let text_cf = CFString::new(text);

        unsafe {
            let system_wide = AXUIElementCreateSystemWide();
            if system_wide.is_null() {
                return Err(anyhow::anyhow!("AXUIElementCreateSystemWide returned null"));
            }

            let mut focused_raw: CFTypeRef = ptr::null();
            let copy_err = AXUIElementCopyAttributeValue(
                system_wide,
                attr_focused.as_concrete_TypeRef(),
                &mut focused_raw,
            );
            CFRelease(system_wide as CFTypeRef);

            if copy_err != KAX_ERROR_SUCCESS {
                return Err(anyhow::anyhow!(
                    "AXUIElementCopyAttributeValue(AXFocusedUIElement) failed: AXError {}",
                    copy_err
                ));
            }
            if focused_raw.is_null() {
                return Err(anyhow::anyhow!("No focused accessibility element"));
            }

            let set_err = AXUIElementSetAttributeValue(
                focused_raw as *const c_void,
                attr_selected.as_concrete_TypeRef(),
                text_cf.as_CFTypeRef(),
            );
            CFRelease(focused_raw);

            if set_err != KAX_ERROR_SUCCESS {
                return Err(anyhow::anyhow!(
                    "AXUIElementSetAttributeValue(AXSelectedText) failed: AXError {}",
                    set_err
                ));
            }
        }

        Ok(())
    }

    /// Send Ctrl+Z / Cmd+Z (undo).
    pub fn inject_undo(&self) -> anyhow::Result<()> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Failed to init enigo: {:?}", e))?;
        #[cfg(target_os = "macos")]
        let modifier = Key::Meta;
        #[cfg(not(target_os = "macos"))]
        let modifier = Key::Control;
        enigo
            .key(modifier, Direction::Press)
            .map_err(|e| anyhow::anyhow!("enigo: {:?}", e))?;
        enigo
            .key(Key::Unicode('z'), Direction::Click)
            .map_err(|e| anyhow::anyhow!("enigo: {:?}", e))?;
        enigo
            .key(modifier, Direction::Release)
            .map_err(|e| anyhow::anyhow!("enigo: {:?}", e))?;
        Ok(())
    }

    /// Send N backspace key presses.
    pub fn inject_backspaces(&self, n: usize) -> anyhow::Result<()> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Failed to init enigo: {:?}", e))?;
        for _ in 0..n {
            enigo
                .key(Key::Backspace, Direction::Click)
                .map_err(|e| anyhow::anyhow!("enigo: {:?}", e))?;
        }
        Ok(())
    }
}
