#![allow(dead_code)]

use std::time::Duration;
use tokio::time::sleep;
use enigo::{Enigo, Keyboard, Settings};

use crate::config::{FormattingConfig, InjectionMethod};

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub async fn inject(&self, text: &str) -> anyhow::Result<()> {
        let config = FormattingConfig::default(); // In production, get from config
        
        let method = match config.injection_method {
            InjectionMethod::Auto => {
                if text.len() > config.clipboard_threshold {
                    InjectionMethod::Clipboard
                } else {
                    InjectionMethod::Keystrokes
                }
            }
            other => other,
        };

        match method {
            InjectionMethod::Keystrokes => self.inject_via_keystrokes(text, config.keystroke_delay_ms).await,
            InjectionMethod::Clipboard => self.inject_via_clipboard(text).await,
            _ => unreachable!(),
        }
    }

    async fn inject_via_keystrokes(&self, text: &str, delay_ms: u64) -> anyhow::Result<()> {
        log::info!("Typing via enigo: {}", text);
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow::anyhow!("Failed to init enigo: {:?}", e))?;
        enigo.text(text).map_err(|e| anyhow::anyhow!("Failed to type text: {:?}", e))?;
        if delay_ms > 0 {
            sleep(Duration::from_millis(delay_ms * text.len() as u64)).await;
        }
        Ok(())
    }

    async fn inject_via_clipboard(&self, text: &str) -> anyhow::Result<()> {
        // Stub implementation - would use clipboard in production
        log::info!("Would paste: {}", text);
        sleep(Duration::from_millis(100)).await;
        Ok(())
    }
}
