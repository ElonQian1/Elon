//! Metering helpers for non-LLM compute that still spends server-side capacity.
//!
//! The existing accounting table is token-shaped, so audio/image/TTS entries use
//! conservative "compute units" stored as tokens. They are server-trusted and go
//! through the same quota and prepaid-balance deduction path as LLM usage.

use crate::{
    billing_lifecycle::TrustedBillingCall, cli_usage::CliTokenUsage, store::Store, token_usage_api,
};

pub(crate) const USAGE_MODE_METERED_COMPUTE: &str = "server_metered_compute";
pub(crate) const USAGE_MODE_VOICE_ASR: &str = "server_voice_asr";
pub(crate) const USAGE_MODE_VOICE_REALTIME: &str = "server_voice_realtime";
pub(crate) const USAGE_MODE_VOICE_TTS: &str = "server_voice_tts";

pub(crate) fn reserve_image_generation<'a>(
    store: &'a Store,
    user_id: &str,
    compute_call_id: &str,
    feature: &str,
    model: &str,
    prompt: &str,
) -> Result<TrustedBillingCall<'a>, String> {
    let metered_model = metered_image_model(model);
    let (input, output) = image_generation_units(prompt);
    reserve_units(
        store,
        user_id,
        compute_call_id,
        feature,
        USAGE_MODE_METERED_COMPUTE,
        Some(&metered_model),
        input,
        output,
        "billing_image_min_reservation_fen",
    )
}

pub(crate) fn record_image_generation_with_key(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: &str,
    prompt: &str,
    idempotency_key: Option<&str>,
) {
    let (input, output) = image_generation_units(prompt);
    record_units_with_key(
        store,
        user_id,
        feature,
        USAGE_MODE_METERED_COMPUTE,
        metered_image_model(model),
        input,
        output,
        idempotency_key,
    );
}

pub(crate) fn reserve_encoded_asr<'a>(
    store: &'a Store,
    user_id: &str,
    compute_call_id: &str,
    feature: &str,
    model: &str,
    audio_bytes: usize,
) -> Result<TrustedBillingCall<'a>, String> {
    let metered_model = metered_asr_model(model);
    reserve_units(
        store,
        user_id,
        compute_call_id,
        feature,
        USAGE_MODE_VOICE_ASR,
        Some(&metered_model),
        encoded_audio_units(audio_bytes),
        0,
        "billing_asr_min_reservation_fen",
    )
}

pub(crate) fn record_encoded_asr_with_key(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: &str,
    audio_bytes: usize,
    idempotency_key: Option<&str>,
) {
    record_units_with_key(
        store,
        user_id,
        feature,
        USAGE_MODE_VOICE_ASR,
        metered_asr_model(model),
        encoded_audio_units(audio_bytes),
        0,
        idempotency_key,
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
    record_pcm_asr_with_key(
        store,
        user_id,
        feature,
        model,
        pcm_bytes,
        sample_rate,
        channels,
        None,
    );
}

pub(crate) fn reserve_pcm_asr<'a>(
    store: &'a Store,
    user_id: &str,
    compute_call_id: &str,
    feature: &str,
    model: &str,
    pcm_bytes: usize,
    sample_rate: u32,
    channels: u16,
) -> Result<TrustedBillingCall<'a>, String> {
    let metered_model = metered_asr_model(model);
    reserve_units(
        store,
        user_id,
        compute_call_id,
        feature,
        USAGE_MODE_VOICE_ASR,
        Some(&metered_model),
        pcm_audio_units(pcm_bytes, sample_rate, channels),
        0,
        "billing_asr_min_reservation_fen",
    )
}

pub(crate) fn record_pcm_asr_with_key(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: &str,
    pcm_bytes: usize,
    sample_rate: u32,
    channels: u16,
    idempotency_key: Option<&str>,
) {
    record_units_with_key(
        store,
        user_id,
        feature,
        USAGE_MODE_VOICE_ASR,
        metered_asr_model(model),
        pcm_audio_units(pcm_bytes, sample_rate, channels),
        0,
        idempotency_key,
    );
}

pub(crate) fn record_realtime_voice_estimate_with_key(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: &str,
    input_pcm_bytes: usize,
    output_pcm_bytes: usize,
    sample_rate: u32,
    channels: u16,
    idempotency_key: Option<&str>,
) {
    let input = pcm_audio_units(input_pcm_bytes, sample_rate, channels);
    let output = pcm_audio_units(output_pcm_bytes, sample_rate, channels);
    record_units_with_key(
        store,
        user_id,
        feature,
        USAGE_MODE_VOICE_REALTIME,
        format!("metered-realtime:{model}"),
        input,
        output,
        idempotency_key,
    );
}

pub(crate) fn reserve_realtime_voice_turn<'a>(
    store: &'a Store,
    user_id: &str,
    compute_call_id: &str,
    feature: &str,
    model: &str,
) -> Result<TrustedBillingCall<'a>, String> {
    let metered_model = metered_realtime_model(model);
    reserve_units(
        store,
        user_id,
        compute_call_id,
        feature,
        USAGE_MODE_VOICE_REALTIME,
        Some(&metered_model),
        1,
        1,
        "billing_realtime_voice_min_reservation_fen",
    )
}

pub(crate) fn reserve_tts_synthesis<'a>(
    store: &'a Store,
    user_id: &str,
    compute_call_id: &str,
    feature: &str,
    provider: &str,
    spoken_text: &str,
) -> Result<TrustedBillingCall<'a>, String> {
    let metered_model = metered_tts_model(provider);
    let input = text_units(spoken_text, 2);
    let output = text_units(spoken_text, 8).max(1);
    reserve_units(
        store,
        user_id,
        compute_call_id,
        feature,
        USAGE_MODE_VOICE_TTS,
        Some(&metered_model),
        input,
        output,
        "billing_tts_min_reservation_fen",
    )
}

pub(crate) fn record_tts_synthesis_with_key(
    store: &Store,
    user_id: &str,
    feature: &str,
    provider: &str,
    spoken_text: &str,
    audio_bytes: usize,
    idempotency_key: Option<&str>,
) {
    let input = text_units(spoken_text, 2);
    let output = ceil_div(audio_bytes as i64, 2_048).max(1);
    record_units_with_key(
        store,
        user_id,
        feature,
        USAGE_MODE_VOICE_TTS,
        metered_tts_model(provider),
        input,
        output,
        idempotency_key,
    );
}

fn reserve_units<'a>(
    store: &'a Store,
    user_id: &str,
    compute_call_id: &str,
    feature: &str,
    usage_mode: &str,
    model: Option<&str>,
    input_tokens: i64,
    output_tokens: i64,
    min_config_key: &str,
) -> Result<TrustedBillingCall<'a>, String> {
    let model_for_cost = model.unwrap_or("metered-compute");
    let estimated = crate::billing::estimate_cost_for_tokens(
        store,
        model_for_cost,
        input_tokens.max(0),
        0,
        output_tokens.max(0),
    )
    .max(crate::billing::configured_reservation_fen(
        store,
        min_config_key,
        crate::billing::configured_reservation_fen(store, "billing_default_reservation_fen", 1),
    ));
    TrustedBillingCall::reserve(
        store,
        user_id,
        compute_call_id,
        feature,
        usage_mode,
        model,
        estimated,
    )
}

fn record_units_with_key(
    store: &Store,
    user_id: &str,
    feature: &str,
    usage_mode: &str,
    model: String,
    input_tokens: i64,
    output_tokens: i64,
    idempotency_key: Option<&str>,
) {
    let usage = CliTokenUsage {
        input_tokens: input_tokens.max(0),
        output_tokens: output_tokens.max(0),
        total_tokens: input_tokens.max(0) + output_tokens.max(0),
        model: Some(model.clone()),
        ..CliTokenUsage::default()
    };
    token_usage_api::record_trusted_usage_with_key(
        store,
        user_id,
        feature,
        usage_mode,
        Some(&model),
        &usage,
        idempotency_key,
    );
}

fn image_generation_units(prompt: &str) -> (i64, i64) {
    (text_units(prompt, 4), 1_500)
}

fn metered_image_model(model: &str) -> String {
    format!("metered-image:{model}")
}

fn metered_asr_model(model: &str) -> String {
    format!("metered-asr:{model}")
}

fn metered_realtime_model(model: &str) -> String {
    format!("metered-realtime:{model}")
}

fn metered_tts_model(provider: &str) -> String {
    format!("metered-tts:{provider}")
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
