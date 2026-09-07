//! Private bodies never travel on the legacy WS/HTTP connection.
use super::{bound, database, identity};
use crate::{
    private_read_projection::{self as contract, storage},
    NodeRuntime,
};
use std::{sync::Arc, time::Duration};

fn endpoint() -> Result<reqwest::Url, &'static str> {
    let origin =
        std::env::var("ELON_PRIVATE_READ_HTTPS_ORIGIN").map_err(|_| "projection_https_disabled")?;
    contract::transport::endpoint(&origin)
}
pub(crate) fn spawn(runtime: Arc<NodeRuntime>) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(8))
            .build()
        {
            Ok(v) => v,
            Err(_) => return,
        };
        let mut tick = tokio::time::interval(Duration::from_secs(5));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            sync_once(&runtime, &client).await;
        }
    });
}
async fn sync_once(runtime: &Arc<NodeRuntime>, client: &reqwest::Client) {
    let (creds, epoch) = runtime.credential_session().await;
    let Some(creds) = creds.filter(|c| bound(c, &runtime.install_id)) else {
        return;
    };
    // An endpoint root cannot be downgraded to the legacy database secret surface.
    if runtime.endpoint_credentials.endpoint_required().await {
        return;
    }
    let hash = contract::digest(creds.agent_secret.as_bytes());
    let binding = identity(&creds, &runtime.install_id, &hash);
    let Ok(pending) = database().and_then(|conn| storage::pending(&conn, &binding)) else {
        return;
    };
    for value in pending {
        let mut epoch_rx = runtime.subscribe_credential_epoch();
        if runtime.require_credential_epoch(epoch).is_err() {
            return;
        }
        let outcome = match endpoint() {
            Err(code) => Err((false, code)),
            Ok(url) => {
                let send = client
                    .post(url)
                    .bearer_auth(&creds.agent_secret)
                    .header("x-elon-node-id", &creds.agent_id)
                    .header("cache-control", "no-store")
                    .json(&value)
                    .send();
                tokio::select! {
                    _=epoch_rx.changed()=>return,
                    result=send=>match result {
                      Err(_)=>Err((false,"projection_https_unavailable")),
                      Ok(response)=>read_ack(response,&value).await
                    }
                }
            }
        };
        if runtime.require_credential_epoch(epoch).is_err() {
            return;
        }
        let (accepted, rejected, code) = match outcome {
            Ok(()) => (true, false, None),
            Err((permanent, code)) => (false, permanent, Some(code)),
        };
        let _ = runtime
            .with_current_credential_session(epoch, &creds, || {
                database().and_then(|conn| {
                    storage::settle(
                        &conn,
                        &binding,
                        &value,
                        contract::now_ms(),
                        accepted,
                        rejected,
                        code,
                    )
                })
            })
            .await;
    }
}
async fn read_ack(
    mut response: reqwest::Response,
    value: &contract::Projection,
) -> Result<(), (bool, &'static str)> {
    if response.status() != reqwest::StatusCode::OK {
        let permanent = matches!(
            response.status().as_u16(),
            400 | 401 | 403 | 409 | 413 | 422
        );
        return Err((
            permanent,
            if permanent {
                "projection_cloud_rejected"
            } else {
                "projection_cloud_unavailable"
            },
        ));
    }
    if response.content_length().is_some_and(|v| v > 4096) {
        return Err((false, "projection_ack_invalid"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| (false, "projection_ack_unavailable"))?
    {
        if bytes.len() + chunk.len() > 4096 {
            return Err((false, "projection_ack_invalid"));
        }
        bytes.extend_from_slice(&chunk);
    }
    contract::transport::validate_ack(&bytes, value).map_err(|code| (false, code))
}
