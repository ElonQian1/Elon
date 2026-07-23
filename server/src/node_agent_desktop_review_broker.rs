//! Ephemeral Desktop review signer isolated behind an attested local pipe.
//!
//! The private key exists only in the NodeAgent process.  A same-SID PC
//! executor can open the public pipe, but cannot obtain a signature because
//! the server resolves the real pipe client PID and requires a trusted Codex
//! Desktop package ancestor while rejecting every Elon executor ancestor.

use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rsa::{
    pkcs1v15::SigningKey,
    rand_core::OsRng,
    signature::{SignatureEncoding, Signer},
    traits::PublicKeyParts,
    RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing::warn;

pub(crate) const CAPABILITY: &str = "desktop_review_broker_v1";
const PROTOCOL: &str = "elon.desktop_review_broker.v1";
const MAX_REQUEST_BYTES: usize = 16 * 1024;

#[derive(Clone)]
pub(crate) struct DesktopReviewBroker {
    inner: Option<Arc<BrokerInner>>,
    unavailable_reason: Option<&'static str>,
}

struct BrokerInner {
    pipe_name: String,
    key_id: String,
    private_key: RsaPrivateKey,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignRequest {
    protocol: String,
    owner_user_id: String,
    task_id: String,
    method: String,
    endpoint_path: String,
    body_sha256: String,
}

#[derive(Debug, Serialize)]
struct SignResponse<'a> {
    ok: bool,
    code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    ticket: Option<String>,
}

#[derive(Clone, Debug)]
struct ProcessIdentity {
    pid: u32,
    parent_pid: u32,
    name: String,
    image_path: Option<String>,
}

impl DesktopReviewBroker {
    pub(crate) fn initialize(install_id: &str) -> Self {
        #[cfg(all(windows, not(test)))]
        {
            return match Self::generate(install_id) {
                Ok(broker) => broker,
                Err(error) => {
                    warn!(%error, "Desktop review broker key generation failed closed");
                    Self {
                        inner: None,
                        unavailable_reason: Some("ephemeral_key_generation_failed"),
                    }
                }
            };
        }
        #[cfg(any(not(windows), test))]
        {
            let _ = install_id;
            Self {
                inner: None,
                unavailable_reason: Some("windows_named_pipe_required"),
            }
        }
    }

    fn generate(install_id: &str) -> anyhow::Result<Self> {
        let private_key = RsaPrivateKey::new(&mut OsRng, 3072)?;
        let public_key = private_key.to_public_key();
        let key_id = public_key_id(&public_key);
        let install_hash = format!("{:x}", Sha256::digest(install_id.as_bytes()));
        Ok(Self {
            inner: Some(Arc::new(BrokerInner {
                pipe_name: format!("elon-desktop-review-{}", &install_hash[..24]),
                key_id,
                private_key,
            })),
            unavailable_reason: None,
        })
    }

    pub(crate) fn verifier(&self) -> Option<(String, RsaPublicKey)> {
        self.inner
            .as_ref()
            .map(|inner| (inner.key_id.clone(), inner.private_key.to_public_key()))
    }

    pub(crate) fn status_payload(&self) -> Value {
        match self.inner.as_ref() {
            Some(inner) => json!({
                "protocol": PROTOCOL,
                "available": true,
                "transport": "windows_named_pipe",
                "pipe_name": inner.pipe_name,
                "caller_policy": "trusted_codex_desktop_ancestry",
                "private_key_persistence": "memory_only",
                "same_sid_executor_direct_signing": "denied"
            }),
            None => json!({
                "protocol": PROTOCOL,
                "available": false,
                "reason": self.unavailable_reason.unwrap_or("unavailable")
            }),
        }
    }

    pub(crate) fn spawn(&self) {
        #[cfg(windows)]
        if let Some(inner) = self.inner.clone() {
            tokio::spawn(async move {
                if let Err(error) = windows_pipe::serve(inner).await {
                    warn!(%error, "Desktop review broker stopped; reviews fail closed");
                }
            });
        }
    }
}

impl BrokerInner {
    fn sign(&self, request: &SignRequest) -> Result<String, &'static str> {
        if request.protocol != PROTOCOL
            || request.owner_user_id.trim().is_empty()
            || request.owner_user_id.len() > 256
            || request.task_id.trim().is_empty()
            || request.task_id.len() > 256
            || request.method != "POST"
            || request.endpoint_path
                != crate::node_agent_desktop_review_auth::endpoint_path(&request.task_id)
            || request.body_sha256.len() != 64
            || !request
                .body_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("desktop_review_broker_request_invalid");
        }
        let expires = now_secs().saturating_add(120);
        let mut nonce = [0u8; 24];
        rsa::rand_core::RngCore::fill_bytes(&mut OsRng, &mut nonce);
        let nonce = BASE64
            .encode(nonce)
            .trim_end_matches('=')
            .replace('+', "-")
            .replace('/', "_");
        let message = crate::node_agent_desktop_review_auth::ticket_message_v3(
            &request.owner_user_id,
            &request.task_id,
            &request.method,
            &request.endpoint_path,
            &request.body_sha256.to_ascii_lowercase(),
            expires,
            &nonce,
            &self.key_id,
        );
        let signature = SigningKey::<Sha256>::new(self.private_key.clone())
            .sign(message.as_bytes())
            .to_bytes();
        Ok(format!(
            "v3.{}.{}.{}.{}",
            self.key_id,
            expires,
            nonce,
            BASE64.encode(signature)
        ))
    }
}

fn public_key_id(key: &RsaPublicKey) -> String {
    let digest = Sha256::digest(key.n().to_bytes_be());
    hex::encode(&digest[..8])
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn trusted_desktop_ancestry(
    client_pid: u32,
    processes: &[ProcessIdentity],
) -> Result<(), &'static str> {
    let by_pid = processes
        .iter()
        .map(|process| (process.pid, process))
        .collect::<std::collections::HashMap<_, _>>();
    let mut pid = client_pid;
    let mut seen = std::collections::HashSet::new();
    for _ in 0..24 {
        if pid == 0 || !seen.insert(pid) {
            break;
        }
        let process = by_pid
            .get(&pid)
            .ok_or("desktop_review_caller_process_unavailable")?;
        let name = process.name.to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "elon-cli-worker.exe" | "elon-pc-node.exe" | "一龙开发平台.exe"
        ) {
            return Err("desktop_review_executor_ancestry_denied");
        }
        if name == "chatgpt.exe"
            && process
                .image_path
                .as_deref()
                .is_some_and(is_trusted_codex_path)
        {
            return Ok(());
        }
        pid = process.parent_pid;
    }
    Err("desktop_review_caller_not_codex_desktop")
}

fn is_trusted_codex_path(path: &str) -> bool {
    let path = path.replace('/', "\\").to_ascii_lowercase();
    path.contains("\\program files\\windowsapps\\openai.codex_")
        && path.ends_with("\\app\\chatgpt.exe")
}

#[cfg(windows)]
mod windows_pipe {
    use std::{mem::size_of, os::windows::io::AsRawHandle};

    use anyhow::{Context, Result};
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::windows::named_pipe::{NamedPipeServer, ServerOptions},
        time::{timeout, Duration},
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Pipes::GetNamedPipeClientProcessId,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
    };

    use super::*;

    pub(super) async fn serve(inner: Arc<BrokerInner>) -> Result<()> {
        let pipe_path = format!(r"\\.\pipe\{}", inner.pipe_name);
        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&pipe_path)
            .with_context(|| format!("create Desktop review broker pipe {}", inner.pipe_name))?;
        loop {
            server.connect().await?;
            let connected = server;
            server = ServerOptions::new()
                .reject_remote_clients(true)
                .create(&pipe_path)?;
            let signer = inner.clone();
            tokio::spawn(async move {
                if let Err(error) = handle_client(connected, signer).await {
                    warn!(%error, "Desktop review broker request failed closed");
                }
            });
        }
    }

    async fn handle_client(server: NamedPipeServer, signer: Arc<BrokerInner>) -> Result<()> {
        let client_pid = pipe_client_pid(&server)?;
        let processes = process_snapshot()?;
        let trust = trusted_desktop_ancestry(client_pid, &processes);
        let mut reader = BufReader::new(server);
        let mut line = String::new();
        let bytes = timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .context("Desktop review broker request timed out")??;
        let response = if bytes == 0 || bytes > MAX_REQUEST_BYTES || line.len() > MAX_REQUEST_BYTES
        {
            SignResponse {
                ok: false,
                code: "desktop_review_broker_request_invalid",
                ticket: None,
            }
        } else if let Err(code) = trust {
            SignResponse {
                ok: false,
                code,
                ticket: None,
            }
        } else {
            match serde_json::from_str::<SignRequest>(line.trim_end()) {
                Ok(request) => match signer.sign(&request) {
                    Ok(ticket) => SignResponse {
                        ok: true,
                        code: "desktop_review_ticket_minted",
                        ticket: Some(ticket),
                    },
                    Err(code) => SignResponse {
                        ok: false,
                        code,
                        ticket: None,
                    },
                },
                Err(_) => SignResponse {
                    ok: false,
                    code: "desktop_review_broker_request_invalid",
                    ticket: None,
                },
            }
        };
        let mut server = reader.into_inner();
        let mut payload = serde_json::to_vec(&response)?;
        payload.push(b'\n');
        server.write_all(&payload).await?;
        server.shutdown().await?;
        Ok(())
    }

    fn pipe_client_pid(server: &NamedPipeServer) -> Result<u32> {
        let mut pid = 0u32;
        let ok = unsafe {
            GetNamedPipeClientProcessId(server.as_raw_handle() as _, &mut pid as *mut u32)
        };
        anyhow::ensure!(ok != 0 && pid != 0, "resolve named-pipe client PID");
        Ok(pid)
    }

    fn process_snapshot() -> Result<Vec<ProcessIdentity>> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        anyhow::ensure!(snapshot != INVALID_HANDLE_VALUE, "create process snapshot");
        let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut processes = Vec::new();
        let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
        while ok != 0 {
            let name = wide_string(&entry.szExeFile);
            let image_path = if name.eq_ignore_ascii_case("ChatGPT.exe") {
                query_image_path(entry.th32ProcessID)
            } else {
                None
            };
            processes.push(ProcessIdentity {
                pid: entry.th32ProcessID,
                parent_pid: entry.th32ParentProcessID,
                name,
                image_path,
            });
            ok = unsafe { Process32NextW(snapshot, &mut entry) };
        }
        unsafe { CloseHandle(snapshot) };
        Ok(processes)
    }

    fn query_image_path(pid: u32) -> Option<String> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return None;
        }
        let mut buffer = vec![0u16; 32_768];
        let mut size = buffer.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size) };
        unsafe { CloseHandle(process) };
        (ok != 0).then(|| String::from_utf16_lossy(&buffer[..size as usize]))
    }

    fn wide_string(value: &[u16]) -> String {
        let len = value.iter().position(|ch| *ch == 0).unwrap_or(value.len());
        String::from_utf16_lossy(&value[..len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, parent_pid: u32, name: &str, path: Option<&str>) -> ProcessIdentity {
        ProcessIdentity {
            pid,
            parent_pid,
            name: name.to_string(),
            image_path: path.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn trusted_desktop_chain_is_accepted_but_same_sid_executor_chain_is_denied() {
        let desktop = vec![
            process(10, 20, "powershell.exe", None),
            process(20, 30, "codex-code-mode-host.exe", None),
            process(30, 40, "codex.exe", None),
            process(
                40,
                1,
                "ChatGPT.exe",
                Some(
                    r"C:\Program Files\WindowsApps\OpenAI.Codex_26.715.1.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe",
                ),
            ),
        ];
        assert_eq!(trusted_desktop_ancestry(10, &desktop), Ok(()));

        let executor = vec![
            process(10, 20, "powershell.exe", None),
            process(20, 30, "codex.exe", None),
            process(30, 40, "elon-cli-worker.exe", None),
            process(40, 1, "一龙开发平台.exe", None),
        ];
        assert_eq!(
            trusted_desktop_ancestry(10, &executor),
            Err("desktop_review_executor_ancestry_denied")
        );
    }

    #[test]
    fn lookalike_or_missing_desktop_package_fails_closed() {
        for path in [
            r"C:\Temp\OpenAI.Codex_fake\app\ChatGPT.exe",
            r"C:\Program Files\WindowsApps\Other.App_1.0\app\ChatGPT.exe",
        ] {
            let chain = vec![process(10, 0, "ChatGPT.exe", Some(path))];
            assert_eq!(
                trusted_desktop_ancestry(10, &chain),
                Err("desktop_review_caller_not_codex_desktop")
            );
        }
    }

    #[test]
    fn broker_signature_is_v3_and_bound_to_canonical_review_path() {
        let broker = DesktopReviewBroker::generate("install-test").unwrap();
        let inner = broker.inner.unwrap();
        let body = br#"{"verdict":"accepted","summary":"independently verified"}"#;
        let request = SignRequest {
            protocol: PROTOCOL.to_string(),
            owner_user_id: "owner".to_string(),
            task_id: "local-test".to_string(),
            method: "POST".to_string(),
            endpoint_path: "/api/local-tasks/local-test/supervision/desktop-review".to_string(),
            body_sha256: hex::encode(Sha256::digest(body)),
        };
        let ticket = inner.sign(&request).unwrap();
        assert!(ticket.starts_with(&format!("v3.{}.", inner.key_id)));
        let ledger = std::env::temp_dir().join(format!(
            "elon-review-broker-ledger-{}.json",
            uuid::Uuid::new_v4()
        ));
        let auth = crate::node_agent_desktop_review_auth::DesktopReviewAuth::for_v3_test_key(
            &inner.key_id,
            inner.private_key.to_public_key(),
            ledger.clone(),
        );
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            crate::node_agent_desktop_review_auth::DESKTOP_REVIEW_TICKET_HEADER,
            ticket.parse().unwrap(),
        );
        assert_eq!(
            auth.verify_and_consume(
                &headers,
                "owner",
                "local-test",
                "POST",
                "/api/local-tasks/local-test/supervision/desktop-review",
                b"{}",
            ),
            Err(crate::node_agent_desktop_review_auth::DesktopReviewAuthError::Invalid)
        );
        assert_eq!(
            auth.verify_and_consume(
                &headers,
                "owner",
                "local-test",
                "POST",
                "/api/local-tasks/local-test/supervision/desktop-review",
                body,
            ),
            Ok(())
        );
        let _ = std::fs::remove_file(ledger);
        let mut changed = request;
        changed.endpoint_path = "/other".to_string();
        assert_eq!(
            inner.sign(&changed),
            Err("desktop_review_broker_request_invalid")
        );
    }
}
