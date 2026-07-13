use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

pub(crate) fn write(path: &Path, contents: &[u8]) -> Result<()> {
    write_via_temporary(path, contents, replace)
}

/// Installs a fully-written file only when `path` does not exist.
///
/// This is used for ownership markers: replacing a concurrently-created
/// marker would allow two node installations to believe they both own the
/// same data root.
pub(crate) fn write_new(path: &Path, contents: &[u8]) -> Result<()> {
    write_via_temporary(path, contents, install_new)
}

fn write_via_temporary(
    path: &Path,
    contents: &[u8],
    commit: fn(&Path, &Path) -> Result<()>,
) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("原子写入目标没有父目录: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("无法创建配置目录 {}", parent.display()))?;
    reject_symlink_target(path)?;

    let temporary = temporary_path(path);
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .with_context(|| format!("无法创建临时配置文件 {}", temporary.display()))?;
        file.write_all(contents)
            .with_context(|| format!("无法写入临时配置文件 {}", temporary.display()))?;
        file.sync_all()
            .with_context(|| format!("无法同步临时配置文件 {}", temporary.display()))?;
        commit(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("node-state");
    path.with_file_name(format!(".{file_name}.tmp-{}", uuid::Uuid::new_v4()))
}

fn reject_symlink_target(path: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("无法检查原子写入目标 {}", path.display()));
        }
    };
    let mut rejected = metadata.file_type().is_symlink();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        rejected |= metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    }
    if rejected {
        bail!(
            "拒绝覆盖符号链接、junction 或重解析点配置文件: {}",
            path.display()
        );
    }
    Ok(())
}

#[cfg(windows)]
fn replace(source: &Path, target: &Path) -> Result<()> {
    move_file(source, target, true)
}

#[cfg(windows)]
fn install_new(source: &Path, target: &Path) -> Result<()> {
    move_file(source, target, false)
}

#[cfg(windows)]
fn move_file(source: &Path, target: &Path, replace_existing: bool) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let flags = MOVEFILE_WRITE_THROUGH
        | if replace_existing {
            MOVEFILE_REPLACE_EXISTING
        } else {
            0
        };
    let ok = unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), flags) };
    if ok == 0 {
        return Err(std::io::Error::last_os_error()).with_context(|| {
            if replace_existing {
                "无法原子替换配置文件"
            } else {
                "无法原子独占创建配置文件"
            }
        });
    }
    Ok(())
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(existing: *const u16, target: *const u16, flags: u32) -> i32;
}

#[cfg(not(windows))]
fn replace(source: &Path, target: &Path) -> Result<()> {
    std::fs::rename(source, target)
        .with_context(|| format!("无法原子替换配置文件 {}", target.display()))
}

#[cfg(not(windows))]
fn install_new(source: &Path, target: &Path) -> Result<()> {
    // `rename` replaces on Unix. A same-directory hard link instead gives us
    // an atomic no-clobber install of the already-written inode. Removing the
    // temporary name afterwards does not affect the committed target link.
    std::fs::hard_link(source, target)
        .with_context(|| format!("无法原子独占创建配置文件 {}", target.display()))?;
    let _ = std::fs::remove_file(source);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_new_never_replaces_an_existing_file() {
        let root =
            std::env::temp_dir().join(format!("elon-atomic-write-new-{}", uuid::Uuid::new_v4()));
        let path = root.join("owner.json");

        write_new(&path, b"first").expect("claim file");
        let error = write_new(&path, b"second").expect_err("second claim must fail");

        assert!(error.to_string().contains("独占创建"));
        assert_eq!(std::fs::read(&path).expect("read claimed file"), b"first");
        let _ = std::fs::remove_dir_all(root);
    }
}
