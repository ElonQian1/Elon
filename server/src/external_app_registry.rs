//! External app registry for tenant-style integrations.

use serde::Serialize;

use crate::store::ExternalAppGroupSeed;

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAppDefinition {
    pub id: &'static str,
    pub display_name: &'static str,
    pub chinese_name: &'static str,
    pub logo_text: &'static str,
    pub logo_url: Option<&'static str>,
    pub brand_color: &'static str,
    pub login_label: &'static str,
    pub login_hint: &'static str,
    pub login_url: &'static str,
    pub capabilities: &'static [&'static str],
    pub default_groups: &'static [ExternalAppGroupDefinition],
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAppGroupDefinition {
    pub external_group_id: &'static str,
    pub group_id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub description: &'static str,
    pub position: i64,
    pub auto_join: bool,
}

pub fn external_app_by_id(app_id: &str) -> Option<&'static ExternalAppDefinition> {
    EXTERNAL_APPS.iter().find(|app| app.id == app_id.trim())
}

pub fn external_group_by_group_id(
    group_id: &str,
) -> Option<(
    &'static ExternalAppDefinition,
    &'static ExternalAppGroupDefinition,
)> {
    let group_id = group_id.trim();
    EXTERNAL_APPS.iter().find_map(|app| {
        app.default_groups
            .iter()
            .find(|group| group.group_id == group_id)
            .map(|group| (app, group))
    })
}

pub fn public_external_app_config(app: &ExternalAppDefinition) -> serde_json::Value {
    serde_json::json!({
        "id": app.id,
        "display_name": app.display_name,
        "chinese_name": app.chinese_name,
        "logo_text": app.logo_text,
        "logo_url": app.logo_url,
        "brand_color": app.brand_color,
        "login_label": app.login_label,
        "login_hint": app.login_hint,
        "login_url": app.login_url,
        "capabilities": app.capabilities,
        "default_groups": app.default_groups,
    })
}

pub fn group_seeds(app: &ExternalAppDefinition) -> Vec<ExternalAppGroupSeed> {
    app.default_groups
        .iter()
        .map(|group| ExternalAppGroupSeed {
            app_id: app.id.to_string(),
            external_group_id: group.external_group_id.to_string(),
            group_id: group.group_id.to_string(),
            name: group.name.to_string(),
            position: group.position,
            auto_join: group.auto_join,
        })
        .collect()
}

pub fn service_token_env_names(app_id: &str) -> [String; 2] {
    let normalized = app_id
        .trim()
        .to_ascii_uppercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    [
        format!("ELON_EXTERNAL_APP_{}_TOKEN", normalized),
        "ELON_EXTERNAL_APP_TOKEN".to_string(),
    ]
}

static EXTERNAL_APPS: &[ExternalAppDefinition] = &[
    ExternalAppDefinition {
        id: "fb2",
        display_name: "夺冠体育",
        chinese_name: "夺冠体育",
        logo_text: "🏆",
        logo_url: None,
        brand_color: "#4f6bff",
        login_label: "使用夺冠体育账号登录",
        login_hint: "账号已在夺冠体育注册，请使用夺冠体育项目账号登录。",
        login_url: "fb2://auth/login",
        capabilities: &[
            "chat_center",
            "group_chat",
            "group_summary_posts",
            "ai_documents",
            "context_pack",
            "main_account_authorization",
            "voice_asr",
            "voice_tts",
            "realtime_transcribe",
            "external_group_voice_input",
            "chat_experience_bootstrap",
        ],
        default_groups: &[
            ExternalAppGroupDefinition {
                external_group_id: "official",
                group_id: "ext_fb2_official",
                name: "🏆 夺冠体育官方群",
                icon: "🏆",
                description: "fb2 用户默认加入的官方讨论群。",
                position: 10,
                auto_join: true,
            },
            ExternalAppGroupDefinition {
                external_group_id: "football",
                group_id: "ext_fb2_football",
                name: "⚽ 足彩交流群",
                icon: "⚽",
                description: "足球彩票、赛程、赔率与临场信息讨论。",
                position: 20,
                auto_join: false,
            },
            ExternalAppGroupDefinition {
                external_group_id: "basketball",
                group_id: "ext_fb2_basketball",
                name: "🏀 篮彩讨论群",
                icon: "🏀",
                description: "篮球赛事、篮彩玩法与赛前思路讨论。",
                position: 30,
                auto_join: false,
            },
            ExternalAppGroupDefinition {
                external_group_id: "expert",
                group_id: "ext_fb2_expert",
                name: "👑 专家推荐群",
                icon: "👑",
                description: "专家观点、推荐记录与赛后复盘。",
                position: 40,
                auto_join: false,
            },
            ExternalAppGroupDefinition {
                external_group_id: "newbie",
                group_id: "ext_fb2_newbie",
                name: "🌟 新手交流群",
                icon: "🌟",
                description: "新手玩法、下单流程和风险提示。",
                position: 50,
                auto_join: false,
            },
        ],
    },
    ExternalAppDefinition {
        id: "bb64a",
        display_name: "ElonSpeed",
        chinese_name: "ElonSpeed",
        logo_text: "ES",
        logo_url: None,
        brand_color: "#16a34a",
        login_label: "Use ElonSpeed account",
        login_hint: "ElonSpeed users can link the Windows client to the main AI platform.",
        login_url: "bb64a://auth/login",
        capabilities: &[
            "chat_center",
            "windows_client_ai",
            "local_mcp",
            "bb64a_doctor",
            "context_pack",
            "main_account_authorization",
            "pc_node_agent",
            "dangerous_runtime_tools",
            "source_node_bugfix",
        ],
        default_groups: &[
            ExternalAppGroupDefinition {
                external_group_id: "support",
                group_id: "ext_bb64a_support",
                name: "ElonSpeed Support",
                icon: "ES",
                description:
                    "Default support room for ElonSpeed Windows diagnostics and AI troubleshooting.",
                position: 10,
                auto_join: true,
            },
            ExternalAppGroupDefinition {
                external_group_id: "windows",
                group_id: "ext_bb64a_windows",
                name: "ElonSpeed Windows",
                icon: "PC",
                description:
                    "Windows client routing, proxy, TUN, system proxy and local MCP diagnostics.",
                position: 20,
                auto_join: false,
            },
            ExternalAppGroupDefinition {
                external_group_id: "release",
                group_id: "ext_bb64a_release",
                name: "ElonSpeed Release",
                icon: "REL",
                description:
                    "Aggregated user scenarios that may become BB64A source fixes and releases.",
                position: 30,
                auto_join: false,
            },
        ],
    },
];
