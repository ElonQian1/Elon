//! Ephemeral routing metadata only; native hosts retain sessions and credentials.
use super::contract::{identifier, ResearchCommand, ResearchResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) const HOST_LEASE_MS: u64 = 15_000;
const BINDING_TTL_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnedSession {
    pub project_key: String,
    pub session_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostInput {
    pub instance_id: String,
    pub sessions: Vec<OwnedSession>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct HostSummary {
    pub instance_id: String,
    pub expires_at_ms: u64,
    pub sessions: Vec<String>,
}

struct Host {
    expires: u64,
    sessions: Vec<OwnedSession>,
}
struct Binding {
    instance: String,
    expires: u64,
}
#[derive(Default)]
pub(crate) struct Affinity {
    hosts: HashMap<String, Host>,
    bindings: HashMap<(String, String), Binding>,
}

impl Affinity {
    fn sweep(&mut self, now: u64) {
        self.hosts.retain(|_, h| h.expires > now);
        self.bindings.retain(|_, b| b.expires > now);
    }

    pub fn register(&mut self, input: HostInput, now: u64) -> ResearchResult<()> {
        if !identifier(&input.instance_id) || input.sessions.len() > 8 {
            return Err("invalid_host");
        }
        let mut unique = std::collections::HashSet::new();
        for s in &input.sessions {
            if s.project_key.len() != 64
                || !s.project_key.bytes().all(|c| c.is_ascii_hexdigit())
                || !identifier(&s.session_id)
                || !unique.insert((&s.project_key, &s.session_id))
            {
                return Err("invalid_host");
            }
        }
        self.sweep(now);
        if self.hosts.len() >= 32 && !self.hosts.contains_key(&input.instance_id) {
            return Err("host_limit");
        }
        // Validate the whole heartbeat before mutating any binding.
        for s in &input.sessions {
            self.check_binding(&s.project_key, &s.session_id, &input.instance_id)?;
        }
        let new_count = input
            .sessions
            .iter()
            .filter(|s| {
                !self
                    .bindings
                    .contains_key(&(s.project_key.clone(), s.session_id.clone()))
            })
            .count();
        if self.bindings.len() + new_count > 4096 {
            return Err("host_limit");
        }
        for s in &input.sessions {
            self.bind(&s.project_key, &s.session_id, &input.instance_id, now)?;
        }
        self.hosts.insert(
            input.instance_id,
            Host {
                expires: now.saturating_add(HOST_LEASE_MS),
                sessions: input.sessions,
            },
        );
        Ok(())
    }

    fn check_binding(&self, project: &str, session: &str, instance: &str) -> ResearchResult<()> {
        if self
            .bindings
            .get(&(project.into(), session.into()))
            .is_some_and(|b| b.instance != instance)
        {
            return Err("host_mismatch");
        }
        Ok(())
    }

    pub fn bind(
        &mut self,
        project: &str,
        session: &str,
        instance: &str,
        now: u64,
    ) -> ResearchResult<()> {
        self.check_binding(project, session, instance)?;
        let key = (project.into(), session.into());
        if self.bindings.len() >= 4096 && !self.bindings.contains_key(&key) {
            return Err("host_limit");
        }
        self.bindings.insert(
            key,
            Binding {
                instance: instance.into(),
                expires: now.saturating_add(BINDING_TTL_MS),
            },
        );
        Ok(())
    }

    pub fn live(&self, instance: &str, now: u64) -> bool {
        self.hosts.get(instance).is_some_and(|h| h.expires > now)
    }

    pub fn route(
        &self,
        project: &str,
        command: &ResearchCommand,
        now: u64,
    ) -> ResearchResult<String> {
        if let Some(binding) = command
            .session_id
            .as_ref()
            .and_then(|s| self.bindings.get(&(project.into(), s.clone())))
            .filter(|b| b.expires > now)
        {
            if command
                .instance_id
                .as_ref()
                .is_some_and(|id| *id != binding.instance)
            {
                return Err("host_mismatch");
            }
            return if self.live(&binding.instance, now) {
                Ok(binding.instance.clone())
            } else {
                Err("host_unavailable")
            };
        }
        if let Some(id) = &command.instance_id {
            return if self.live(id, now) {
                Ok(id.clone())
            } else {
                Err("host_unavailable")
            };
        }
        let live: Vec<_> = self.hosts.iter().filter(|(_, h)| h.expires > now).collect();
        match live.as_slice() {
            [(id, _)] => Ok((*id).clone()),
            [] => Err("host_unavailable"),
            _ => Err("host_ambiguous"),
        }
    }

    pub fn summaries(&self, project: &str, now: u64) -> Vec<HostSummary> {
        let mut hosts: Vec<_> = self
            .hosts
            .iter()
            .filter(|(_, h)| h.expires > now)
            .map(|(id, h)| {
                let mut sessions: Vec<_> = h
                    .sessions
                    .iter()
                    .filter(|s| s.project_key == project)
                    .map(|s| s.session_id.clone())
                    .collect();
                sessions.sort();
                HostSummary {
                    instance_id: id.clone(),
                    expires_at_ms: h.expires,
                    sessions,
                }
            })
            .collect();
        hosts.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
        hosts
    }
}
