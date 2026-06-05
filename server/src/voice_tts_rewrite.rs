//! TTS 前置文本整理与台词化。
//!
//! 目标是让普通 AI 回复更适合朗读：去掉 Markdown/代码噪声，压短句子，
//! 再按情绪轻量调整停顿。真正的 LLM 改写由 API 层按配置调用。

use crate::voice_tts_catalog::ResolvedTtsStyle;

pub const MAX_TTS_TEXT_CHARS: usize = 800;

pub fn prepare_text_for_speech(text: &str, style: &ResolvedTtsStyle) -> String {
    let compact = strip_markdown_noise(text);
    let limited = take_chars(&compact, MAX_TTS_TEXT_CHARS);
    apply_style(&limited, style)
}

pub fn clean_llm_rewrite(value: &str, fallback: &str) -> String {
    let cleaned = value
        .trim()
        .trim_matches('"')
        .trim_matches('“')
        .trim_matches('”')
        .trim()
        .to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        take_chars(&cleaned, MAX_TTS_TEXT_CHARS)
    }
}

pub fn take_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect::<String>()
}

fn strip_markdown_noise(text: &str) -> String {
    let mut out = String::new();
    let mut in_code_block = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        if line.starts_with("http://") || line.starts_with("https://") {
            continue;
        }
        let cleaned = line
            .trim_start_matches(|ch| matches!(ch, '-' | '*' | '>' | '#'))
            .trim()
            .replace('`', "")
            .replace("**", "")
            .replace("__", "");
        if cleaned.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&cleaned);
    }
    normalize_spaces(&out)
}

fn apply_style(text: &str, style: &ResolvedTtsStyle) -> String {
    let base = normalize_spaces(text);
    match style.emotion.id {
        "wronged_crying" | "crying_broken" => soften_with_ellipsis(&base),
        "gentle_comfort" | "whisper_low" => soften_punctuation(&base),
        "excited_burst" | "surprised_excited" => brighten_punctuation(&base),
        "cool_detached" | "angry_repressed" => shorten_pauses(&base),
        _ => base,
    }
}

fn soften_with_ellipsis(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '。' | '；' | ';' => out.push_str("……"),
            '！' | '!' => out.push('。'),
            _ => out.push(ch),
        }
    }
    normalize_repeated_punctuation(&out)
}

fn soften_punctuation(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '！' | '!' => out.push('。'),
            '。' => out.push_str("。"),
            _ => out.push(ch),
        }
    }
    out
}

fn brighten_punctuation(text: &str) -> String {
    if text.contains('！') || text.contains('!') {
        normalize_repeated_punctuation(text)
    } else {
        format!("{}！", text.trim_end_matches('。'))
    }
}

fn shorten_pauses(text: &str) -> String {
    text.replace("……", "。").replace('，', "。")
}

fn normalize_spaces(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_repeated_punctuation(value: &str) -> String {
    value
        .replace("…………", "……")
        .replace("！！！", "！！")
        .replace("!!!", "!!")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice_tts_catalog::resolve_style;

    #[test]
    fn strips_code_blocks_before_speech() {
        let style = resolve_style(None, Some("normal"), Some("normal"), None, "");
        let text = "你好\n```kotlin\nprintln(1)\n```\n继续。";
        assert_eq!(prepare_text_for_speech(text, &style), "你好 继续。");
    }

    #[test]
    fn sad_style_adds_soft_pauses() {
        let style = resolve_style(None, Some("wronged_crying"), Some("immersive"), None, "");
        let text = prepare_text_for_speech("你一直没有回我。其实我等了很久。", &style);
        assert!(text.contains("……"));
    }
}
