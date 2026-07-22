use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use axum::extract::ws::Message;
use serde_json::{json, Value};
use tokio::sync::oneshot;

use super::LiveUiSession;
use crate::node_agent_android_live::protocol::PROTOCOL_VERSION;

const ICON_ACK_TIMEOUT: Duration = Duration::from_secs(8);

impl LiveUiSession {
    pub(crate) async fn request_launcher_icon(
        &self,
        package_name: &str,
        size_px: u32,
    ) -> Result<Value> {
        let tx = self.runtime_tx.read().await.clone().ok_or_else(|| {
            anyhow!("Android Live Runtime 尚未连接，无法通过 LauncherApps/PackageManager 读取图标")
        })?;
        let request_id = format!("icon_{}", uuid::Uuid::new_v4().simple());
        let (waiter_tx, waiter_rx) = oneshot::channel();
        self.pending
            .lock()
            .await
            .insert(request_id.clone(), waiter_tx);
        tx.send(Message::Text(serde_json::to_string(&json!({
            "protocolVersion": PROTOCOL_VERSION,
            "messageType": "icon.request",
            "requestId": request_id,
            "packageName": package_name,
            "sizePx": size_px,
        }))?))
        .map_err(|_| anyhow!("Android Live Runtime 连接已断开"))?;
        let response = match tokio::time::timeout(ICON_ACK_TIMEOUT, waiter_rx).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => {
                self.pending.lock().await.remove(&request_id);
                bail!("Android 图标请求通道已关闭");
            }
            Err(_) => {
                self.pending.lock().await.remove(&request_id);
                bail!("等待 Android LauncherApps/PackageManager 图标超时");
            }
        };
        if response.get("messageType").and_then(Value::as_str) == Some("icon.reject") {
            bail!(
                "Android 拒绝图标请求: {}",
                response
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("未知错误")
            );
        }
        Ok(response)
    }
}
