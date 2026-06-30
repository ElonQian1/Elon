// server/src/node_agent_cli_pty.rs

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;

const DEFAULT_COLS: u16 = 120;
const DEFAULT_ROWS: u16 = 30;

#[derive(Debug)]
pub(crate) enum CliPtyEvent {
    Output(String),
    ReaderError(String),
    ReaderClosed,
}

pub(crate) struct CliPtyProcess {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    output_rx: Option<mpsc::UnboundedReceiver<CliPtyEvent>>,
}

impl CliPtyProcess {
    pub(crate) fn spawn(config: CliPtySpawnConfig<'_>) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(pty_size(config.cols, config.rows))
            .context("创建 PTY/ConPTY 会话")?;
        let mut command = CommandBuilder::new(config.program);
        command.args(config.args);
        if let Some(cwd) = config.cwd.filter(|value| !value.trim().is_empty()) {
            command.cwd(cwd);
        }
        for (key, value) in config.env {
            command.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(command)
            .with_context(|| format!("在 PTY/ConPTY 中启动 {}", config.program))?;
        let reader = pair.master.try_clone_reader().context("克隆 PTY 读端")?;
        let writer = pair.master.take_writer().context("取得 PTY 写端")?;
        drop(pair.slave);

        let (tx, output_rx) = mpsc::unbounded_channel();
        spawn_reader_thread(reader, tx);

        Ok(Self {
            master: pair.master,
            child,
            writer: Arc::new(Mutex::new(writer)),
            output_rx: Some(output_rx),
        })
    }

    pub(crate) fn child_pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    pub(crate) fn try_wait(&mut self) -> Result<Option<bool>> {
        self.child
            .try_wait()
            .map(|status| status.map(|status| status.success()))
            .context("检查 PTY 子进程状态")
    }

    pub(crate) fn kill(&mut self) -> Result<()> {
        self.child.kill().context("停止 PTY 子进程")
    }

    pub(crate) fn write_input(&self, text: &str) -> Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("PTY 写端锁已损坏"))?;
        writer.write_all(text.as_bytes()).context("写入 PTY 输入")?;
        writer.flush().context("刷新 PTY 输入")
    }

    pub(crate) fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(pty_size(cols, rows))
            .context("调整 PTY/ConPTY 尺寸")
    }

    pub(crate) fn take_output_rx(&mut self) -> mpsc::UnboundedReceiver<CliPtyEvent> {
        self.output_rx
            .take()
            .expect("PTY output receiver should only be taken once")
    }
}

pub(crate) struct CliPtySpawnConfig<'a> {
    pub(crate) program: &'a str,
    pub(crate) args: &'a [String],
    pub(crate) cwd: Option<&'a str>,
    pub(crate) env: &'a [(String, String)],
    pub(crate) cols: u16,
    pub(crate) rows: u16,
}

pub(crate) fn default_cols() -> u16 {
    DEFAULT_COLS
}

pub(crate) fn default_rows() -> u16 {
    DEFAULT_ROWS
}

fn pty_size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        cols: cols.max(20),
        rows: rows.max(5),
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn spawn_reader_thread(mut reader: Box<dyn Read + Send>, tx: mpsc::UnboundedSender<CliPtyEvent>) {
    std::thread::Builder::new()
        .name("elon-cli-pty-reader".to_string())
        .spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&buffer[..bytes]).to_string();
                        if tx.send(CliPtyEvent::Output(text)).is_err() {
                            return;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        let _ = tx.send(CliPtyEvent::ReaderError(error.to_string()));
                        break;
                    }
                }
            }
            let _ = tx.send(CliPtyEvent::ReaderClosed);
        })
        .expect("PTY reader thread should start");
}
