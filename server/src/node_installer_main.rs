#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    time::{SystemTime, UNIX_EPOCH},
};

const PAYLOAD_MAGIC: &[u8; 32] = b"ELON_NODE_INSTALLER_PAYLOAD_V1!!";
const PAYLOAD_HASH_LEN: usize = 32;
const PAYLOAD_LENGTH_LEN: usize = 8;
const FOOTER_LEN: usize = PAYLOAD_MAGIC.len() + PAYLOAD_LENGTH_LEN + PAYLOAD_HASH_LEN;
const FOOTER_SEARCH_LIMIT: u64 = 4 * 1024 * 1024;
const CLIENT_FILE_NAME: &str = "一龙开发平台.exe";

#[derive(Debug, Clone, PartialEq, Eq)]
struct PayloadDescriptor {
    offset: u64,
    length: u64,
    sha256: [u8; PAYLOAD_HASH_LEN],
}

#[cfg(windows)]
fn main() {
    let silent = std::env::args().skip(1).any(|arg| {
        let value = arg.trim().to_ascii_lowercase();
        matches!(value.as_str(), "--silent" | "/s")
    });
    match install(silent) {
        Ok(message) => {
            if !silent {
                show_message(
                    "一龙开发平台安装程序",
                    &message,
                    windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                );
            }
        }
        Err(error) => {
            let message = format!("安装未完成。\n\n{error:#}\n\n原有项目和账号数据没有被删除。");
            if silent {
                eprintln!("{message}");
            } else {
                show_message(
                    "一龙开发平台安装失败",
                    &message,
                    windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
                );
            }
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("This installer only supports Windows.");
    std::process::exit(1);
}

#[cfg(windows)]
fn install(silent: bool) -> Result<String> {
    if !silent && !confirm_install() {
        return Ok("已取消安装，没有修改这台电脑。".to_string());
    }

    let temp = TempInstallDir::create()?;
    let archive = temp.path.join("elon-node-agent-windows.zip");
    write_embedded_payload(&archive)?;
    let extracted = temp.path.join("client");
    fs::create_dir_all(&extracted).context("无法创建安装解压目录")?;
    extract_archive(&archive, &extracted)?;

    let client = extracted.join(CLIENT_FILE_NAME);
    if !client.is_file() {
        bail!("安装包缺少 {CLIENT_FILE_NAME}");
    }
    let status = hidden_command(&client)
        .arg("--install")
        .current_dir(&extracted)
        .status()
        .with_context(|| format!("无法启动安装载荷 {}", client.display()))?;
    if !status.success() {
        bail!("客户端安装进程返回失败状态 {status}");
    }
    Ok("安装完成，一龙开发平台已经启动。".to_string())
}

fn write_embedded_payload(output: &Path) -> Result<()> {
    let current = std::env::current_exe().context("无法定位当前安装程序")?;
    let mut source =
        File::open(&current).with_context(|| format!("无法读取 {}", current.display()))?;
    let descriptor = locate_payload(&mut source)?;
    source
        .seek(SeekFrom::Start(descriptor.offset))
        .context("无法定位安装载荷")?;

    let mut target =
        File::create(output).with_context(|| format!("无法创建 {}", output.display()))?;
    let mut remaining = descriptor.length;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    while remaining > 0 {
        let requested = usize::try_from(remaining.min(buffer.len() as u64))
            .context("安装载荷长度超出支持范围")?;
        source
            .read_exact(&mut buffer[..requested])
            .context("安装载荷读取不完整")?;
        target
            .write_all(&buffer[..requested])
            .context("安装载荷写入失败")?;
        hasher.update(&buffer[..requested]);
        remaining -= requested as u64;
    }
    target.flush().context("安装载荷落盘失败")?;
    let actual: [u8; PAYLOAD_HASH_LEN] = hasher.finalize().into();
    if actual != descriptor.sha256 {
        bail!("安装载荷 SHA-256 校验失败");
    }
    Ok(())
}

fn locate_payload(file: &mut File) -> Result<PayloadDescriptor> {
    let file_len = file.metadata().context("无法读取安装程序大小")?.len();
    if file_len < FOOTER_LEN as u64 {
        bail!("安装程序没有内置客户端载荷");
    }
    let tail_len = file_len.min(FOOTER_SEARCH_LIMIT);
    file.seek(SeekFrom::End(-(tail_len as i64)))
        .context("无法定位安装程序尾部")?;
    let mut tail = vec![0_u8; tail_len as usize];
    file.read_exact(&mut tail).context("无法读取安装程序尾部")?;
    locate_payload_in_tail(&tail, file_len, tail_len).context("安装程序载荷描述缺失或已损坏")
}

fn locate_payload_in_tail(tail: &[u8], file_len: u64, tail_len: u64) -> Result<PayloadDescriptor> {
    if tail.len() < FOOTER_LEN {
        bail!("安装程序尾部过短");
    }
    for index in (0..=tail.len() - FOOTER_LEN).rev() {
        if &tail[index..index + PAYLOAD_MAGIC.len()] != PAYLOAD_MAGIC {
            continue;
        }
        let length_start = index + PAYLOAD_MAGIC.len();
        let length_end = length_start + PAYLOAD_LENGTH_LEN;
        let length = u64::from_le_bytes(
            tail[length_start..length_end]
                .try_into()
                .context("安装载荷长度无效")?,
        );
        let footer_offset = file_len - tail_len + index as u64;
        if length == 0 || length > footer_offset {
            continue;
        }
        let mut sha256 = [0_u8; PAYLOAD_HASH_LEN];
        sha256.copy_from_slice(&tail[length_end..length_end + PAYLOAD_HASH_LEN]);
        return Ok(PayloadDescriptor {
            offset: footer_offset - length,
            length,
            sha256,
        });
    }
    bail!("找不到安装载荷标记")
}

#[cfg(windows)]
fn extract_archive(archive: &Path, destination: &Path) -> Result<()> {
    let tar_status = hidden_command("tar.exe")
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(destination)
        .status();
    if matches!(tar_status, Ok(status) if status.success()) {
        return Ok(());
    }

    let script = destination
        .parent()
        .context("安装临时目录无父目录")?
        .join("expand-client.ps1");
    fs::write(
        &script,
        b"param([string]$Archive,[string]$Destination)\r\n\
$ErrorActionPreference='Stop'\r\n\
Expand-Archive -LiteralPath $Archive -DestinationPath $Destination -Force\r\n",
    )
    .context("无法准备 Windows 解压回退脚本")?;
    let powershell_status = hidden_command("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .arg("-Archive")
        .arg(archive)
        .arg("-Destination")
        .arg(destination)
        .status()
        .context("无法启动 Windows 解压组件")?;
    if !powershell_status.success() {
        bail!(
            "Windows 解压组件失败：tar={}; PowerShell={powershell_status}",
            display_status(tar_status)
        );
    }
    Ok(())
}

#[cfg(windows)]
fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    use std::os::windows::process::CommandExt;
    let mut command = Command::new(program);
    command.creation_flags(0x0800_0000);
    command
}

#[cfg(windows)]
fn confirm_install() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IDOK, MB_ICONINFORMATION, MB_OKCANCEL, MB_SETFOREGROUND,
    };
    message_box(
        "一龙开发平台安装程序",
        "将为当前 Windows 用户安装一龙开发平台，无需管理员权限。\n\n\
安装不会移动或删除已有项目、缓存和账号数据。",
        MB_OKCANCEL | MB_ICONINFORMATION | MB_SETFOREGROUND,
    ) == IDOK
}

#[cfg(windows)]
fn show_message(title: &str, text: &str, icon: u32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_OK, MB_SETFOREGROUND};
    let _ = message_box(title, text, MB_OK | icon | MB_SETFOREGROUND);
}

#[cfg(windows)]
fn message_box(title: &str, text: &str, style: u32) -> i32 {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;
    let title: Vec<u16> = std::ffi::OsStr::new(title)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let text: Vec<u16> = std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), style) }
}

fn display_status(status: std::io::Result<ExitStatus>) -> String {
    match status {
        Ok(value) => value.to_string(),
        Err(error) => error.to_string(),
    }
}

struct TempInstallDir {
    path: PathBuf,
}

impl TempInstallDir {
    fn create() -> Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "elon-node-installer-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path)
            .with_context(|| format!("无法创建安装临时目录 {}", path.display()))?;
        Ok(Self { path })
    }
}

impl Drop for TempInstallDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_locator_tolerates_authenticode_trailing_bytes() {
        let stub = b"MZ-stub";
        let payload = b"zip-payload";
        let hash: [u8; 32] = Sha256::digest(payload).into();
        let mut bytes = stub.to_vec();
        bytes.extend_from_slice(payload);
        bytes.extend_from_slice(PAYLOAD_MAGIC);
        bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&hash);
        bytes.extend_from_slice(b"fake-authenticode-trailer");

        let descriptor =
            locate_payload_in_tail(&bytes, bytes.len() as u64, bytes.len() as u64).unwrap();
        assert_eq!(descriptor.offset, stub.len() as u64);
        assert_eq!(descriptor.length, payload.len() as u64);
        assert_eq!(descriptor.sha256, hash);
    }

    #[test]
    fn footer_locator_rejects_missing_payload() {
        let error =
            locate_payload_in_tail(&vec![0; FOOTER_LEN], FOOTER_LEN as u64, FOOTER_LEN as u64)
                .unwrap_err();
        assert!(error.to_string().contains("找不到安装载荷标记"));
    }
}
