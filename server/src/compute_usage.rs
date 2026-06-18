//! Metering helpers for server-side compute that should consume prepaid balance.
//!
//! The existing accounting table is token-shaped, so image and AI realtime voice
//! entries use conservative "compute units" stored as tokens. ASR and TTS are
//! chat transport capabilities and are intentionally not metered here.

use crate::{
    billing_lifecycle::TrustedBillingCall,
    cli_usage::CliTokenUsage,
    store::{ComputeMeterEvent, Store, TokenUsageAccountingResult},
    token_usage_api,
};

pub(crate) const USAGE_MODE_METERED_COMPUTE: &str = "server_metered_compute";
pub(crate) const USAGE_MODE_VOICE_REALTIME: &str = "server_voice_realtime";

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
    let units = MeterUnits {
        source: "estimated_image",
        input_unit_kind: "prompt_char",
        output_unit_kind: "image",
        input_units: prompt.chars().count() as i64,
        output_units: 1,
    };
    record_units_with_key(
        store,
        user_id,
        feature,
        USAGE_MODE_METERED_COMPUTE,
        metered_image_model(model),
        input,
        output,
        idempotency_key,
        units,
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
    let units = MeterUnits {
        source: "estimated_realtime_audio",
        input_unit_kind: "audio_ms",
        output_unit_kind: "audio_ms",
        input_units: pcm_audio_millis(input_pcm_bytes, sample_rate, channels),
        output_units: pcm_audio_millis(output_pcm_bytes, sample_rate, channels),
    };
    record_units_with_key(
        store,
        user_id,
        feature,
        USAGE_MODE_VOICE_REALTIME,
        format!("metered-realtime:{model}"),
        input,
        output,
        idempotency_key,
        units,
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

struct MeterUnits<'a> {
    source: &'a str,
    input_unit_kind: &'a str,
    output_unit_kind: &'a str,
    input_units: i64,
    output_units: i64,
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
    units: MeterUnits<'_>,
) {
    let usage = CliTokenUsage {
        input_tokens: input_tokens.max(0),
        output_tokens: output_tokens.max(0),
        total_tokens: input_tokens.max(0) + output_tokens.max(0),
        model: Some(model.clone()),
        ..CliTokenUsage::default()
    };
    let accounting = token_usage_api::record_trusted_usage_with_key(
        store,
        user_id,
        feature,
        usage_mode,
        Some(&model),
        &usage,
        idempotency_key,
    );
    if let Some(result) = accounting.as_ref() {
        record_compute_meter_event(
            store,
            user_id,
            feature,
            usage_mode,
            &model,
            input_tokens,
            output_tokens,
            idempotency_key,
            units,
            result,
        );
    }
}

fn record_compute_meter_event(
    store: &Store,
    user_id: &str,
    feature: &str,
    usage_mode: &str,
    model: &str,
    metered_input_tokens: i64,
    metered_output_tokens: i64,
    idempotency_key: Option<&str>,
    units: MeterUnits<'_>,
    result: &TokenUsageAccountingResult,
) {
    if result.deduplicated {
        return;
    }
    let event = ComputeMeterEvent {
        user_id,
        compute_call_id: idempotency_key,
        feature,
        usage_mode,
        model: Some(model),
        source: units.source,
        input_unit_kind: units.input_unit_kind,
        output_unit_kind: units.output_unit_kind,
        input_units: units.input_units,
        output_units: units.output_units,
        metered_input_tokens,
        metered_output_tokens,
        token_usage_event_id: Some(result.token_usage_event_id.as_str()),
        billing_event_id: result.billing_event_id.as_deref(),
        cost_rmb_fen: result.cost_rmb_fen,
        accounting_status: result.accounting_status.as_str(),
    };
    if let Err(error) = store.record_compute_meter_event(&event) {
        tracing::warn!(
            user_id,
            feature,
            usage_mode,
            "record compute meter event failed: {}",
            error
        );
    }
}

fn image_generation_units(prompt: &str) -> (i64, i64) {
    (text_units(prompt, 4), 1_500)
}

fn metered_image_model(model: &str) -> String {
    format!("metered-image:{model}")
}

fn metered_realtime_model(model: &str) -> String {
    format!("metered-realtime:{model}")
}

fn text_units(text: &str, chars_per_unit: i64) -> i64 {
    ceil_div(text.chars().count() as i64, chars_per_unit.max(1)).max(1)
}

pub(crate) fn pcm_audio_units(pcm_bytes: usize, sample_rate: u32, channels: u16) -> i64 {
    ceil_div(
        pcm_audio_millis(pcm_bytes, sample_rate, channels) * 50,
        1_000,
    )
    .max(1)
}

fn pcm_audio_millis(pcm_bytes: usize, sample_rate: u32, channels: u16) -> i64 {
    let bytes_per_second = sample_rate.max(1) as i64
        * channels.max(1) as i64
        * crate::voice_config::PCM16_BYTES_PER_SAMPLE as i64;
    ceil_div(pcm_bytes as i64 * 1_000, bytes_per_second)
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
        assert_eq!(pcm_audio_millis(one_second, 24_000, 1), 1_000);
    }
}
