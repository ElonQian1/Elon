use super::{files, model::*, privacy};
use serde_json::{json, Value};
use std::path::Path;

fn page(items: Vec<Value>, command: &ResearchCommand) -> Value {
    let total = items.len();
    let offset = command.offset.unwrap_or(0).min(total);
    let limit = command.limit.unwrap_or(30).clamp(1, 50);
    let mut end = offset;
    let mut bytes: usize = 0;
    while end < (offset + limit).min(total) {
        let item_bytes = serde_json::to_vec(&items[end])
            .map(|b| b.len())
            .unwrap_or(usize::MAX);
        if bytes.saturating_add(item_bytes) > 52 * 1024 {
            break;
        }
        bytes += item_bytes;
        end += 1;
    }
    json!({"schema":RESULT_SCHEMA,"kind":command.kind,"items":&items[offset..end],"total":total,
        "offset":offset,"next_offset":if end<total{Some(end)}else{None}})
}
pub fn list(items: Vec<Value>, command: &ResearchCommand) -> Value {
    page(items, command)
}

fn slice(
    root: &Path,
    session: &Session,
    item: &Resource,
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    let body = files::read_body(root, &session.id, &item.sha256)?;
    // Recheck locally edited content before crossing MCP; its stored digest is checked first.
    let (body, _) = privacy::clean_body(&body)?;
    if offset > body.len() || !body.is_char_boundary(offset) {
        return Err("invalid_content_offset".into());
    }
    let mut end = (offset + limit.clamp(1, 8192)).min(body.len());
    while end > offset && !body.is_char_boundary(end) {
        end -= 1;
    }
    Ok(
        json!({"item":item,"content":&body[offset..end],"offset":offset,
        "next_offset":if end<body.len(){Some(end)}else{None},"complete":end==body.len()}),
    )
}
pub fn execute(root: &Path, session: &Session, command: &ResearchCommand) -> Result<Value, String> {
    match command.kind.as_str() {
        "status" => {
            Ok(json!({"schema":RESULT_SCHEMA,"kind":command.kind,"session":session.summary()}))
        }
        "resources" => Ok(page(
            session.resources.iter().map(|r| json!(r)).collect(),
            command,
        )),
        "requests" => Ok(page(
            session.requests.iter().map(|r| json!(r)).collect(),
            command,
        )),
        "read_resource" => {
            let item = session
                .resources
                .iter()
                .find(|r| Some(&r.id) == command.resource_id.as_ref())
                .ok_or("resource_not_found")?;
            let mut result = slice(
                root,
                session,
                item,
                command.offset.unwrap_or(0),
                command.limit.unwrap_or(8192),
            )?;
            result["schema"] = json!(RESULT_SCHEMA);
            result["kind"] = json!(command.kind);
            Ok(result)
        }
        "read_request" => {
            let item = session
                .requests
                .iter()
                .find(|r| Some(&r.id) == command.request_id.as_ref())
                .ok_or("request_not_found")?;
            let body = |id: &Option<String>| -> Result<Value, String> {
                match id
                    .as_ref()
                    .and_then(|id| session.resources.iter().find(|r| r.id == *id))
                {
                    Some(resource) => slice(
                        root,
                        session,
                        resource,
                        command.offset.unwrap_or(0).min(resource.size_bytes),
                        command.limit.unwrap_or(4096),
                    ),
                    None => Ok(Value::Null),
                }
            };
            Ok(
                json!({"schema":RESULT_SCHEMA,"kind":command.kind,"request":item,
                "request_body":body(&item.request_resource_id)?,"response_body":body(&item.response_resource_id)?}),
            )
        }
        "search" => search(root, session, command),
        _ => Err("unsupported_research_action".into()),
    }
}
fn search(root: &Path, session: &Session, command: &ResearchCommand) -> Result<Value, String> {
    let query = command
        .query
        .as_deref()
        .filter(|q| !q.trim().is_empty() && q.len() <= 200)
        .ok_or("invalid_search_query")?;
    let mut items = Vec::new();
    let mut partial = false;
    for item in &session.resources {
        if items.len() >= 200 {
            partial = true;
            break;
        }
        let Ok(body) = files::read_body(root, &session.id, &item.sha256) else {
            partial = true;
            continue;
        };
        let (body, _) = privacy::clean_body(&body)?;
        for (index, (offset, _)) in body.match_indices(query).take(21).enumerate() {
            if index == 20 {
                partial = true;
                break;
            }
            let mut start = offset.saturating_sub(160);
            while !body.is_char_boundary(start) {
                start += 1;
            }
            let mut end = (offset + query.len() + 240).min(body.len());
            while !body.is_char_boundary(end) {
                end -= 1;
            }
            items.push(json!({"resource_id":item.id,"url":item.url,"offset":offset,"excerpt":&body[start..end]}));
            if items.len() >= 200 {
                break;
            }
        }
    }
    let mut result = page(items, command);
    result["partial"] = json!(partial);
    Ok(result)
}
