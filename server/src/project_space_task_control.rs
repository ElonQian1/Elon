use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use tokio::sync::watch;

static CHANNEL_AI_TASKS: LazyLock<Mutex<HashMap<String, ChannelAiTaskControl>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) struct ChannelAiTaskControl {
    project_id: String,
    channel_id: String,
    cancel_tx: watch::Sender<bool>,
}

impl ChannelAiTaskControl {
    pub(crate) fn request_cancel(&self) {
        let _ = self.cancel_tx.send(true);
    }
}

pub(crate) fn register_channel_ai_task_control(
    task_id: &str,
    project_id: &str,
    channel_id: &str,
    cancel_tx: watch::Sender<bool>,
) {
    if let Ok(mut tasks) = CHANNEL_AI_TASKS.lock() {
        tasks.insert(
            task_id.to_string(),
            ChannelAiTaskControl {
                project_id: project_id.to_string(),
                channel_id: channel_id.to_string(),
                cancel_tx,
            },
        );
    }
}

pub(crate) fn active_channel_ai_task_ids() -> Vec<String> {
    CHANNEL_AI_TASKS
        .lock()
        .map(|tasks| tasks.keys().cloned().collect())
        .unwrap_or_default()
}

pub(crate) fn is_channel_ai_task_active(task_id: &str, project_id: &str, channel_id: &str) -> bool {
    CHANNEL_AI_TASKS
        .lock()
        .ok()
        .and_then(|tasks| {
            tasks
                .get(task_id)
                .map(|task| task.project_id == project_id && task.channel_id == channel_id)
        })
        .unwrap_or(false)
}

pub(crate) fn take_channel_ai_task_control(
    task_id: &str,
    project_id: &str,
    channel_id: &str,
) -> Option<ChannelAiTaskControl> {
    let mut tasks = CHANNEL_AI_TASKS.lock().ok()?;
    let matches = tasks
        .get(task_id)
        .map(|task| task.project_id == project_id && task.channel_id == channel_id)
        .unwrap_or(false);
    if matches {
        tasks.remove(task_id)
    } else {
        None
    }
}

pub(crate) fn remove_channel_ai_task_control(task_id: &str) {
    if let Ok(mut tasks) = CHANNEL_AI_TASKS.lock() {
        tasks.remove(task_id);
    }
}
