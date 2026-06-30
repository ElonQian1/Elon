use anyhow::Result;
use serde::Serialize;

use crate::{group_ai::types::ProjectAiEvent, types::AppState};

#[derive(Debug, Serialize)]
pub(crate) struct MatterEventsDelta {
    pub events: Vec<ProjectAiEvent>,
    pub latest_event_id: Option<String>,
    pub latest_event_created_at: Option<String>,
    pub has_more: bool,
}

pub(crate) fn matter_events_delta(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    after: Option<&str>,
    limit: i64,
) -> Result<MatterEventsDelta> {
    if state
        .store
        .get_project_ai_matter(project_id, matter_id)?
        .is_none()
    {
        anyhow::bail!("Matter 不存在");
    }
    let limit = limit.clamp(1, 200) as usize;
    let all_events = state
        .store
        .list_project_ai_matter_events(project_id, matter_id)?;
    let start_index = after
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|after| event_start_index(&all_events, after))
        .unwrap_or(0);
    let remaining = all_events.len().saturating_sub(start_index);
    let events = all_events
        .into_iter()
        .skip(start_index)
        .take(limit)
        .collect::<Vec<_>>();
    let latest_event_id = events.last().map(|event| event.id.clone());
    let latest_event_created_at = events.last().map(|event| event.created_at.clone());
    Ok(MatterEventsDelta {
        events,
        latest_event_id,
        latest_event_created_at,
        has_more: remaining > limit,
    })
}

fn event_start_index(events: &[ProjectAiEvent], after: &str) -> Option<usize> {
    if let Some(index) = events.iter().position(|event| event.id == after) {
        return Some(index + 1);
    }
    Some(
        events
            .iter()
            .position(|event| event.created_at.as_str() > after)
            .unwrap_or(events.len()),
    )
}
