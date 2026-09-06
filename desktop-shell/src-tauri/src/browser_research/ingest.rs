use super::{files, host::HostEvent, model::*, privacy};
use std::path::Path;

fn resource(
    session: &mut Session,
    root: &Path,
    event: &HostEvent,
    body: &str,
    kind: &str,
) -> Result<String, String> {
    let (body, redacted) = privacy::clean_body(body)?;
    let url = privacy::safe_url(&event.url).ok_or("invalid_resource_url")?;
    let digest = hash(body.as_bytes());
    if let Some(existing) = session.resources.iter().find(|r| {
        r.sha256 == digest
            && r.url == url
            && r.resource_type == kind
            && r.generation == event.generation
    }) {
        return Ok(existing.id.clone());
    }
    if session.resources.len() >= RESOURCE_LIMIT
        || session.bytes + body.len() as u64 > SESSION_BYTES
    {
        return Err("capture_capacity_reached".into());
    }
    files::save_body(root, &session.id, &body)?;
    let id = hash(
        format!(
            "{}:{}:{}:{}",
            session.id,
            event.generation,
            session.resources.len(),
            digest
        )
        .as_bytes(),
    );
    session.resources.push(Resource {
        id: id.clone(),
        url,
        resource_type: kind.into(),
        mime: event
            .mime
            .clone()
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect(),
        size_bytes: body.len(),
        sha256: digest,
        generation: event.generation,
        captured_at_ms: now_ms(),
        truncated: event.truncated,
        redacted,
        script_id: event.script_id.clone().filter(|id| id.len() <= 128),
        host_request_id: event.request_id.clone().filter(|id| id.len() <= 128),
    });
    session.bytes += body.len() as u64;
    Ok(id)
}

pub fn accept(session: &mut Session, root: &Path, event: HostEvent) -> Result<(), String> {
    if event.generation < session.generation {
        return Ok(());
    }
    if let Some(code) = event.error_code.as_deref() {
        session.gap(code);
    }
    if !files::manifests(root)
        .iter()
        .any(|m| m.id == session.site.id && m.fingerprint() == session.site_fingerprint)
    {
        session.active = false;
        session.phase = "scope_changed".into();
        return files::save_session(root, session);
    }
    if now_ms() >= session.expires_at_ms {
        session.active = false;
        session.phase = "expired".into();
        return files::save_session(root, session);
    }
    match event.kind.as_str() {
        "navigation" => {
            session.generation = event.generation;
            session.phase = if event.error_code.as_deref()
                == Some("identity_navigation_not_captured")
                || session.site.identity(&event.url)
                || privacy::identity_path(&event.url)
            {
                "login"
            } else {
                "loading"
            }
            .into();
            return files::save_session(root, session);
        }
        "paused" | "closed" => {
            session.generation = event.generation;
            session.active = false;
            session.phase = event.kind;
            return files::save_session(root, session);
        }
        "ready" => {
            session.generation = event.generation;
            if session.active {
                session.phase = "observing".into();
            }
            return files::save_session(root, session);
        }
        "gap" => {
            session.gap(event.error_code.as_deref().unwrap_or("capture_gap"));
            return files::save_session(root, session);
        }
        _ => {}
    }
    if !session.active
        || event.generation != session.generation
        || privacy::identity_path(&event.url)
    {
        return Ok(());
    }
    let is_request = event.kind == "request";
    if !session
        .site
        .allows(&event.url, if is_request { "api" } else { "resource" })
    {
        return Ok(());
    }
    if is_request {
        let Some(host_id) = event.request_id.as_deref().filter(|id| id.len() <= 256) else {
            return Ok(());
        };
        let observed_url = privacy::safe_url(&event.url).ok_or("invalid_resource_url")?;
        let id = hash(
            format!(
                "{}:{}:{host_id}:{observed_url}",
                session.id, event.generation
            )
            .as_bytes(),
        );
        let index = if let Some(i) = session.requests.iter().position(|r| r.id == id) {
            i
        } else {
            if session.requests.len() >= 512 {
                session.gap("request_limit");
                return files::save_session(root, session);
            }
            session.requests.push(Request {
                id: id.clone(),
                url: privacy::safe_url(&event.url).ok_or("invalid_resource_url")?,
                method: event
                    .method
                    .as_deref()
                    .unwrap_or("GET")
                    .chars()
                    .filter(|c| c.is_ascii_uppercase())
                    .take(12)
                    .collect(),
                status: None,
                generation: event.generation,
                request_resource_id: None,
                response_resource_id: None,
                initiator: None,
            });
            session.requests.len() - 1
        };
        if let Some(body) = event.request_body.as_deref() {
            let body_id = resource(session, root, &event, body, "request_body")?;
            session.requests[index].request_resource_id = Some(body_id);
        }
        if let Some(body) = event.body.as_deref() {
            let body_id = resource(session, root, &event, body, "response_body")?;
            session.requests[index].response_resource_id = Some(body_id);
        }
        if event.status.is_some() {
            session.requests[index].status = event.status;
        }
        if let Some(initiator) = event.initiator.as_ref() {
            session.requests[index].initiator = privacy::clean_initiator(initiator);
        }
    } else if let Some(body) = event.body.as_deref() {
        resource(
            session,
            root,
            &event,
            body,
            event.resource_type.as_deref().unwrap_or("script"),
        )?;
    }
    files::save_session(root, session)
}
