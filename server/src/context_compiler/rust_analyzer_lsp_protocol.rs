use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use serde_json::{json, Value};

pub(crate) enum LspRequestStatus {
    Succeeded(Value),
    Failed(String),
    TimedOut,
}

pub(crate) struct RustAnalyzerLspClient {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
    stderr_handle: Option<thread::JoinHandle<String>>,
    next_id: u64,
}

impl RustAnalyzerLspClient {
    pub(crate) fn start(root: &Path) -> Result<Self, String> {
        let mut child = Command::new("rust-analyzer")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to spawn rust-analyzer: {error}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to open rust-analyzer stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "failed to open rust-analyzer stdout".to_string())?;
        let stderr = child.stderr.take();
        let (sender, messages) = mpsc::channel();
        thread::spawn(move || read_lsp_messages(stdout, sender));
        let stderr_handle = stderr.map(|stderr| thread::spawn(move || read_pipe(stderr)));

        Ok(Self {
            child,
            stdin,
            messages,
            stderr_handle,
            next_id: 1,
        })
    }

    pub(crate) fn initialize(
        &mut self,
        root_uri: &str,
        root_name: &str,
        timeout: Duration,
    ) -> LspRequestStatus {
        let params = json!({
            "processId": null,
            "rootUri": root_uri,
            "workspaceFolders": [{
                "uri": root_uri,
                "name": root_name,
            }],
            "capabilities": {
                "textDocument": {
                    "callHierarchy": { "dynamicRegistration": false },
                    "hover": { "contentFormat": ["markdown", "plaintext"] }
                },
                "workspace": { "workspaceFolders": true }
            },
            "initializationOptions": {
                "cargo": {
                    "allTargets": true,
                    "buildScripts": { "enable": false }
                },
                "procMacro": { "enable": false },
                "checkOnSave": false
            }
        });
        let status = self.request("initialize", params, timeout);
        if matches!(status, LspRequestStatus::Succeeded(_)) {
            let _ = self.notification("initialized", json!({}));
        }
        status
    }

    pub(crate) fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> LspRequestStatus {
        let id = self.next_request_id();
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        if let Err(error) = write_lsp_message(&mut self.stdin, &message) {
            return LspRequestStatus::Failed(format!("failed to write LSP request: {error}"));
        }

        let deadline = Instant::now() + timeout;
        loop {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return LspRequestStatus::TimedOut;
            };
            match self.messages.recv_timeout(remaining) {
                Ok(message) if is_response_for(&message, id) => {
                    if let Some(error) = message.get("error") {
                        return LspRequestStatus::Failed(compact(&error.to_string(), 280));
                    }
                    return LspRequestStatus::Succeeded(
                        message.get("result").cloned().unwrap_or(Value::Null),
                    );
                }
                Ok(message) if is_server_request(&message) => {
                    let _ = self.respond_to_server_request(&message);
                }
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => return LspRequestStatus::TimedOut,
                Err(RecvTimeoutError::Disconnected) => {
                    return LspRequestStatus::Failed(
                        "rust-analyzer LSP stdout closed before response".to_string(),
                    );
                }
            }
        }
    }

    pub(crate) fn notification(&mut self, method: &str, params: Value) -> Result<(), String> {
        write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
            }),
        )
        .map_err(|error| error.to_string())
    }

    pub(crate) fn shutdown(&mut self, timeout: Duration) {
        let _ = self.request("shutdown", Value::Null, timeout);
        let _ = self.notification("exit", Value::Null);
        let wait_started = Instant::now();
        while wait_started.elapsed() < timeout {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => thread::sleep(Duration::from_millis(30)),
                Err(_) => return,
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    pub(crate) fn stderr_excerpt(&mut self) -> Vec<String> {
        self.stderr_handle
            .take()
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .take(4)
            .map(|line| compact(line, 220))
            .collect()
    }

    fn next_request_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn respond_to_server_request(&mut self, message: &Value) -> Result<(), String> {
        let Some(id) = message.get("id").cloned() else {
            return Ok(());
        };
        write_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": null,
            }),
        )
        .map_err(|error| error.to_string())
    }
}

impl Drop for RustAnalyzerLspClient {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn write_lsp_message(stdin: &mut ChildStdin, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_string(value)?;
    write!(stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    stdin.flush()
}

fn read_lsp_messages<R: Read>(pipe: R, sender: mpsc::Sender<Value>) {
    let mut reader = BufReader::new(pipe);
    loop {
        let Some(content_length) = read_content_length(&mut reader) else {
            break;
        };
        let mut body = vec![0u8; content_length];
        if reader.read_exact(&mut body).is_err() {
            break;
        }
        let Ok(text) = String::from_utf8(body) else {
            continue;
        };
        if let Ok(message) = serde_json::from_str::<Value>(&text) {
            if sender.send(message).is_err() {
                break;
            }
        }
    }
}

fn read_content_length<R: BufRead>(reader: &mut R) -> Option<usize> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).ok()?;
        if read == 0 {
            return None;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            return content_length;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
}

fn read_pipe<R: Read>(mut pipe: R) -> String {
    let mut output = String::new();
    let _ = pipe.read_to_string(&mut output);
    output
}

fn is_response_for(message: &Value, id: u64) -> bool {
    message
        .get("id")
        .and_then(Value::as_u64)
        .map(|value| value == id)
        .unwrap_or(false)
        && (message.get("result").is_some() || message.get("error").is_some())
}

fn is_server_request(message: &Value) -> bool {
    message.get("id").is_some()
        && message.get("method").and_then(Value::as_str).is_some()
        && message.get("result").is_none()
        && message.get("error").is_none()
}

fn compact(value: &str, max_chars: usize) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = single_line.chars().take(max_chars).collect::<String>();
    if single_line.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
