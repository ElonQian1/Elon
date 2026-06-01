//! join_request_events.rs — 项目加入申请 WS 推送事件
//!
//! 两类事件：
//!   - `join_request_received`：推给项目 owner（有新申请待审）
//!   - `join_request_reviewed`：推给申请人（审批结果）

use serde::Serialize;
use std::sync::LazyLock;
use tokio::sync::broadcast;

static TX: LazyLock<broadcast::Sender<JoinRequestEvent>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(128);
    tx
});

/// 加入申请推送事件（owner 收到新申请 / 申请人收到审批结果）
#[derive(Debug, Clone, Serialize)]
pub struct JoinRequestEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "projectName")]
    pub project_name: String,
    /// 申请人账号（owner 接收时用）
    #[serde(rename = "applicantAccount")]
    pub applicant_account: String,
    /// 审批结果：approved / rejected（申请人接收时用）
    pub status: String,
    /// 目标推送用户 ID（不序列化，仅 global_ws 路由用）
    #[serde(skip)]
    pub target_user_id: String,
}

impl JoinRequestEvent {
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

pub fn subscribe() -> broadcast::Receiver<JoinRequestEvent> {
    TX.subscribe()
}

/// 向 owner 推送"有新申请"事件
pub fn publish_new_request(
    owner_user_id: &str,
    request_id: &str,
    project_id: &str,
    project_name: &str,
    applicant_account: &str,
) {
    let _ = TX.send(JoinRequestEvent {
        event_type: "join_request_received".to_string(),
        request_id: request_id.to_string(),
        project_id: project_id.to_string(),
        project_name: project_name.to_string(),
        applicant_account: applicant_account.to_string(),
        status: "pending".to_string(),
        target_user_id: owner_user_id.to_string(),
    });
}

/// 向申请人推送审批结果
pub fn publish_review_result(
    applicant_user_id: &str,
    request_id: &str,
    project_id: &str,
    project_name: &str,
    status: &str, // "approved" | "rejected"
) {
    let _ = TX.send(JoinRequestEvent {
        event_type: "join_request_reviewed".to_string(),
        request_id: request_id.to_string(),
        project_id: project_id.to_string(),
        project_name: project_name.to_string(),
        applicant_account: String::new(),
        status: status.to_string(),
        target_user_id: applicant_user_id.to_string(),
    });
}
