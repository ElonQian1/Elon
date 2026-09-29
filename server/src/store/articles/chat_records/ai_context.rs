//! Resolve a selected card by its persisted group message, never by client-supplied IDs.
use super::*;
use crate::store::social_ai_messages::requests::attachments::{append_record, GroupAiAttachment};
use serde_json::{json, Value};

pub(crate) fn expand(
    conn: &Connection,
    user: &str,
    group: &str,
    message: &str,
    content: &str,
    files: &mut Vec<GroupAiAttachment>,
) -> Result<Option<Value>> {
    if !content.starts_with(PREFIX) {
        return Ok(None);
    }
    let id: String = conn
        .query_row(
            "SELECT id FROM social_chat_records WHERE message_id=?1 AND group_id=?2",
            params![message, group],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| fail(404, "聊天记录已撤回或不可访问"))?;
    let view = readable(conn, user, group, &id)?;
    let mut messages = Vec::new();
    let mut assets = std::collections::BTreeMap::<String, String>::new();
    for row in &view.document.messages {
        let mut item = json!({"id":row.id,"parent_id":row.parent_id,"speaker":row.sender,
            "time":row.time,"kind":row.kind,"text":row.text});
        if matches!(row.kind.as_str(), "image" | "file" | "video" | "audio") {
            item["filename"] = json!(row.filename);
            item["media_status"] = json!("not_exported");
            if let Some(asset) = &row.asset_id {
                if matches!(row.kind.as_str(), "video" | "audio") {
                    item["media_status"] = json!("not_sent_audio_video");
                } else {
                    let name = match assets.get(asset) {
                        Some(name) => name.clone(),
                        None => {
                            let (mime, size, sha): (String, u64, String) = conn.query_row(
                                "SELECT a.mime_type,length(a.bytes),a.sha256 FROM social_chat_record_assets a
                                 JOIN social_chat_record_asset_refs r ON r.asset_id=a.id
                                 WHERE r.record_id=?1 AND a.id=?2 AND a.group_id=?3 AND a.owner_id=?4",
                                params![id, asset, group, view.owner_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
                            )?;
                            let path =
                                format!("/api/me/groups/{group}/chat-records/{id}/assets/{asset}");
                            append_record(
                                files,
                                message,
                                &json!([{"attachment_id":asset,"display_name":row.filename,
                                "mime_type":mime,"size_bytes":size,"sha256":sha,"url":path}]),
                            )?;
                            let name = files.last().expect("appended record asset").name.clone();
                            assets.insert(asset.clone(), name.clone());
                            name
                        }
                    };
                    item["attachment"] = json!(name);
                    item["media_status"] = json!("attached");
                }
            }
        }
        messages.push(item);
    }
    Ok(Some(
        json!({"source":"wechat_export","title":view.document.title,
        "messages":messages,"warnings":view.document.warnings,
        "media_policy":"Only attached files were supplied. Unexported media, audio, video and linked pages have not been read."}),
    ))
}
