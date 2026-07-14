use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::NodeRuntime;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidDeviceLeaseProof {
    pub(crate) lease_id: String,
    pub(crate) project_id: String,
    pub(crate) hardware_serial: String,
}

pub(crate) async fn validate_android_device_lease(
    runtime: &NodeRuntime,
    proof: Option<&AndroidDeviceLeaseProof>,
) -> Result<()> {
    let proof = proof.context("这台公共测试手机尚未取得使用权，请先在网页端点击使用")?;
    let token = runtime
        .user_token()
        .await
        .context("Windows 节点尚未登录，无法校验公共测试手机使用权")?;
    let client = crate::node_agent_cloud_net::direct_cloud_client(Duration::from_secs(8))
        .context("创建设备占用校验客户端失败")?;
    let response = client
        .post(format!(
            "{}/api/me/modules/ui-tuner/android-device-lease/validate",
            runtime.cloud_http_url().trim_end_matches('/')
        ))
        .bearer_auth(token)
        .json(proof)
        .send()
        .await
        .context("无法连接云端校验公共测试手机使用权")?;
    if !response.status().is_success() {
        let message = response.text().await.unwrap_or_default();
        bail!(
            "公共测试手机使用权已失效或被其他用户占用{}",
            if message.is_empty() {
                String::new()
            } else {
                format!("：{message}")
            }
        );
    }
    Ok(())
}
