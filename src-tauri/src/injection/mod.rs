#![allow(dead_code)]

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::time::Duration;
use tokio::time::sleep;

use crate::config::{FormattingConfig, InjectionMethod};

const PASTE_WAIT_MS: u64 = 100;

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
                if text.len() > config.clipboard_threshold {
                    InjectionMethod::Clipboard
                } else {
                    InjectionMethod::Keystrokes
                }
            }
            other => other.clone(),
        };

        let mut last_err = None;
        for attempt in 1..=config.retry_attempts {
            let result = match method {
                InjectionMethod::Keystrokes => {
                    self.inject_via_keystrokes(text, config.keystroke_delay_ms)
                        .await
                }
                InjectionMethod::Clipboard => self.inject_via_clipboard(text).await,
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
        {
            let mut enigo = Enigo::new(&Settings::default())
                .map_err(|e| anyhow::anyhow!("Failed to init enigo: {:?}", e))?;
            enigo
                .text(text)
                .map_err(|e| anyhow::anyhow!("Failed to type text: {:?}", e))?;
        }
        if delay_ms > 0 {
            sleep(Duration::from_millis(
                delay_ms.saturating_mul(text.len() as u64),
            ))
            .await;
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
        drop(enigo);

        sleep(Duration::from_millis(PASTE_WAIT_MS)).await;

        if let Err(e) = clipboard.set_text(&old_text) {
            log::warn!("Failed to restore clipboard after paste: {}", e);
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
