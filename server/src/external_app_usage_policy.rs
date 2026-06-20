//! Public usage and billing boundary for external app integrations.

use serde_json::{json, Value};

pub(crate) fn public_usage_policy_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.usage_policy.v1",
            "free_channels": [
                {
                    "name": "android_system_asr",
                    "metering": "free",
                    "reason": "手机系统本地语音识别不消耗主项目算力。",
                    "billing_gate": "never"
                },
                {
                    "name": "cloud_asr_fallback",
                    "metering": "free",
                    "endpoint": "/api/voice/asr",
                    "reason": "云端 ASR 是聊天基础输入通道，不因 AI 额度为 0 拒绝。",
                    "billing_gate": "never"
                },
                {
                    "name": "tts",
                    "metering": "free",
                    "endpoint": "/api/voice/tts",
                    "reason": "TTS 是聊天基础输出通道，不按模型 token 计费。",
                    "billing_gate": "never"
                },
                {
                    "name": "external_context_fetch",
                    "metering": "free",
                    "endpoint": "/api/main-project/context/pack",
                    "reason": "上下文拉取是 AI 回复前的数据准备，不直接消耗用户 token。",
                    "billing_gate": "auth_and_limits_only"
                }
            ],
            "billable_channels": [
                {
                    "name": "ai_reply_generation",
                    "metering": "token_or_model_usage",
                    "reason": "只有模型生成回复内容才消耗 AI 额度。",
                    "examples": [
                        "群聊 @AI 后生成回答",
                        "AI 助手回答用户问题",
                        "赛事分析生成文本",
                        "用户订单/票据风险剖析生成文本"
                    ],
                    "billing_gate": "before_model_call"
                }
            ],
            "integration_rules": [
                "fb2 不应在 ASR/TTS 按钮或上下文拉取前检查 AI token 余额。",
                "余额不足只能阻断 AI 生成回复，不能阻断录音转文字、文字转语音或 context pack 拉取。",
                "ASR/TTS/context fetch 仍必须保留鉴权、文件大小、时长、频率和安全限制。",
                "免费试用额度应配置在 AI 回复/模型调用层，不配置在 ASR/TTS 层。"
            ],
            "default_usage_policy": default_usage_policy()
        })),
        _ => None,
    }
}

pub(crate) fn default_usage_policy() -> Value {
    json!({
        "asr_free": true,
        "tts_free": true,
        "context_fetch_free": true,
        "ai_reply_billable": true,
        "no_guaranteed_win": true,
        "no_betting_commitment": true,
        "explain_uncertainty": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_public_usage_policy_guidance() {
        let guidance = public_usage_policy_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.usage_policy.v1");
        assert!(guidance["free_channels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|channel| channel["name"] == "cloud_asr_fallback"));
        assert_eq!(guidance["default_usage_policy"]["ai_reply_billable"], true);
        assert!(public_usage_policy_guidance("unknown").is_none());
    }

    #[test]
    fn default_policy_keeps_voice_and_context_free() {
        let policy = default_usage_policy();
        assert_eq!(policy["asr_free"], true);
        assert_eq!(policy["tts_free"], true);
        assert_eq!(policy["context_fetch_free"], true);
        assert_eq!(policy["ai_reply_billable"], true);
    }
}
