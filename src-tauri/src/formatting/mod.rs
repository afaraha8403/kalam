#![allow(dead_code)]

use crate::config::FormattingConfig;
use regex::Regex;

pub fn format_text(text: &str, config: &FormattingConfig) -> String {
    let mut result = text.to_string();

    // Auto-punctuation
    if config.auto_punctuation {
        result = add_punctuation(&result);
    }

    // Remove filler words
    if config.filler_word_removal {
        result = remove_filler_words(&result);
    }

    // Apply voice commands
    if config.voice_commands {
        result = apply_voice_commands(&result);
    }

    // Apply custom rules
    for rule in &config.custom_rules {
        if rule.enabled {
            if let Ok(regex) = Regex::new(&rule.pattern) {
                result = regex.replace_all(&result, &rule.replacement).to_string();
            }
        }
    }

    result.trim().to_string()
}

fn add_punctuation(text: &str) -> String {
    // Simple punctuation rules
    let mut result = text.to_string();

    // Add period if text looks like a sentence (starts with capital, no ending punctuation)
    if !result.ends_with(|c: char| c == '.' || c == '?' || c == '!') {
        // Check if it looks like a complete sentence
        if result
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            result.push('.');
        }
    }

    result
}

fn remove_filler_words(text: &str) -> String {
    let fillers = [
        r"\bum\b",
        r"\buh\b",
        r"\ber\b",
        r"\bah\b",
        r"\blike\b",
        r"\byou know\b",
        r"\bI mean\b",
        r"\bbasically\b",
        r"\bactually\b",
    ];

    let mut result = text.to_string();
    for filler in &fillers {
        if let Ok(regex) = Regex::new(filler) {
            result = regex.replace_all(&result, "").to_string();
        }
    }

    // Clean up extra spaces
    result = result.split_whitespace().collect::<Vec<_>>().join(" ");

    result
}

fn apply_voice_commands(text: &str) -> String {
    let commands: Vec<(&str, &str)> = vec![
        (r"\bperiod\b", "."),
        (r"\bfull stop\b", "."),
        (r"\bcomma\b", ","),
        (r"\bquestion mark\b", "?"),
        (r"\bexclamation mark\b", "!"),
        (r"\bexclamation point\b", "!"),
        (r"\bcolon\b", ":"),
        (r"\bsemicolon\b", ";"),
        (r"\bdash\b", "-"),
        (r"\bhyphen\b", "-"),
        (r"\bnew line\b", "\n"),
        (r"\bnew paragraph\b", "\n\n"),
        (r"\btab\b", "\t"),
    ];

    let mut result = text.to_string();
    for (pattern, replacement) in commands {
        if let Ok(regex) = Regex::new(pattern) {
            result = regex.replace_all(&result, replacement).to_string();
        }
    }

    result
}
