use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::adb_path::adb_path;

#[derive(Debug, Clone)]
pub(crate) struct AdbOutput {
    pub stdout: Vec<u8>,
}

pub(crate) async fn run_adb(
    args: &[String],
    timeout_duration: Duration,
    max_stdout_bytes: usize,
) -> Result<AdbOutput> {
    let mut command = Command::new(adb_path());
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::node_agent_exec::hide_tokio_command_window(&mut command);

    let output = tokio::time::timeout(timeout_duration, command.output())
        .await
        .with_context(|| format!("adb 命令超时: {}", args.join(" ")))?
        .with_context(|| format!("无法启动 adb: {}", args.join(" ")))?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.stdout.len() > max_stdout_bytes {
        bail!(
            "adb stdout 超过限制: {} > {}",
            output.stdout.len(),
            max_stdout_bytes
        );
    }
    if !output.status.success() {
        bail!(
            "adb 命令失败 exit={:?}: {}",
            output.status.code(),
            if stderr.is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                stderr.clone()
            }
        );
    }
    Ok(AdbOutput {
        stdout: output.stdout,
    })
}

pub(crate) async fn run_adb_text(
    args: &[String],
    timeout_duration: Duration,
    max_stdout_bytes: usize,
) -> Result<String> {
    let output = run_adb(args, timeout_duration, max_stdout_bytes).await?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(crate) async fn run_adb_text_with_stdin(
    args: &[String],
    input: &str,
    timeout_duration: Duration,
    max_stdout_bytes: usize,
    action: &str,
) -> Result<String> {
    let mut command = Command::new(adb_path());
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::node_agent_exec::hide_tokio_command_window(&mut command);

    let mut child = command
        .spawn()
        .with_context(|| format!("无法启动 adb: {action}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("无法打开 adb 标准输入: {action}"))?;
    stdin
        .write_all(format!("{}\n", input.trim()).as_bytes())
        .await
        .with_context(|| format!("无法写入 adb 标准输入: {action}"))?;
    drop(stdin);

    let output = tokio::time::timeout(timeout_duration, child.wait_with_output())
        .await
        .with_context(|| format!("adb 命令超时: {action}"))?
        .with_context(|| format!("adb 命令失败: {action}"))?;
    if output.stdout.len() > max_stdout_bytes {
        bail!(
            "adb stdout 超过限制: {} > {}",
            output.stdout.len(),
            max_stdout_bytes
        );
    }
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        bail!(
            "adb 命令失败 exit={:?}: {}",
            output.status.code(),
            if stderr.is_empty() { stdout } else { stderr }
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(crate) fn validate_device_id(device_id: &str) -> Result<()> {
    let value = device_id.trim();
    if value.is_empty() {
        bail!("deviceId 不能为空");
    }
    if value.len() > 128 {
        bail!("deviceId 过长");
    }
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':')))
    {
        bail!("deviceId 包含非法字符");
    }
    Ok(())
}

pub(crate) fn validate_package_name(package_name: &str) -> Result<()> {
    let value = package_name.trim();
    if value.is_empty() {
        bail!("packageName 不能为空");
    }
    if value.len() > 180 {
        bail!("packageName 过长");
    }
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')))
    {
        bail!("packageName 包含非法字符");
    }
    Ok(())
}

pub(crate) fn validate_connect_address(address: &str) -> Result<()> {
    let value = address.trim();
    if value.is_empty() {
        bail!("无线 ADB 地址不能为空");
    }
    if value.len() > 180 || value.contains('/') || value.contains('\\') || value.contains(' ') {
        bail!("无线 ADB 地址格式不合法");
    }
    let Some((host, port)) = value.rsplit_once(':') else {
        bail!("无线 ADB 地址必须是 host:port");
    };
    if host.is_empty() || port.parse::<u16>().is_err() {
        bail!("无线 ADB 地址必须是 host:port");
    }
    if host
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':')))
    {
        bail!("无线 ADB host 包含非法字符");
    }
    Ok(())
}

pub(crate) fn validate_pairing_code(code: &str) -> Result<()> {
    let value = code.trim();
    if value.len() != 6 || !value.chars().all(|ch| ch.is_ascii_digit()) {
        bail!("无线 ADB 配对码必须是 6 位数字");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_device_ids() {
        assert!(validate_device_id("e0d909c3").is_ok());
        assert!(validate_device_id("192.168.1.8:5555").is_ok());
        assert!(validate_device_id("bad id").is_err());
    }

    #[test]
    fn validates_connect_addresses() {
        assert!(validate_connect_address("192.168.1.8:5555").is_ok());
        assert!(validate_connect_address("phone.local:5555").is_ok());
        assert!(validate_connect_address("192.168.1.8").is_err());
    }

    #[test]
    fn validates_pairing_codes() {
        assert!(validate_pairing_code("123456").is_ok());
        assert!(validate_pairing_code("12345").is_err());
        assert!(validate_pairing_code("12 456").is_err());
    }
}
