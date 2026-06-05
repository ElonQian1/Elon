//! 方案 A：把 PCM 写入 PipeWire `pw-cat`，等价于"虚拟麦克风录音输入"。
//!
//! 流程：
//!   Android PCM → 本模块 spawn 的 `pw-cat --playback --raw ... --target=<sink>`
//!   → PipeWire sink → `sink.monitor` → `module-remap-source` 暴露成 `codex_mic`
//!
//! 注意（必读）：
//!   当前 Codex CLI Linux TUI 不会从 codex_mic 真正采集（`start_realtime_local_audio`
//!   在 Linux 是空函数）。本模块只负责"把音频喂到虚拟麦"这一段，是否能投喂进
//!   Codex 取决于 CLI 的语音实现。验证方法：
//!     parecord -d codex_mic.monitor /tmp/probe.wav
//!     aplay /tmp/probe.wav

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, Command},
};

use crate::voice_config::{
    VirtualMicConfig, PCM16_BYTES_PER_SAMPLE, REALTIME_CHANNELS, REALTIME_SAMPLE_RATE_HZ,
};

/// 已启动的 pw-cat 子进程及其 stdin 句柄。
pub struct PwcatHandle {
    child: Child,
    stdin: Option<ChildStdin>,
    written_bytes: u64,
}

impl PwcatHandle {
    /// 启动一个 pw-cat 子进程，把 stdin 的 PCM 播放到 `target_sink`。
    pub fn spawn(cfg: &VirtualMicConfig) -> Result<Self> {
        let mut command = Command::new(&cfg.pwcat_path);
        command
            .arg("--playback")
            .arg("--raw")
            .arg(format!("--rate={}", REALTIME_SAMPLE_RATE_HZ))
            .arg(format!("--channels={}", REALTIME_CHANNELS))
            .arg("--format=s16")
            .arg(format!("--target={}", cfg.target_sink))
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = command
            .spawn()
            .with_context(|| format!("启动 pw-cat 失败：{}", cfg.pwcat_path))?;
        let stdin = child.stdin.take().context("pw-cat 未能打开 stdin")?;

        Ok(Self {
            child,
            stdin: Some(stdin),
            written_bytes: 0,
        })
    }

    /// 写入一段 PCM16 字节。要求长度是偶数。
    pub async fn write_pcm(&mut self, pcm: &[u8]) -> Result<()> {
        let stdin = self.stdin.as_mut().context("pw-cat stdin 已关闭")?;
        stdin
            .write_all(pcm)
            .await
            .context("写入 pw-cat stdin 失败")?;
        self.written_bytes += pcm.len() as u64;
        Ok(())
    }

    /// 写入静音补尾，让 Codex 更容易判断"用户说完了"。
    pub async fn write_silence_ms(&mut self, ms: u64) -> Result<()> {
        if ms == 0 {
            return Ok(());
        }
        let samples =
            (REALTIME_SAMPLE_RATE_HZ as u64 * ms / 1000) as usize * REALTIME_CHANNELS as usize;
        let buf = vec![0u8; samples * PCM16_BYTES_PER_SAMPLE];
        self.write_pcm(&buf).await
    }

    pub fn written_bytes(&self) -> u64 {
        self.written_bytes
    }

    /// 关闭 stdin 并等待子进程退出。
    pub async fn shutdown(mut self) {
        if let Some(mut stdin) = self.stdin.take() {
            let _ = stdin.shutdown().await;
        }
        let _ = self.child.wait().await;
    }
}
