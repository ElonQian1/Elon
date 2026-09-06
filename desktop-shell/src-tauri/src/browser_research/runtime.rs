use super::{files, host, ingest, model::*, query};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Default)]
pub(crate) struct ResearchRuntime {
    inner: Arc<Mutex<Core>>,
}
#[derive(Default)]
struct Core {
    sessions: HashMap<String, Session>,
    hosts: HashMap<String, host::HostHandle>,
    overflows: HashMap<String, Arc<AtomicU64>>,
    loaded: HashSet<String>,
}
struct Scope {
    root: PathBuf,
    project: String,
    owner: String,
}

impl Core {
    fn prune_missing_hosts(&mut self, mut exists: impl FnMut(&str) -> bool) {
        let removed = prune_handles(
            &mut self.hosts,
            &mut self.overflows,
            |host| host.label.as_str(),
            &mut exists,
        );
        for id in removed {
            if let Some(session) = self.sessions.get_mut(&id) {
                session.active = false;
                session.phase = "closed".into();
            }
        }
    }
}
fn prune_handles<H>(
    hosts: &mut HashMap<String, H>,
    overflows: &mut HashMap<String, Arc<AtomicU64>>,
    label: impl Fn(&H) -> &str,
    mut exists: impl FnMut(&str) -> bool,
) -> Vec<String> {
    let mut removed = Vec::new();
    hosts.retain(|id, host| {
        let keep = exists(label(host));
        if !keep {
            removed.push(id.clone());
        }
        keep
    });
    overflows.retain(|id, _| hosts.contains_key(id));
    removed
}

impl ResearchRuntime {
    fn scope(&self, app: &AppHandle, project: &str, owner: &str) -> Result<Scope, String> {
        if !digest_id(project)
            || owner.is_empty()
            || owner.len() > 512
            || owner.chars().any(char::is_control)
        {
            return Err("invalid_research_identity".into());
        }
        let owner = hash(owner.as_bytes());
        let root = app
            .path()
            .app_local_data_dir()
            .map_err(|_| "storage_unavailable")?
            .join("browser-research-v1")
            .join(project)
            .join(&owner);
        files::ensure_directory(&root)?;
        let mut core = self.inner.lock().map_err(|_| "research_unavailable")?;
        if core.loaded.insert(format!("{project}:{owner}")) {
            for session in files::load_sessions(&root, project, &owner) {
                core.sessions.insert(session.id.clone(), session);
            }
        }
        Ok(Scope {
            root,
            project: project.into(),
            owner,
        })
    }
    pub fn execute(
        &self,
        app: &AppHandle,
        project: &str,
        owner: &str,
        command: ResearchCommand,
    ) -> Result<Value, String> {
        let scope = self.scope(app, project, owner)?;
        self.inner
            .lock()
            .map_err(|_| "research_unavailable")?
            .prune_missing_hosts(|label| app.get_webview(label).is_some());
        match command.kind.as_str() {
            "sites" => {
                return Ok(query::list(
                    files::manifests(&scope.root)
                        .iter()
                        .map(|s| json!(s))
                        .collect(),
                    &command,
                ))
            }
            "register_site" => {
                let manifest = command.manifest.clone().ok_or("invalid_site_manifest")?;
                let items = files::register(&scope.root, manifest)?;
                return Ok(query::list(
                    items.iter().map(|s| json!(s)).collect(),
                    &command,
                ));
            }
            "sessions" => {
                let core = self.inner.lock().map_err(|_| "research_unavailable")?;
                let mut sessions: Vec<&Session> = core
                    .sessions
                    .values()
                    .filter(|s| s.project_key == scope.project && s.owner_hash == scope.owner)
                    .collect();
                sessions.sort_by_key(|s| std::cmp::Reverse(s.expires_at_ms));
                return Ok(query::list(
                    sessions.iter().map(|s| s.summary()).collect(),
                    &command,
                ));
            }
            "open" => return self.open(app, scope, &command),
            _ => {}
        }
        let id = command.session_id.as_deref().ok_or("session_required")?;
        let mut core = self.inner.lock().map_err(|_| "research_unavailable")?;
        if core
            .overflows
            .get(id)
            .is_some_and(|count| count.load(Ordering::Relaxed) > 0)
        {
            if let Some(session) = core.sessions.get_mut(id) {
                session.gap("event_queue_full");
            }
        }
        let session = core.sessions.get(id).ok_or("session_not_found")?;
        if session.project_key != scope.project || session.owner_hash != scope.owner {
            return Err("session_scope_mismatch".into());
        }
        if !files::manifests(&scope.root)
            .iter()
            .any(|m| m.id == session.site.id && m.fingerprint() == session.site_fingerprint)
        {
            return Err("site_scope_changed".into());
        }
        if command.kind == "pause" || command.kind == "resume" {
            let handle = core
                .hosts
                .get(id)
                .cloned()
                .ok_or("research_host_unavailable")?;
            if command.kind == "resume" && now_ms() >= session.expires_at_ms {
                return Err("research_session_expired".into());
            }
            let session = core.sessions.get_mut(id).ok_or("session_not_found")?;
            session.active = command.kind == "resume";
            session.phase = if session.active {
                "observing"
            } else {
                "paused"
            }
            .into();
            let result =
                json!({"schema":RESULT_SCHEMA,"kind":command.kind,"session":session.summary()});
            files::save_session(&scope.root, session)?;
            drop(core);
            if command.kind == "pause" {
                host::pause(&handle);
            } else {
                host::resume(app, &handle)?;
            }
            return Ok(result);
        }
        // Expired grants cannot disclose persisted private bodies; opening again requires a new session.
        if now_ms() >= session.expires_at_ms && command.kind != "status" {
            return Err("research_session_expired".into());
        }
        query::execute(&scope.root, session, &command)
    }
    fn open(
        &self,
        app: &AppHandle,
        scope: Scope,
        command: &ResearchCommand,
    ) -> Result<Value, String> {
        let site = files::manifests(&scope.root)
            .into_iter()
            .find(|s| Some(&s.id) == command.site_id.as_ref())
            .ok_or("site_not_found")?;
        site.validate()?;
        let mut core = self.inner.lock().map_err(|_| "research_unavailable")?;
        core.prune_missing_hosts(|label| app.get_webview(label).is_some());
        if let Some(existing) = core.sessions.values().find(|s| {
            s.project_key == scope.project
                && s.owner_hash == scope.owner
                && s.site.id == site.id
                && s.site_fingerprint == site.fingerprint()
                && s.active
                && now_ms() < s.expires_at_ms
        }) {
            return Ok(json!({"schema":RESULT_SCHEMA,"kind":"open","session":existing.summary()}));
        }
        if core.hosts.len() >= 8 {
            return Err("research_window_limit".into());
        }
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let id = hash(
            format!(
                "{}:{}:{}:{}:{}",
                scope.project,
                scope.owner,
                now_ms(),
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            )
            .as_bytes(),
        );
        let session = Session {
            schema: "yilong.browser-research.session.v1".into(),
            id: id.clone(),
            project_key: scope.project.clone(),
            owner_hash: scope.owner.clone(),
            site_fingerprint: site.fingerprint(),
            site: site.clone(),
            active: true,
            generation: 0,
            expires_at_ms: now_ms() + SESSION_DURATION,
            phase: "opening".into(),
            bytes: 0,
            resources: vec![],
            requests: vec![],
            gaps: vec![],
        };
        files::save_session(&scope.root, &session)?;
        let config = host::HostConfig {
            label: format!("browser-research-{}", &id[..32]),
            start_url: site.entry_url.clone(),
            profile_dir: scope.root.join("profiles").join(&site.id),
            navigation_origins: site.navigation_origins,
            resource_origins: site.resource_origins,
            api_origins: site.api_origins,
            identity_origins: site.identity_origins,
            max_body_bytes: BODY_LIMIT,
            expires_at_ms: session.expires_at_ms,
        };
        core.sessions.insert(id.clone(), session);
        drop(core);
        let (tx, rx) = std::sync::mpsc::sync_channel::<host::HostEvent>(64);
        let worker = self.clone();
        let worker_id = id.clone();
        let worker_root = scope.root.clone();
        std::thread::Builder::new()
            .name("browser-research-store".into())
            .spawn(move || {
                while let Ok(event) = rx.recv() {
                    if let Ok(mut core) = worker.inner.lock() {
                        if let Some(session) = core.sessions.get_mut(&worker_id) {
                            if let Err(code) = ingest::accept(session, &worker_root, event) {
                                session.gap(&code);
                                let _ = files::save_session(&worker_root, session);
                            }
                        }
                    }
                }
            })
            .map_err(|_| "research_worker_unavailable")?;
        let overflow = Arc::new(AtomicU64::new(0));
        let sink_overflow = overflow.clone();
        let sink: host::HostSink = Arc::new(move |event| {
            if tx.try_send(event).is_err() {
                sink_overflow.fetch_add(1, Ordering::Relaxed);
            }
        });
        let opened = host::open(app, config, sink);
        let mut core = self.inner.lock().map_err(|_| "research_unavailable")?;
        core.overflows.insert(id.clone(), overflow.clone());
        match opened {
            Ok(handle) => {
                core.hosts.insert(id.clone(), handle);
            }
            Err(code) => {
                if let Some(s) = core.sessions.get_mut(&id) {
                    s.active = false;
                    s.phase = "host_unavailable".into();
                    s.gap(&code);
                    let _ = files::save_session(&scope.root, s);
                }
                return Err(code);
            }
        }
        let session = core.sessions.get_mut(&id).ok_or("session_not_found")?;
        if overflow.load(Ordering::Relaxed) > 0 {
            session.gap("event_queue_full");
        }
        Ok(json!({"schema":RESULT_SCHEMA,"kind":"open","session":session.summary()}))
    }
}

#[cfg(test)]
mod handle_tests {
    use super::*;
    #[test]
    fn closed_windows_release_slots_but_open_paused_login_windows_keep_theirs() {
        let mut hosts: HashMap<String, String> = (0..8)
            .map(|n| (n.to_string(), format!("window-{n}")))
            .collect();
        let mut overflows: HashMap<String, Arc<AtomicU64>> = (0..9)
            .map(|n| (n.to_string(), Arc::new(AtomicU64::new(0))))
            .collect();
        let removed = prune_handles(&mut hosts, &mut overflows, String::as_str, |label| {
            !matches!(label, "window-1" | "window-3")
        });
        assert_eq!(removed.len(), 2);
        assert_eq!(hosts.len(), 6);
        assert_eq!(overflows.len(), 6);
        assert!(hosts.contains_key("7")); // Still-open login/paused windows are never expired away.
        hosts.insert("8".into(), "window-8".into());
        hosts.insert("9".into(), "window-9".into());
        assert_eq!(hosts.len(), 8);
    }
}
