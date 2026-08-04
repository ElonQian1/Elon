use serde_json::{json, Value};

pub(super) fn create_job_schema() -> Value {
    json!({
        "type":"object",
        "required":["job_id","idempotency_key","workload","provider_scope","max_consumer_charge_micros","currency"],
        "properties":{
            "job_id":{"type":"string","minLength":1,"maxLength":160},
            "idempotency_key":{"type":"string","minLength":1,"maxLength":200},
            "merchant_id":{"type":["string","null"],"maxLength":160},
            "workload":workload_schema(),
            "provider_scope":provider_scope_schema(),
            "max_consumer_charge_micros":{"type":"integer","minimum":0},
            "currency":{"type":"string","minLength":1,"maxLength":32}
        },
        "additionalProperties":false
    })
}

pub(super) fn quote_job_schema() -> Value {
    json!({
        "type":"object",
        "required":["job_id","offer_id","price_snapshot_id","expected_job_revision","expected_job_digest"],
        "properties":{
            "job_id":{"type":"string","minLength":1,"maxLength":160},
            "offer_id":{"type":"string","minLength":1,"maxLength":160},
            "price_snapshot_id":{"type":"string","minLength":1,"maxLength":160},
            "expected_job_revision":{"type":"integer","minimum":1},
            "expected_job_digest":{"type":"string","minLength":1,"maxLength":200}
        },
        "additionalProperties":false
    })
}

fn workload_schema() -> Value {
    json!({
        "type":"object",
        "required":["schema","task_kind","input_artifacts","resources","output","usage_limits","data_class","retry_policy","checkpoint_policy","verification_policy","deadline_at"],
        "properties":{
            "schema":{"type":"string","const":"compute_federation.workload.v1"},
            "task_kind":{"type":"string","enum":["llm_chat","embedding","rerank","image_generation","video_generation","evaluation_shard","gpu_batch"]},
            "input_artifacts":{"type":"array","items":artifact_schema()},
            "model":{"anyOf":[model_schema(),{"type":"null"}]},
            "runtime":{"anyOf":[runtime_schema(),{"type":"null"}]},
            "resources":resource_schema(),
            "output":output_schema(),
            "usage_limits":{"type":"array","items":{"type":"object","required":["meter","max_quantity"],"properties":{"meter":{"type":"string","minLength":1,"maxLength":80},"max_quantity":{"type":"integer","minimum":1}},"additionalProperties":false}},
            "data_class":{"type":"string","enum":["public","low_sensitivity","restricted"]},
            "shard":{"anyOf":[shard_schema(),{"type":"null"}]},
            "retry_policy":retry_schema(),
            "checkpoint_policy":checkpoint_schema(),
            "verification_policy":verification_schema(),
            "deadline_at":{"type":"string","format":"date-time"}
        },
        "additionalProperties":false
    })
}

fn provider_scope_schema() -> Value {
    json!({
        "type":"object",
        "required":["allowed_provider_ids","allowed_provider_kinds","excluded_provider_ids","required_trust_tier","required_regions"],
        "properties":{
            "allowed_provider_ids":{"type":"array","items":{"type":"string","minLength":1,"maxLength":160},"uniqueItems":true},
            "allowed_provider_kinds":{"type":"array","items":{"type":"string","enum":["user_node","managed_cluster","external_pool"]},"uniqueItems":true},
            "excluded_provider_ids":{"type":"array","items":{"type":"string","minLength":1,"maxLength":160},"uniqueItems":true},
            "required_trust_tier":{"type":"string","minLength":1,"maxLength":80},
            "required_regions":{"type":"array","items":{"type":"string","minLength":1,"maxLength":80},"uniqueItems":true}
        },
        "additionalProperties":false
    })
}

fn artifact_schema() -> Value {
    json!({
        "type":"object",
        "required":["artifact_id","digest_algorithm","digest","media_type","size_bytes","location_ref"],
        "properties":{
            "artifact_id":{"type":"string","minLength":1,"maxLength":160},
            "digest_algorithm":{"type":"string","minLength":1,"maxLength":32},
            "digest":{"type":"string","minLength":1,"maxLength":256},
            "media_type":{"type":"string","minLength":1,"maxLength":160},
            "size_bytes":{"type":"integer","minimum":0},
            "location_ref":{"type":"string","minLength":1,"maxLength":1000},
            "encryption_profile":{"type":["string","null"],"maxLength":160}
        },
        "additionalProperties":false
    })
}

fn model_schema() -> Value {
    json!({
        "type":"object",
        "required":["model_id","model_family","model_digest","adapter_digests"],
        "properties":{
            "model_id":{"type":"string","minLength":1,"maxLength":160},
            "model_family":{"type":"string","minLength":1,"maxLength":160},
            "model_digest":{"type":"string","minLength":1,"maxLength":256},
            "tokenizer_digest":{"type":["string","null"],"maxLength":256},
            "adapter_digests":{"type":"array","items":{"type":"string","minLength":1,"maxLength":256},"uniqueItems":true}
        },
        "additionalProperties":false
    })
}

fn runtime_schema() -> Value {
    json!({
        "type":"object",
        "required":["runtime_family","runtime_version","precision","runner_digest"],
        "properties":{
            "runtime_family":{"type":"string","minLength":1,"maxLength":160},
            "runtime_version":{"type":"string","minLength":1,"maxLength":80},
            "precision":{"type":"string","minLength":1,"maxLength":80},
            "runner_digest":{"type":"string","minLength":1,"maxLength":256},
            "plugin_id":{"type":["string","null"],"maxLength":160},
            "plugin_version":{"type":["string","null"],"maxLength":80},
            "plugin_digest":{"type":["string","null"],"maxLength":256}
        },
        "additionalProperties":false
    })
}

fn resource_schema() -> Value {
    json!({
        "type":"object",
        "required":["accelerator_kinds","min_accelerator_count","min_vram_bytes","min_ram_bytes","min_disk_bytes","max_runtime_seconds","allow_network_egress"],
        "properties":{
            "accelerator_kinds":{"type":"array","items":{"type":"string","minLength":1,"maxLength":80},"uniqueItems":true},
            "min_accelerator_count":{"type":"integer","minimum":0},
            "min_vram_bytes":{"type":"integer","minimum":0},
            "min_ram_bytes":{"type":"integer","minimum":0},
            "min_disk_bytes":{"type":"integer","minimum":0},
            "max_runtime_seconds":{"type":"integer","minimum":1},
            "allow_network_egress":{"type":"boolean"}
        },
        "additionalProperties":false
    })
}

fn output_schema() -> Value {
    json!({
        "type":"object",
        "required":["media_type","max_output_bytes","streaming","result_artifact_required","deterministic_digest_expected"],
        "properties":{
            "media_type":{"type":"string","minLength":1,"maxLength":160},
            "max_output_bytes":{"type":"integer","minimum":0},
            "streaming":{"type":"boolean"},
            "result_artifact_required":{"type":"boolean"},
            "deterministic_digest_expected":{"type":"boolean"}
        },
        "additionalProperties":false
    })
}

fn shard_schema() -> Value {
    json!({
        "type":"object",
        "required":["shard_id","shard_index","shard_count","merge_strategy"],
        "properties":{
            "shard_id":{"type":"string","minLength":1,"maxLength":160},
            "shard_index":{"type":"integer","minimum":0},
            "shard_count":{"type":"integer","minimum":1},
            "merge_strategy":{"type":"string","minLength":1,"maxLength":80}
        },
        "additionalProperties":false
    })
}

fn retry_schema() -> Value {
    json!({
        "type":"object",
        "required":["max_attempts","initial_backoff_ms","max_backoff_ms","retryable_error_codes"],
        "properties":{
            "max_attempts":{"type":"integer","minimum":1},
            "initial_backoff_ms":{"type":"integer","minimum":0},
            "max_backoff_ms":{"type":"integer","minimum":0},
            "retryable_error_codes":{"type":"array","items":{"type":"string","minLength":1,"maxLength":80},"uniqueItems":true}
        },
        "additionalProperties":false
    })
}

fn checkpoint_schema() -> Value {
    json!({
        "type":"object",
        "required":["mode","max_checkpoints"],
        "properties":{
            "mode":{"type":"string","minLength":1,"maxLength":80},
            "interval_seconds":{"type":["integer","null"],"minimum":1},
            "max_checkpoints":{"type":"integer","minimum":0},
            "checkpoint_media_type":{"type":["string","null"],"maxLength":160}
        },
        "additionalProperties":false
    })
}

fn verification_schema() -> Value {
    json!({
        "type":"object",
        "required":["verification_tier","minimum_independent_receipts","duplicate_sample_rate_basis_points","require_server_metering"],
        "properties":{
            "verification_tier":{"type":"string","minLength":1,"maxLength":80},
            "minimum_independent_receipts":{"type":"integer","minimum":0},
            "duplicate_sample_rate_basis_points":{"type":"integer","minimum":0,"maximum":10000},
            "challenge_profile_id":{"type":["string","null"],"maxLength":160},
            "require_server_metering":{"type":"boolean"}
        },
        "additionalProperties":false
    })
}
