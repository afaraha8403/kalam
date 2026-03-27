#![allow(dead_code)]

use crate::config::{FormattingConfig, Snippet};
use regex::{Regex, RegexBuilder};

/// Actions to run before injecting text (e.g. undo, delete last).
#[derive(Debug, Clone)]
pub enum VoiceAction {
    Undo,
    DeleteLastChars(usize),
}

/// Format and expand text; returns (final text, actions to run before injection).
/// Pass last_injected_len and last_injected_text so "delete that" / "scratch that" can be resolved.
pub fn format_text(
    text: &str,
    config: &FormattingConfig,
    snippets: &[Snippet],
    last_injected_len: usize,
    last_injected_text: Option<&str>,
) -> (String, Vec<VoiceAction>) {
    let mut result = text.to_string();

    result = apply_snippets(&result, snippets);
    if config.auto_punctuation {
        result = add_punctuation(&result);
    }
    if config.filler_word_removal {
        result = remove_filler_words(&result);
    }

    let mut actions = Vec::new();
    if config.voice_commands {
        let (text_out, cmd_actions) =
            apply_voice_commands_with_actions(&result, last_injected_len, last_injected_text);
        result = text_out;
        actions = cmd_actions;
    }

    for rule in &config.custom_rules {
        if rule.enabled {
            if let Ok(regex) = Regex::new(&rule.pattern) {
                result = regex.replace_all(&result, &rule.replacement).to_string();
            }
        }
    }

    (result.trim().to_string(), actions)
}

/// Replace snippet triggers (e.g. @@email) with their expansions. Longer triggers are applied
/// first to avoid partial matches. Matching is case-insensitive; replaced text uses the expansion as-is.
fn apply_snippets(text: &str, snippets: &[Snippet]) -> String {
    if snippets.is_empty() {
        return text.to_string();
    }
    let mut sorted: Vec<&Snippet> = snippets.iter().collect();
    sorted.sort_by(|a, b| b.trigger.len().cmp(&a.trigger.len()));
    let mut result = text.to_string();
    for s in sorted {
        if s.trigger.is_empty() {
            continue;
        }
        let Ok(re) = RegexBuilder::new(&regex::escape(&s.trigger))
            .case_insensitive(true)
            .build()
        else {
            continue;
        };
        if re.is_match(&result) {
            result = re.replace_all(&result, s.expansion.as_str()).to_string();
        }
    }
    result
}

fn add_punctuation(text: &str) -> String {
    // Simple punctuation rules
    let mut result = text.to_string();

    // Add period if text looks like a sentence (starts with capital, no ending punctuation)
    if !result
        .chars()
        .last()
        .is_some_and(|c| ['.', '?', '!'].contains(&c))
    {
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

fn apply_voice_commands_with_actions(
    text: &str,
    last_injected_len: usize,
    last_injected_text: Option<&str>,
) -> (String, Vec<VoiceAction>) {
    let text_commands: Vec<(&str, &str)> = vec![
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
    for (pattern, replacement) in text_commands {
        if let Ok(regex) = Regex::new(pattern) {
            result = regex.replace_all(&result, replacement).to_string();
        }
    }

    let mut actions = Vec::new();

    // "undo" → Ctrl+Z / Cmd+Z
    if let Ok(r) = Regex::new(r"\bundo\b") {
        if r.is_match(&result) {
            actions.push(VoiceAction::Undo);
            result = r.replace_all(&result, "").to_string();
        }
    }

    // "delete that" → backspace last injected chunk
    if let Ok(r) = Regex::new(r"\bdelete\s+that\b") {
        if r.is_match(&result) && last_injected_len > 0 {
            actions.push(VoiceAction::DeleteLastChars(last_injected_len));
            result = r.replace_all(&result, "").to_string();
        }
    }

    // "scratch that" → backspace to last sentence (length of last sentence in previous injection)
    if let Ok(r) = Regex::new(r"\bscratch\s+that\b") {
        if r.is_match(&result) {
            let backspace_count = last_injected_text
                .and_then(|s| {
                    let trim = s.trim_end();
                    trim.rfind(['.', '!', '?']).map(|i| trim.len() - i - 1)
                })
                .unwrap_or(last_injected_len);
            if backspace_count > 0 {
                actions.push(VoiceAction::DeleteLastChars(backspace_count));
            }
            result = r.replace_all(&result, "").to_string();
        }
    }

    result = result.split_whitespace().collect::<Vec<_>>().join(" ");
    (result, actions)
}
