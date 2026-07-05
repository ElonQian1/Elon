use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::{command as launcher_command, log_file, paths};

pub(crate) fn open_recovery_page(
    install_dir: &Path,
    port: u16,
    reason: &str,
    detail: &str,
) -> Result<()> {
    let page = recovery_page_path(install_dir);
    if let Some(parent) = page.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create recovery page directory {}", parent.display()))?;
    }
    let html = build_recovery_html(port, reason, detail);
    std::fs::write(&page, html)
        .with_context(|| format!("write local recovery page {}", page.display()))?;
    open_file(&page).with_context(|| format!("open local recovery page {}", page.display()))?;
    log_file::record_event(
        install_dir,
        "launcher_opened_recovery_page",
        true,
        &format!("port={port}; reason={reason}; path={}", page.display()),
    );
    Ok(())
}

fn recovery_page_path(install_dir: &Path) -> PathBuf {
    paths::internal_dir(install_dir).join("pc-recovery.html")
}

fn build_recovery_html(port: u16, reason: &str, detail: &str) -> String {
    let pc_url = format!("http://127.0.0.1:{port}/pc");
    let status_url = format!("http://127.0.0.1:{port}/api/status");
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta http-equiv="refresh" content="8;url={pc_url}" />
  <title>一龙工作台正在恢复</title>
  <style>
    :root {{ color-scheme: light; font-family: "Microsoft YaHei", "Segoe UI", Arial, sans-serif; }}
    body {{ margin: 0; min-height: 100vh; display: grid; place-items: center; background: #f7f8fa; color: #18202f; }}
    main {{ width: min(760px, calc(100vw - 32px)); background: #fff; border: 1px solid #d9dee8; border-radius: 8px; padding: 28px; box-shadow: 0 18px 50px rgba(24, 32, 47, .10); }}
    h1 {{ margin: 0 0 10px; font-size: 26px; line-height: 1.25; }}
    p {{ margin: 8px 0; line-height: 1.7; color: #465163; }}
    .status {{ margin: 18px 0; padding: 14px 16px; border-left: 4px solid #c23b3b; background: #fff6f4; color: #7b1d1d; }}
    .actions {{ display: flex; flex-wrap: wrap; gap: 10px; margin-top: 20px; }}
    a {{ border: 1px solid #c8d0dc; border-radius: 7px; padding: 10px 14px; color: #18202f; text-decoration: none; background: #fff; }}
    a.primary {{ border-color: #1f6feb; background: #1f6feb; color: #fff; }}
    code {{ background: #eef1f6; border-radius: 5px; padding: 2px 5px; }}
  </style>
</head>
<body>
  <main>
    <h1>一龙工作台正在恢复</h1>
    <p>本机 Win 端没有在预期时间内响应，启动器已经把这个状态交给后台守护层处理。</p>
    <div class="status">
      <strong>{reason}</strong>
      <p>{detail}</p>
    </div>
    <p>页面会自动重试打开 <code>{pc_url}</code>。如果仍然打不开，可以启动或修复 Win 端，并导出诊断给客服或开发者。</p>
    <div class="actions">
      <a class="primary" href="{pc_url}">重新打开本机工作台</a>
      <a href="elon-node://open">启动 Win 端</a>
      <a href="elon-node://repair">修复客户端入口</a>
      <a href="elon-node://diagnostics/export">导出诊断</a>
    </div>
    <p>本机状态接口：<code>{status_url}</code></p>
  </main>
</body>
</html>"#,
        pc_url = html_escape(&pc_url),
        status_url = html_escape(&status_url),
        reason = html_escape(reason),
        detail = html_escape(detail)
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn open_file(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        let script = format!(
            "Start-Process -FilePath '{}'",
            launcher_command::ps_single_quote(&path.to_string_lossy())
        );
        let mut command = launcher_command::powershell_hidden_command(&script);
        let status = launcher_command::status_hidden(&mut command)
            .context("failed to open recovery page via PowerShell")?;
        if !status.success() {
            anyhow::bail!("PowerShell failed to open recovery page");
        }
    }
    #[cfg(not(windows))]
    {
        let mut command = launcher_command::silent_command("xdg-open");
        command.arg(path);
        launcher_command::spawn_hidden(&mut command)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_recovery_html;

    #[test]
    fn recovery_page_points_to_local_pc_and_maintenance_protocols() {
        let html = build_recovery_html(7799, "admin_timeout", "HTTP <timeout>");

        assert!(html.contains("http://127.0.0.1:7799/pc"));
        assert!(html.contains("elon-node://repair"));
        assert!(html.contains("elon-node://diagnostics/export"));
        assert!(html.contains("HTTP &lt;timeout&gt;"));
        assert!(!html.contains("HTTP <timeout>"));
    }
}
