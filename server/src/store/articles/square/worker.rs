use super::*;
use crate::types::AppState;
use std::{sync::Arc, time::Duration};
pub(crate) fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        let Ok(client) = provider::Client::new() else {
            tracing::error!("SQUARE_CLIENT_INIT_FAILED");
            return;
        };
        loop {
            match state.store.square_claim() {
                Ok(Some(work)) => execute(&state.store, &client, work).await,
                Ok(None) => tokio::time::sleep(Duration::from_secs(3)).await,
                Err(_) => {
                    tracing::warn!("SQUARE_QUEUE_READ_FAILED");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    });
}
async fn execute(store: &Store, client: &provider::Client, work: Work) {
    let prepared =
        tokio::time::timeout(Duration::from_secs(600), prepare(store, client, &work)).await;
    let (key, body) = match prepared {
        Ok(Ok(value)) => value,
        result => {
            let message = match result {
                Ok(Err(e)) => e
                    .downcast_ref::<ArticleFault>()
                    .map(|f| f.1.clone())
                    .unwrap_or_else(|| "发布准备失败，尚未提交，可重试".into()),
                _ => "媒体准备超时，尚未提交，可重试".into(),
            };
            let _ = store.square_finish(&work, "failed", &message, None);
            return;
        }
    };
    if let Err(error) = store.square_submitting(&work) {
        let message = error
            .downcast_ref::<ArticleFault>()
            .map(|f| f.1.as_str())
            .unwrap_or("发布状态变化，请重新读取");
        let _ = store.square_finish(&work, "failed", message, None);
        return;
    }
    let (status, message, id) = match client.publish(&key, body).await {
        provider::Outcome::Published(id) => ("published", "币安已返回成功回执".into(), Some(id)),
        provider::Outcome::Failed(message) => ("failed", message, None),
        provider::Outcome::Uncertain => (
            "uncertain",
            "提交后没有取得可靠回执。请到币安核实，避免重复发布".into(),
            None,
        ),
    };
    if store
        .square_finish(&work, status, &message, id.as_deref())
        .is_err()
    {
        tracing::error!("SQUARE_RECEIPT_PERSIST_FAILED");
    }
}
async fn image(
    store: &Store,
    client: &provider::Client,
    work: &Work,
    key: &str,
    id: &str,
) -> Result<String> {
    store.square_reserve_upload(work)?;
    let (mime, bytes) = store.square_media_bytes(&work.owner, id, false)?;
    Ok(client.upload(key, &mime, bytes, false).await?.1)
}
async fn prepare(store: &Store, client: &provider::Client, work: &Work) -> Result<(String, Value)> {
    let key = store.square_work_key(work)?;
    let p = &work.payload;
    let mut body = json!({"contentType":1,"bodyTextOnly":p.text});
    match p.selection.mode {
        Mode::Article => {
            body["contentType"] = json!(2);
            body["title"] = json!(p.title);
            if let Some(id) = &p.selection.cover_id {
                body["cover"] = json!(image(store, client, work, &key, id).await?);
            }
        }
        Mode::Text => {}
        Mode::Images => {
            let mut urls = vec![];
            for id in &p.selection.media_ids {
                urls.push(image(store, client, work, &key, id).await?);
            }
            body["imageList"] = json!(urls);
        }
        Mode::Video => {
            let id = p
                .selection
                .video_id
                .as_deref()
                .ok_or_else(|| fail(400, "视频缺失"))?;
            let (duration, cover) = store.square_video_metadata(&work.owner, id)?;
            store.square_reserve_upload(work)?;
            let (mime, bytes) = store.square_media_bytes(&work.owner, id, true)?;
            let (ticket, _) = client.upload(&key, &mime, bytes, true).await?;
            body["contentType"] = json!(3);
            body["fileTicket"] = json!(ticket);
            body["cover"] = json!(image(store, client, work, &key, &cover).await?);
            body["videoTimeSeconds"] = json!(duration);
            body["isPublish"] = json!(true);
        }
    }
    Ok((key, body))
}
