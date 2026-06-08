//! Metering helpers for non-LLM compute that still spends server-side capacity.
//!
//! The existing accounting table is token-shaped, so audio/image/TTS entries use
//! conservative "compute units" stored as tokens. They are server-trusted and go
//! through the same quota and prepaid-balance deduction path as LLM usage.

use crate::{cli_usage::CliTokenUsage, store::Store, token_usage_api};

pub(crate) const USAGE_MODE_METERED_COMPUTE: &str = "server_metered_compute";
pub(crate) const USAGE_MODE_VOICE_ASR: &str = "server_voice_asr";
pub(crate) const USAGE_MODE_VOICE_REALTIME: &str = "server_voice_realtime";
pub(crate) const USAGE_MODE_VOICE_TTS: &str = "server_voice_tts";

pub(crate) fn record_image_generation(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: &str,
    prompt: &str,
) {
    let input = text_units(prompt, 4);
    let output = 1_500;
    record_units(
        store,
        user_id,
        feature,
        USAGE_MODE_METERED_COMPUTE,
        format!("metered-image:{model}"),
        input,
        output,
    );
}

pub(crate) fn record_encoded_asr(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: &str,
    audio_bytes: usize,
) {
    record_units(
        store,
        user_id,
        feature,
        USAGE_MODE_VOICE_ASR,
        format!("metered-asr:{model}"),
        encoded_audio_units(audio_bytes),
        0,
    );
}

pub(crate) fn record_pcm_asr(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: &str,
    pcm_bytes: usize,
    sample_rate: u32,
    channels: u16,
) {
    record_units(
        store,
        user_id,
        feature,
        USAGE_MODE_VOICE_ASR,
        format!("metered-asr:{model}"),
        pcm_audio_units(pcm_bytes, sample_rate, channels),
        0,
    );
}

pub(crate) fn record_realtime_voice_estimate(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: &str,
    input_pcm_bytes: usize,
    output_pcm_bytes: usize,
    sample_rate: u32,
    channels: u16,
) {
    let input = pcm_audio_units(input_pcm_bytes, sample_rate, channels);
    let output = pcm_audio_units(output_pcm_bytes, sample_rate, channels);
    record_units(
        store,
        user_id,
        feature,
        USAGE_MODE_VOICE_REALTIME,
        format!("metered-realtime:{model}"),
        input,
        output,
    );
}

pub(crate) fn record_tts_synthesis(
    store: &Store,
    user_id: &str,
    feature: &str,
    provider: &str,
    spoken_text: &str,
    audio_bytes: usize,
) {
    let input = text_units(spoken_text, 2);
    let output = ceil_div(audio_bytes as i64, 2_048).max(1);
    record_units(
        store,
        user_id,
        feature,
        USAGE_MODE_VOICE_TTS,
        format!("metered-tts:{provider}"),
        input,
        output,
    );
}

fn record_units(
    store: &Store,
    user_id: &str,
    feature: &str,
    usage_mode: &str,
    model: String,
    input_tokens: i64,
    output_tokens: i64,
) {
    let usage = CliTokenUsage {
        input_tokens: input_tokens.max(0),
        output_tokens: output_tokens.max(0),
        total_tokens: input_tokens.max(0) + output_tokens.max(0),
        model: Some(model.clone()),
        ..CliTokenUsage::default()
    };
    token_usage_api::record_trusted_usage(
        store,
        user_id,
        feature,
        usage_mode,
        Some(&model),
        &usage,
    );
}

fn text_units(text: &str, chars_per_unit: i64) -> i64 {
    ceil_div(text.chars().count() as i64, chars_per_unit.max(1)).max(1)
}

fn encoded_audio_units(audio_bytes: usize) -> i64 {
    ceil_div(audio_bytes as i64, 1_024).max(1)
}

pub(crate) fn pcm_audio_units(pcm_bytes: usize, sample_rate: u32, channels: u16) -> i64 {
    let bytes_per_second = sample_rate.max(1) as i64
        * channels.max(1) as i64
        * crate::voice_config::PCM16_BYTES_PER_SAMPLE as i64;
    let millis = ceil_div(pcm_bytes as i64 * 1_000, bytes_per_second);
    ceil_div(millis * 50, 1_000).max(1)
}

fn ceil_div(n: i64, d: i64) -> i64 {
    if n <= 0 {
        0
    } else {
        (n + d.max(1) - 1) / d.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_audio_units_are_duration_based() {
        let one_second = 24_000 * 2;
        assert_eq!(pcm_audio_units(one_second, 24_000, 1), 50);
        assert_eq!(pcm_audio_units(one_second / 2, 24_000, 1), 25);
    }

    #[test]
    fn encoded_audio_units_have_minimum() {
        assert_eq!(encoded_audio_units(1), 1);
        assert_eq!(encoded_audio_units(2_049), 3);
    }
}
