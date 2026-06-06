//! 女声情绪 TTS 的产品级预设目录。
//!
//! 这里不绑定具体模型进程，只描述“角色、情绪、强度”和默认路由策略。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsVoicePreset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub prompt_audio: &'static str,
    pub role_prompt: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsEmotionPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub emotion_audio: &'static str,
    pub text_style: &'static str,
    pub base_emo_alpha: f32,
    pub speed: f32,
    pub pause_style: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsIntensityPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub alpha_multiplier: f32,
    pub provider_hint: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TtsProvider {
    Auto,
    IndexTts2,
    CosyVoice3,
    GptSoVits,
}

impl TtsProvider {
    pub fn as_worker_id(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::IndexTts2 => "index_tts2",
            Self::CosyVoice3 => "cosyvoice3",
            Self::GptSoVits => "gpt_sovits",
        }
    }

    pub fn from_env_value(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "index_tts2" | "indextts2" | "index-tts2" => Self::IndexTts2,
            "cosyvoice3" | "cosy_voice3" | "cosyvoice" => Self::CosyVoice3,
            "gpt_sovits" | "gpt-sovits" | "gptsovits" => Self::GptSoVits,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedTtsStyle {
    pub voice: TtsVoicePreset,
    pub emotion: TtsEmotionPreset,
    pub intensity: TtsIntensityPreset,
    pub provider: TtsProvider,
    pub emo_alpha: f32,
}

pub fn voices() -> Vec<TtsVoicePreset> {
    vec![
        TtsVoicePreset {
            id: "female_warm",
            label: "温柔姐姐",
            description: "温柔陪伴型，适合聊天、安慰和日常回复",
            prompt_audio: "voices/female_warm_neutral.wav",
            role_prompt: "温柔、亲近、稳定，有陪伴感的年轻女声",
        },
        TtsVoicePreset {
            id: "female_bright",
            label: "元气女友",
            description: "活泼元气型，适合开心、鼓励和轻松互动",
            prompt_audio: "voices/female_bright_neutral.wav",
            role_prompt: "明亮、轻快、亲切，有明显笑意的年轻女声",
        },
        TtsVoicePreset {
            id: "female_mature",
            label: "成熟秘书",
            description: "成熟知性型，适合解释、总结和认真建议",
            prompt_audio: "voices/female_mature_neutral.wav",
            role_prompt: "成熟、清晰、知性，语气可靠的女性声线",
        },
        TtsVoicePreset {
            id: "female_cool",
            label: "冷淡女王",
            description: "冷淡疏离型，适合克制、压抑和低情绪表达",
            prompt_audio: "voices/female_cool_neutral.wav",
            role_prompt: "冷静、疏离、克制，声线偏低的女性声线",
        },
        TtsVoicePreset {
            id: "female_sweet",
            label: "甜美陪伴",
            description: "甜美可爱型，适合撒娇、害羞和轻剧情",
            prompt_audio: "voices/female_sweet_neutral.wav",
            role_prompt: "甜美、柔软、可爱，亲密陪伴感强的年轻女声",
        },
    ]
}

pub fn emotions() -> Vec<TtsEmotionPreset> {
    vec![
        TtsEmotionPreset {
            id: "normal",
            label: "正常",
            description: "轻情绪、自然聊天",
            emotion_audio: "emotions/female_neutral.wav",
            text_style: "自然口语，少量停顿，保留原意",
            base_emo_alpha: 0.35,
            speed: 1.0,
            pause_style: "balanced",
        },
        TtsEmotionPreset {
            id: "gentle_comfort",
            label: "温柔安慰",
            description: "慢一点、轻一点，像在安慰对方",
            emotion_audio: "emotions/female_gentle_comfort.wav",
            text_style: "句子柔和，短句，适度停顿",
            base_emo_alpha: 0.55,
            speed: 0.94,
            pause_style: "soft",
        },
        TtsEmotionPreset {
            id: "wronged_crying",
            label: "委屈快哭",
            description: "委屈、脆弱、接近哭腔",
            emotion_audio: "emotions/female_crying_broken.wav",
            text_style: "短句，多停顿，省略号，轻微重复",
            base_emo_alpha: 0.72,
            speed: 0.9,
            pause_style: "broken",
        },
        TtsEmotionPreset {
            id: "happy_sweet",
            label: "开心撒娇",
            description: "开心、甜美、有亲近感",
            emotion_audio: "emotions/female_happy_soft.wav",
            text_style: "短句，带笑意，轻快但不过度夸张",
            base_emo_alpha: 0.62,
            speed: 1.06,
            pause_style: "light",
        },
        TtsEmotionPreset {
            id: "excited_burst",
            label: "兴奋爆发",
            description: "节奏快、情绪高、适合惊喜反馈",
            emotion_audio: "emotions/female_happy_excited.wav",
            text_style: "短句，感叹，节奏快",
            base_emo_alpha: 0.84,
            speed: 1.12,
            pause_style: "fast",
        },
        TtsEmotionPreset {
            id: "angry_repressed",
            label: "压抑生气",
            description: "克制的不满，不做吼叫",
            emotion_audio: "emotions/female_angry_repressed.wav",
            text_style: "句子短，语气压低，停顿干净",
            base_emo_alpha: 0.68,
            speed: 0.96,
            pause_style: "clean",
        },
        TtsEmotionPreset {
            id: "cool_detached",
            label: "冷淡疏离",
            description: "少情绪词、淡淡的距离感",
            emotion_audio: "emotions/female_cool_detached.wav",
            text_style: "短句，少修饰，停顿克制",
            base_emo_alpha: 0.45,
            speed: 0.98,
            pause_style: "clean",
        },
        TtsEmotionPreset {
            id: "shy_nervous",
            label: "害羞紧张",
            description: "轻微犹豫、紧张、亲密感",
            emotion_audio: "emotions/female_shy_nervous.wav",
            text_style: "短句，轻微停顿，避免过度卖萌",
            base_emo_alpha: 0.58,
            speed: 0.96,
            pause_style: "hesitant",
        },
        TtsEmotionPreset {
            id: "sad_low",
            label: "失落低落",
            description: "低落、心疼、语速稍慢",
            emotion_audio: "emotions/female_sad_low.wav",
            text_style: "句子更短，停顿更长，语气低",
            base_emo_alpha: 0.6,
            speed: 0.9,
            pause_style: "slow",
        },
        TtsEmotionPreset {
            id: "surprised_excited",
            label: "惊喜激动",
            description: "惊讶后转开心，适合成功提示",
            emotion_audio: "emotions/female_surprised.wav",
            text_style: "前短后扬，保留惊喜感",
            base_emo_alpha: 0.7,
            speed: 1.08,
            pause_style: "light",
        },
        TtsEmotionPreset {
            id: "crying_broken",
            label: "崩溃哭腔",
            description: "强剧情哭腔，只适合关键句",
            emotion_audio: "emotions/female_crying_broken.wav",
            text_style: "极短句，多省略号，情绪断裂",
            base_emo_alpha: 0.9,
            speed: 0.84,
            pause_style: "broken",
        },
        TtsEmotionPreset {
            id: "serious_encourage",
            label: "认真鼓励",
            description: "认真、坚定、适合任务建议",
            emotion_audio: "emotions/female_serious_encourage.wav",
            text_style: "清楚、有力量，停顿稳定",
            base_emo_alpha: 0.48,
            speed: 0.98,
            pause_style: "balanced",
        },
        TtsEmotionPreset {
            id: "whisper_low",
            label: "低声耳语",
            description: "更轻、更近，适合低声提示",
            emotion_audio: "emotions/female_whisper.wav",
            text_style: "短句，轻声，停顿柔和",
            base_emo_alpha: 0.58,
            speed: 0.88,
            pause_style: "soft",
        },
    ]
}

pub fn intensities() -> Vec<TtsIntensityPreset> {
    vec![
        TtsIntensityPreset {
            id: "normal",
            label: "普通模式",
            description: "情绪强度 0.3-0.5，适合长时间聊天",
            alpha_multiplier: 0.75,
            provider_hint: "cosyvoice3",
        },
        TtsIntensityPreset {
            id: "immersive",
            label: "沉浸模式",
            description: "情绪强度 0.55-0.75，适合角色感回复",
            alpha_multiplier: 1.0,
            provider_hint: "index_tts2",
        },
        TtsIntensityPreset {
            id: "dramatic",
            label: "剧情爆发",
            description: "情绪强度 0.8-0.95，只适合关键句",
            alpha_multiplier: 1.18,
            provider_hint: "index_tts2",
        },
    ]
}

pub fn resolve_style(
    voice_id: Option<&str>,
    emotion_id: Option<&str>,
    intensity_id: Option<&str>,
    requested_provider: Option<TtsProvider>,
    text: &str,
) -> ResolvedTtsStyle {
    let voice = find_voice(voice_id).unwrap_or_else(|| {
        voices()
            .into_iter()
            .next()
            .expect("TTS voice catalog must not be empty")
    });
    let inferred_emotion = emotion_id
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| infer_emotion_id(text).to_string());
    let emotion = find_emotion(Some(&inferred_emotion)).unwrap_or_else(|| {
        emotions()
            .into_iter()
            .next()
            .expect("TTS emotion catalog must not be empty")
    });
    let intensity = find_intensity(intensity_id).unwrap_or_else(|| {
        intensities()
            .into_iter()
            .next()
            .expect("TTS intensity catalog must not be empty")
    });
    let provider = choose_provider(requested_provider, &emotion, &intensity);
    let emo_alpha = (emotion.base_emo_alpha * intensity.alpha_multiplier).clamp(0.25, 0.95);

    ResolvedTtsStyle {
        voice,
        emotion,
        intensity,
        provider,
        emo_alpha,
    }
}

fn choose_provider(
    requested: Option<TtsProvider>,
    emotion: &TtsEmotionPreset,
    intensity: &TtsIntensityPreset,
) -> TtsProvider {
    match requested.unwrap_or(TtsProvider::Auto) {
        TtsProvider::Auto => {
            if intensity.id == "normal" && matches!(emotion.id, "normal" | "serious_encourage") {
                TtsProvider::CosyVoice3
            } else {
                TtsProvider::IndexTts2
            }
        }
        provider => provider,
    }
}

fn find_voice(id: Option<&str>) -> Option<TtsVoicePreset> {
    let target = id?.trim();
    voices().into_iter().find(|item| item.id == target)
}

fn find_emotion(id: Option<&str>) -> Option<TtsEmotionPreset> {
    let target = id?.trim();
    emotions().into_iter().find(|item| item.id == target)
}

fn find_intensity(id: Option<&str>) -> Option<TtsIntensityPreset> {
    let target = id?.trim();
    intensities().into_iter().find(|item| item.id == target)
}

pub fn infer_emotion_id(text: &str) -> &'static str {
    let content = text.trim();
    if content.is_empty() {
        return "normal";
    }
    let exclamations = content
        .chars()
        .filter(|ch| matches!(ch, '!' | '！'))
        .count();
    if has_any(content, &["低声", "小声", "悄悄", "耳语", "轻声"]) {
        "whisper_low"
    } else if has_any(content, &["崩溃", "再也", "哭腔", "撑不住"]) {
        "crying_broken"
    } else if has_any(content, &["委屈", "想哭", "心疼", "对不起", "抱歉"]) {
        "wronged_crying"
    } else if has_any(
        content,
        &["别怕", "我在", "没关系", "放心", "慢慢来", "陪着你"],
    ) {
        "gentle_comfort"
    } else if has_any(content, &["害羞", "紧张", "不好意思"]) {
        "shy_nervous"
    } else if has_any(content, &["难过", "失落", "遗憾", "可惜"]) {
        "sad_low"
    } else if has_any(
        content,
        &["风险", "错误", "失败", "不要", "不能", "需要检查"],
    ) {
        "serious_encourage"
    } else if exclamations >= 2 || has_any(content, &["太好了", "惊喜", "恭喜"]) {
        "surprised_excited"
    } else if has_any(content, &["开心", "好呀", "当然可以", "哈哈", "真棒"]) {
        "happy_sweet"
    } else {
        "normal"
    }
}

fn has_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| text.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_exposes_five_voices_and_emotional_presets() {
        assert_eq!(voices().len(), 5);
        assert!(emotions().iter().any(|item| item.id == "crying_broken"));
        assert_eq!(intensities().len(), 3);
    }

    #[test]
    fn voice_catalog_uses_distinct_speaker_prompts() {
        let voices = voices();
        let mut prompts = std::collections::HashSet::new();
        for voice in voices {
            assert!(
                prompts.insert(voice.prompt_audio),
                "duplicate prompt_audio for voice {}",
                voice.id
            );
        }
    }

    #[test]
    fn strong_emotions_route_to_index_by_default() {
        let style = resolve_style(None, Some("wronged_crying"), Some("immersive"), None, "");
        assert_eq!(style.provider, TtsProvider::IndexTts2);
    }

    #[test]
    fn normal_serious_chat_routes_to_cosyvoice() {
        let style = resolve_style(None, Some("normal"), Some("normal"), None, "");
        assert_eq!(style.provider, TtsProvider::CosyVoice3);
    }
}
