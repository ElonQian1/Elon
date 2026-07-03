## 当前项目实现状态（2026-07-03）

项目已经支持五条 PC 节点 AI 运行路线。更准确的架构应分成三层，避免把“模型从哪里来”“CLI 怎么接入”“前端怎么展示过程”混在一起：

| 层级 | 决定什么 | 当前状态 |
|---|---|---|
| 1. 运行路线 | 模型 / AI 从哪里来、项目在哪台电脑执行 | 已有 Route A/B/C1/C2/C3 |
| 2. CLI 会话 / 传输模式 | node-agent 如何启动、连接、取消和恢复 CLI | Codex 默认 `direct_json_pipe`；`pipe_sidecar` 是目标形态，当前未独立实现；`pty_sidecar` 保留给终端接管 |
| 3. 前端展示 / 恢复模式 | 网页端如何展示公开过程、折叠最终回复、恢复或接管任务 | 结构化过程卡片读 JSON 事件和 journal；终端 attach 读 PTY sidecar |

运行路线只决定“模型 / AI 从哪里来、项目在哪台电脑执行”，不要和“是否打开 PTY 终端接管”混在一起：

| 路线 | 前端值 | AI / 模型来源 | 项目文件和命令执行位置 | 当前定位 |
|---|---|---|---|---|
| 路线 A：本机AI | `route_a` | 项目绑定 PC 上已登录的 Codex / Claude / Gemini / Copilot CLI | 项目绑定 PC 节点 | 项目会话默认优先；适合 owner 自己电脑已准备好 CLI |
| 路线 B：我的Key | `route_b` | 项目绑定 PC 上配置的 OpenAI-compatible API key | 项目绑定 PC 节点的一龙工具 runtime | 用户自带模型 key，仍在本机安全执行文件/命令 |
| 路线 C1：平台AI | `route_c` / `route_c1` | 一龙平台提供模型能力 | 项目绑定 PC 节点的一龙工具 runtime | 用户没有 CLI/key 时兜底；文件读写和命令不搬到服务器 |
| 路线 C2：远程AI | `route_c2` | 其他用户 PC 节点的 API runtime | 被授权的远程 PC 节点 | 借用远程电脑和它的 API key |
| 路线 C3：远程Codex | `route_c3` | 其他用户 PC 节点已登录的 Codex / Claude / Copilot 等 CLI | 被授权的远程 PC 节点 | 借用远程电脑上的专业 CLI |

PC 项目会话默认值是 `route_a`。PC 前端“强制 Codex / 直连”开关会把本轮请求强制成 `route_a`，并传入 `localNodeId` 与项目 `workspacePath`；关闭该开关时，按当前运行路线选择器走 `route_a` / `route_b` / `route_c` / `route_c2` / `route_c3`。

Route A 本机 CLI 是否使用 PTY 是第二层传输模式选择，不是新的路线。Route A / Route C3 都可以在各自节点内选择 `direct_json_pipe`、未来 `pipe_sidecar` 或 `pty_sidecar`。

CLI 会话 / 传输模式当前这样定位：

| 模式 | 当前是否具备 | 定位 |
|---|---|---|
| `direct_json_pipe` | 已具备，Codex 默认 | node-agent 直接启动 `codex exec --json`，读取干净 stdout JSONL / stderr，并把事件写入任务过程 |
| `pipe_sidecar` | 当前未独立实现，建议作为下一阶段目标 | sidecar 管进程生命周期、取消、journal、session id、恢复入口；stdout/stderr 仍走程序 pipe，不进入 PTY |
| `pty_sidecar` | 已具备，辅助路 | 用 portable_pty / ConPTY 管真实终端，适合 TUI、人工接管、resize、交互输入和终端型 CLI |

长期看，`pipe_sidecar` 比“把 Codex JSON 放进 PTY”更好：它能保留 sidecar 的生命周期管理、日志和恢复能力，同时不污染 JSONL。当前不要把它描述成已实现能力；现在 Codex 的默认后台开发主路仍是 `direct_json_pipe`，也就是 `codex exec --json` + 直接 stdout JSONL 解析：

```text
PC 网页端
  -> Rust server
  -> node-agent
  -> codex exec --json
  -> 直接读取 stdout JSONL
  -> 解析 assistant_message / tool_call / tool_result / usage / final_reply
  -> 网页端结构化过程卡片
```

这条主路不默认进入 PTY/ConPTY。原因是 `codex exec --json` 输出的是给程序消费的机器事件流，PTY 是给人看的终端画面；一旦进入 PTY，终端折行、ANSI 控制序列、光标帧、提示文本或启动警告可能污染 JSONL，导致前端只剩“Codex 正在处理中”而看不到命令、工具结果和最终回复。当前节点默认 `ELON_CODEX_JSON_DIRECT_STDOUT=1`，因此 Codex 跳过 PTY sidecar；只有显式设置 `ELON_CODEX_JSON_DIRECT_STDOUT=0` 才让 Codex 回到旧 sidecar 路径。

PTY/ConPTY sidecar 仍然保留，但它是辅助路，不是 Codex 结构化过程展示主路：

- 用户点击“打开终端接管”、需要真实终端输入、resize、取消或调试时走 sidecar/PTY。
- 交互式 `codex` TUI、Copilot / Claude / Gemini 这类终端型 CLI 可以继续使用 sidecar/PTY。
- 任务恢复以 task journal、Codex session/thread id、云端快照和 sidecar registry 组合实现；不重新接管任意外部终端 TTY。

Route B/C1/C2 的一龙工具 runtime 已经包含 `list_dir`、`search_files`、`file_info`、`read_file`、`read_file_range`、只读 `git_status` / `git_diff` / `git_log` / `git_show`、`write_file`、`apply_patch`、`run_command`。其中 `git_status`、`git_diff`、`git_log`、`git_show` 是项目内只读 Git 检查工具，不消耗命令审批；`write_file`、`apply_patch`、`run_command` 在非只读模式下会先向 PC 网页端发出工具审批卡，用户批准后才会真正执行；拒绝、超时或任务取消都不会执行该工具。`write_file` 审批会展示整文件替换 diff，并在敏感路径、旧/新内容命中敏感字段、过大 diff、二进制内容或非 UTF-8 旧文件时 fail-closed；用户批准后还会复查旧文件 hash，防止审批后文件被外部进程改动。`apply_patch` 复用现有 unified diff 安全检查，继续拒绝 `.git`、大小写变体 `.GIT`、绝对路径、`..`、越界路径和非 unified diff。

PC Dev Runtime 生成的项目级 `scripts\elon-agent.ps1` 已为 Route B/C1/C2 增加 `.elon\agent-runs\*.jsonl` 生命周期日志：记录运行开始、模型轮次、工具名称和目标、结果大小、完成或失败状态；不记录完整文件内容、工具输出、prompt 或 API key。Win 节点本地受保护接口 `/api/project-agent-runs` 可以按 `workspace_path` 读取这些日志摘要、尾部事件、活跃控制句柄、最近可续任务和顶层 `recovery_entry`，方便 PC UI 直接展示“推荐恢复”入口、做任务恢复和压力测试。

任务生命周期压力测试已经覆盖 Route A/B/C1/C2/C3 终态、取消、并发 journal 写入、分页回放、节点重启后丢失控制句柄、等待工具审批时节点重启、终态任务清理遗留审批 waiter、过期 Codex session 清理和超大工具事件截断；其中“等待审批时重启”的契约是：前端仍可从 journal 回放审批卡历史，但不能继续审批已经丢失的内存 waiter，只能基于快照开启新任务。恢复契约会结构化暴露 `tool_approval_recovery`，状态包含 `active_waiter`、`no_active_waiter`、`lost_after_restart`、`closed_by_terminal_task`、`unavailable`，PC UI 会优先使用该字段解释历史审批为什么可点、失效或只能基于快照继续；本机 `approval_state.approvals[]` 也会给每个审批项返回 `next_action`、`requires_new_task` 和可选 `checkpoint`。Route B/C1/C2 发起 `write_file`、`apply_patch`、`run_command` 审批时，会把不含 prompt/API key/写入正文的 `approval_checkpoint` 写进本机 journal：包含注册时间、过期时间、action/diff 指纹、重启后当前仍需 `continue_from_snapshot` 的原因和后续必须重新校验的字段。这样已经完成“审批恢复所需证据先落库”的第一步，但为了安全，节点重启后直接续批仍未开放；旧 waiter 丢失时仍必须基于快照新开任务。项目频道里的“本机 Agent 运行”面板也会在推荐恢复卡、最近任务卡和继续草稿里显示这条审批恢复状态，避免接手任务的 AI 或用户误解旧审批能力。任务正常结束、失败或取消进入统一收尾时，会按 req_id 清空仍残留的本机工具审批；本机 task journal API 在刷新终态任务时也会把历史 pending 审批标成“已关闭/任务已结束”，避免 PC UI 继续显示可误解的失效审批按钮。

CLI TTY 接管方案当前明确走“有限连续性”契约：不重新接管已经打开的原 CLI 终端 TTY；本机状态接口会结构化暴露 `not_supported`、`resume_order`、`recommended_next_actions` 和 `future_work`，前端优先提示运行句柄、journal 回放、Codex session 自动续接或云端快照新任务四种继续路径。真正接管原 TTY 仍需要后续 PTY/ConPTY 会话层、会话 id 持久化和前端 attach 授权协议。

Win 客户端“注册本地项目”流程会自动读取常见项目清单来填项目名、描述、Git 远端、分支和运行/测试/构建命令；其中 Node.js 项目会识别 npm/pnpm/yarn/bun 锁文件、`dev/start/serve/watch`、`test/check/test:unit/typecheck`、`build/compile/dist` 等常见脚本，并在缺少 scripts 但检测到 Vite / Next / Astro / Tauri 依赖时自动给出开发和构建命令；Wails 桌面项目会识别 `wails.json` + `go.mod` 并在用户直接选择 Wails 项目目录时自动给出 `wails dev`、`go test ./...`、`wails build`；Android/Gradle 项目会从 `settings.gradle` 或 `settings.gradle.kts` 的 `rootProject.name` 自动识别项目名，.NET 项目会从 `.sln` 或 `.csproj` 自动识别名称和描述，Python 项目会识别 `pyproject.toml`、`requirements.txt`、`uv.lock`、`poetry.lock`、`Pipfile`、`manage.py`。选择 monorepo 根目录时，也会浅层识别 `server/`、`backend/`、`api/`、`app/`、`cmd/`、`web/`、`frontend/`、`client/`、`android/` 里的 `package.json`、`Cargo.toml`、`pyproject.toml`、`go.mod`、Gradle/Tauri/Wails/.NET 清单和 Python 子模块，减少用户手填字段。

Route C1 平台AI能力已经有服务端预算审计和运营后台报告：记录 admitted / success / provider_error / output_rejected 等结果，不保存 prompt 或完整输出；运营报告会显示 pending 调用、超过阈值仍未完成的 stale pending 调用和对应审计事件，方便发现服务器模型调用卡住或 provider 异常。`/api/agent/runtime/status` 会返回结构化 `blockingReasons`，把运维开关、agent 策略、server_api_key-only、平台预算、个人额度、限流等不可用原因统一给 Win 节点和 PC UI。Win 节点读取服务器 Route C1 状态时会 fail-closed：如果 `policy.enabled=false`、`admissionAvailability.ready=false`、用户/平台预算耗尽、频率限制、`agentPolicy` 明确不可用，或 `blockingReasons` 非空，即使顶层 `ready=true` 也不会把 Route C1 显示成可用。

Win 节点 `/api/status` 会返回 `runtime_policy`，结构化暴露 full_access 的真实边界：Route A/C3 full_access 只适用于已安装 CLI 且需要本机项目授权；Route B/C1/C2 即使 full_access，也不会绕过工作区路径检查、命令白名单、工具审批或高危 `git push` 拦截。PC 页面和后续运营面板可以直接读取 `runtime_policy.fullAccess`、`runtime_policy.routeBC.highRiskGitPushDenied` 和 `full_access_grant_count` 做可视化，不再只靠文档记忆。

Win 客户端维护入口已经集中到 PC 设置页和本机 `/api/client-maintenance`：状态接口会返回安装目录、运行日志、启动器日志、任务 journal、诊断目录、完整维护动作列表和唯一 `primary_maintenance_action`。PC 设置页会把首要建议动作排在维护按钮第一位，例如安装布局异常时优先“修复客户端入口”，布局正常时优先“检查更新”；卸载仍保留确认文案，诊断导出继续输出脱敏 JSON。

这还不是完整 Codex 桌面版 parity。后续仍建议补：基于已落库 `approval_checkpoint` 做重启后续批的二次安全校验和执行恢复；原 CLI TTY 仍是有限连续性，真正重新 attach 还需要后续 PTY/ConPTY 会话层。

---

可以。更准确地说，你可以做一个 **Rust CLI**，让它在本机接收你的自然语言需求，然后调用远程 AI API 生成代码，最后由 CLI 把代码写成 `.ps1`、`.bat`、`.py`、`.sh`、`.rs` 等文件。

重点是：**API key 只负责授权，不负责生成代码**。生成代码的是远程模型；你的 CLI 负责调用 API、解析返回结果、校验路径、写入文件。OpenAI 官方文档也建议通过 API key 或短期 token 做 Bearer 认证，并提醒 API key 是秘密，不要暴露在客户端代码里，最好从环境变量或密钥管理服务读取。([OpenAI开发者][1])

## 推荐架构

你可以分两种做法。

**自己用的 CLI：**

```text
你的终端
  ↓
Rust CLI
  ↓ 读取 OPENAI_API_KEY
远程 AI API
  ↓ 返回 JSON：文件名 + 文件内容
Rust CLI
  ↓ 安全检查
写入本机文件
```

**给别人用的 CLI：**

```text
用户终端
  ↓
Rust CLI
  ↓ 登录 token，不放你的 OpenAI API key
你自己的后端 API
  ↓ 后端保存真正的 API key
OpenAI / 其他模型 API
  ↓
你自己的后端
  ↓
Rust CLI 写入用户电脑
```

如果这个 CLI 将来要发给别人用，不建议把你的 API key 放进二进制程序里，因为别人可以逆向或抓包拿到。更好的方式是让 CLI 请求你自己的后端，由后端保管 API key。

下面给你一个 **最小可运行版本**。它包含：

命令行参数解析、调用 OpenAI Responses API、要求模型返回 JSON、解析生成文件、阻止写到危险路径、支持 `--dry-run` 预览、支持 `--yes` 自动写入。

Rust 里可以用 `clap` 做 CLI 参数解析，它支持通过 derive 定义命令结构；用 `reqwest` 做 HTTP 请求；用 `tokio` 跑异步网络请求。([Docs.rs][2]) OpenAI 文档推荐用 Responses API 做文本生成，模型可以返回代码、JSON 等文本内容。([OpenAI开发者][3]) 这里默认模型我用 `gpt-5.4-mini`，因为官方模型页把它列为更低延迟、较低成本的选择之一；你也可以改成账号可用的其他模型。([OpenAI开发者][4])

---

## 1. 创建项目

```bash
cargo new scriptgen
cd scriptgen
```

---

## 2. `Cargo.toml`

```toml
# Cargo.toml

[package]
name = "scriptgen"
version = "0.1.0"
edition = "2024"

[dependencies]
# clap：负责解析命令行参数，例如 gen、--out、--dry-run
clap = { version = "4", features = ["derive"] }

# tokio：异步运行时，reqwest 的异步 HTTP 请求需要它
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }

# reqwest：HTTP 客户端，用来调用远程 API
reqwest = { version = "0.13", features = ["json"] }

# serde / serde_json：序列化请求 JSON，反序列化返回 JSON
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# anyhow：让错误处理更舒服
anyhow = "1"
```

---

## 3. `src/main.rs`

````rust
// src/main.rs

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

/// 这个结构体代表整个 CLI 程序的入口参数。
#[derive(Debug, Parser)]
#[command(
    name = "scriptgen",
    version,
    about = "用远程 AI API 生成本机脚本文件的 Rust CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// CLI 支持的子命令。
#[derive(Debug, Subcommand)]
enum Commands {
    /// 根据自然语言需求生成脚本文件
    Gen {
        /// 你想生成什么脚本，例如：帮我写一个清理 Windows 临时文件的 PowerShell 脚本
        prompt: String,

        /// 输出目录，默认是当前目录下的 generated_scripts
        #[arg(short, long, default_value = "generated_scripts")]
        out: PathBuf,

        /// 目标脚本语言，例如 powershell、bat、python、bash、rust
        #[arg(short, long, default_value = "powershell")]
        lang: String,

        /// 使用的模型。可以改成你账号可用的模型
        #[arg(long, default_value = "gpt-5.4-mini")]
        model: String,

        /// 只预览，不写入文件
        #[arg(long)]
        dry_run: bool,

        /// 跳过确认，直接写入文件
        #[arg(short, long)]
        yes: bool,
    },
}

/// 远程模型应该返回的整体 JSON 结构。
///
/// 期望格式：
/// {
///   "notes": "可选说明",
///   "files": [
///     {
///       "path": "hello.ps1",
///       "content": "Write-Host \"Hello\""
///     }
///   ]
/// }
#[derive(Debug, Deserialize)]
struct GeneratedProject {
    notes: Option<String>,
    files: Vec<GeneratedFile>,
}

/// 远程模型返回的单个文件。
#[derive(Debug, Deserialize)]
struct GeneratedFile {
    path: String,
    content: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Gen {
            prompt,
            out,
            lang,
            model,
            dry_run,
            yes,
        } => {
            run_gen_command(prompt, out, lang, model, dry_run, yes).await?;
        }
    }

    Ok(())
}

/// 执行 gen 子命令。
async fn run_gen_command(
    prompt: String,
    out_dir: PathBuf,
    lang: String,
    model: String,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    let api_key = env::var("OPENAI_API_KEY")
        .context("没有找到环境变量 OPENAI_API_KEY。请先在终端里设置你的 API key。")?;

    let response_text = call_openai_api(&api_key, &model, &lang, &prompt)
        .await
        .context("调用远程 AI API 失败")?;

    let project = parse_generated_project(&response_text)
        .context("远程 API 返回的内容不是合法的项目 JSON")?;

    if project.files.is_empty() {
        bail!("远程 API 没有返回任何文件。");
    }

    println!("\n即将生成以下文件：");
    for file in &project.files {
        let safe_path = safe_join(&out_dir, &file.path)?;
        println!("  - {}", safe_path.display());
    }

    if let Some(notes) = &project.notes {
        println!("\n模型说明：\n{}\n", notes);
    }

    if dry_run {
        print_dry_run_preview(&project)?;
        return Ok(());
    }

    if !yes {
        confirm_before_write()?;
    }

    write_project_files(&out_dir, &project)?;

    println!("\n文件已写入：{}", out_dir.display());
    println!("建议先人工检查脚本内容，不要直接运行陌生脚本。");

    Ok(())
}

/// 调用 OpenAI Responses API。
async fn call_openai_api(
    api_key: &str,
    model: &str,
    lang: &str,
    user_prompt: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let instructions = build_system_instructions();
    let input = build_user_input(lang, user_prompt);

    let request_body = json!({
        "model": model,
        "instructions": instructions,
        "input": input
    });

    let response = client
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await
        .context("HTTP 请求发送失败")?;

    let status = response.status();
    let body_text = response
        .text()
        .await
        .context("读取 HTTP 响应内容失败")?;

    if !status.is_success() {
        return handle_api_error(status, body_text);
    }

    let value: Value = serde_json::from_str(&body_text)
        .context("OpenAI API 返回的响应不是合法 JSON")?;

    extract_output_text(&value).context("无法从 OpenAI 响应中提取文本输出")
}

/// 构造给模型的高优先级指令。
fn build_system_instructions() -> String {
    let text = r#"
你是一个帮助用户生成 PC 本机脚本文件的代码生成器。

你必须只返回 JSON，不要返回 Markdown，不要返回解释文本，不要使用 ``` 代码块。

返回格式必须是：

{
  "notes": "简短说明，可为空字符串",
  "files": [
    {
      "path": "相对路径文件名",
      "content": "完整文件内容"
    }
  ]
}

强制规则：
1. path 必须是相对路径，不能是绝对路径。
2. path 不能包含 ..。
3. 不要生成会删除用户重要数据、窃取信息、持久化后门、绕过安全软件、下载执行未知程序的脚本。
4. 生成的脚本应该尽量安全、可读、带注释。
5. 如果脚本可能修改系统状态，要在脚本里加入明确提示或确认步骤。
6. 不要让脚本自动提权。
7. 不要把 API key、密码、token 写进生成文件。
"#;

    text.trim().to_string()
}

/// 构造用户输入。
fn build_user_input(lang: &str, user_prompt: &str) -> String {
    format!(
        r#"
目标脚本语言：{lang}

用户需求：
{user_prompt}

请根据用户需求生成一个或多个文件。
请确保返回严格 JSON。
"#
    )
}

/// 处理 API 错误。
fn handle_api_error(status: StatusCode, body_text: String) -> Result<String> {
    bail!(
        "API 请求失败。\nHTTP 状态码：{}\n响应内容：{}",
        status,
        body_text
    );
}

/// 从 Responses API 的 JSON 响应中提取文本。
///
/// 注意：Responses API 的 output 数组里可能有多个 item，
/// 不能假设 output[0].content[0].text 一定存在。
fn extract_output_text(value: &Value) -> Result<String> {
    let mut parts: Vec<String> = Vec::new();

    if let Some(output_array) = value.get("output").and_then(Value::as_array) {
        for output_item in output_array {
            if let Some(content_array) = output_item.get("content").and_then(Value::as_array) {
                for content_item in content_array {
                    if let Some(text) = content_item.get("text").and_then(Value::as_str) {
                        parts.push(text.to_string());
                    }
                }
            }
        }
    }

    if parts.is_empty() {
        bail!("响应里没有找到文本内容。原始响应：{}", value);
    }

    Ok(parts.join("\n"))
}

/// 解析模型返回的项目 JSON。
fn parse_generated_project(text: &str) -> Result<GeneratedProject> {
    let cleaned = clean_possible_json_text(text);

    let project: GeneratedProject = serde_json::from_str(&cleaned)
        .with_context(|| format!("解析失败，清理后的内容是：\n{}", cleaned))?;

    Ok(project)
}

/// 尽量清理模型可能额外包裹的内容。
///
/// 理想情况下，模型应该只返回 JSON。
/// 但为了容错，这里会尝试截取第一个 { 到最后一个 }。
fn clean_possible_json_text(text: &str) -> String {
    let trimmed = text.trim();

    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start <= end {
            return trimmed[start..=end].to_string();
        }
    }

    trimmed.to_string()
}

/// 预览生成内容，不写入文件。
fn print_dry_run_preview(project: &GeneratedProject) -> Result<()> {
    println!("\n========== DRY RUN 预览 ==========");

    for file in &project.files {
        println!("\n--- 文件：{} ---", file.path);
        println!("{}", file.content);
    }

    println!("\n========== DRY RUN 结束 ==========");
    Ok(())
}

/// 写入所有生成文件。
fn write_project_files(out_dir: &Path, project: &GeneratedProject) -> Result<()> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("创建输出目录失败：{}", out_dir.display()))?;

    for file in &project.files {
        let full_path = safe_join(out_dir, &file.path)?;

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建父目录失败：{}", parent.display()))?;
        }

        fs::write(&full_path, &file.content)
            .with_context(|| format!("写入文件失败：{}", full_path.display()))?;
    }

    Ok(())
}

/// 安全拼接输出目录和模型返回的相对路径。
///
/// 这个函数会拒绝：
/// - 绝对路径
/// - 包含 .. 的路径
/// - Windows 盘符路径，例如 C:/xxx
/// - 空路径
///
/// 这样可以避免模型返回 "../../xxx" 之类的危险路径。
fn safe_join(base_dir: &Path, model_path: &str) -> Result<PathBuf> {
    let raw = model_path.trim();

    if raw.is_empty() {
        bail!("文件路径不能为空。");
    }

    if raw.contains(':') {
        bail!("文件路径不能包含冒号，疑似 Windows 盘符路径：{}", raw);
    }

    let normalized = raw.replace('\\', "/");
    let relative_path = Path::new(&normalized);

    if relative_path.is_absolute() {
        bail!("文件路径不能是绝对路径：{}", raw);
    }

    let mut cleaned = PathBuf::new();

    for component in relative_path.components() {
        match component {
            Component::Normal(part) => {
                cleaned.push(part);
            }
            Component::CurDir => {
                // 忽略 ./xxx 里的当前目录符号
            }
            Component::ParentDir => {
                bail!("文件路径不能包含 .. ：{}", raw);
            }
            Component::RootDir | Component::Prefix(_) => {
                bail!("文件路径不能包含根目录或盘符：{}", raw);
            }
        }
    }

    if cleaned.as_os_str().is_empty() {
        bail!("清理后的文件路径为空：{}", raw);
    }

    Ok(base_dir.join(cleaned))
}

/// 写入前让用户确认。
fn confirm_before_write() -> Result<()> {
    print!("\n确认写入这些文件吗？输入 yes 继续：");
    io::stdout().flush().context("刷新 stdout 失败")?;

    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .context("读取用户输入失败")?;

    let answer = answer.trim().to_lowercase();

    if answer != "yes" && answer != "y" && answer != "是" {
        bail!("用户取消写入。");
    }

    Ok(())
}
````

---

## 4. 设置 API key

Windows PowerShell 临时设置：

```powershell
$env:OPENAI_API_KEY = "你的_API_KEY"
```

macOS / Linux 临时设置：

```bash
export OPENAI_API_KEY="你的_API_KEY"
```

临时设置只在当前终端窗口有效。以后你可以升级成读取系统 Keychain / Windows Credential Manager / Linux Secret Service。Rust 生态里有 `keyring` 相关库可以连接系统凭据存储。([Docs.rs][5])

---

## 5. 运行测试

先用 `--dry-run`，只预览，不写文件：

```bash
cargo run -- gen "帮我写一个 Windows PowerShell 脚本，列出当前目录下最大的 20 个文件" --lang powershell --dry-run
```

确认没问题后写入文件：

```bash
cargo run -- gen "帮我写一个 Windows PowerShell 脚本，列出当前目录下最大的 20 个文件" --lang powershell --out ./my_scripts --yes
```

生成后你会看到类似：

```text
my_scripts/find_large_files.ps1
```

---

## 你这个 CLI 后面可以升级成这样

最建议的路线是：

```text
v0.1：只生成文件，不执行脚本
v0.2：加入 dry-run、diff 预览、确认写入
v0.3：加入项目模板，比如 powershell、python、rust-cli、bat
v0.4：加入本地历史记录，保存每次 prompt 和生成结果
v0.5：加入你自己的远程后端，隐藏真正的 API key
v0.6：加入安全策略，比如禁止删除系统目录、禁止自启动、禁止下载执行
```

最重要的安全原则：**你的 CLI 可以生成脚本，但不要默认自动执行脚本**。先生成、展示、确认，再让用户自己决定是否运行。

[1]: https://developers.openai.com/api/reference/overview/ "API Overview | OpenAI API Reference"
[2]: https://docs.rs/clap/latest/clap/_derive/index.html "clap::_derive - Rust"
[3]: https://developers.openai.com/api/docs/guides/text "Text generation | OpenAI API"
[4]: https://developers.openai.com/api/docs/models "Models | OpenAI API"
[5]: https://docs.rs/keyring?utm_source=chatgpt.com "keyring - Rust"


对，你可以把它理解成一种 **AI Agent CLI**，但不一定是“套壳其他 AI 代理 CLI”。

更准确地说，有两条路线：

```text
路线 A：套壳别人的 CLI
你的 Rust CLI
  ↓
调用 codex / claude / gemini 等现成 CLI
  ↓
它们自己调用模型、读写文件、执行命令

路线 B：自己写 Agent Runtime
你的 Rust CLI
  ↓
直接调用 OpenAI / Claude / Gemini API
  ↓
你自己实现：读文件、写文件、执行命令、权限确认、安全限制
```

我更推荐 **路线 B**。因为你想做的是“自己的 CLI”，不是单纯给别人的 CLI 包一层皮。

OpenAI 官方对 Codex CLI 的描述就是：它是一个可以在本地终端运行的 coding agent，能够在选定目录里读取、修改并运行代码；而且 Codex CLI 本身也是用 Rust 构建的。([OpenAI开发者][1]) Claude Code 也是类似定位：运行在本地终端，可以理解代码库、编辑文件、运行命令，并且在修改文件或执行命令前请求许可。([Claude][2]) 所以你想做的东西，方向上确实和 Codex CLI、Claude Code 这种产品属于同一类。

但是，**API key 本身不能操控 PC**。API key 只是让你的 CLI 能调用远程模型。真正操控 PC 的能力，来自你在本机 Rust 程序里开放的工具。

可以这样理解：

```text
AI 模型：负责思考、规划、生成命令
API key：负责认证和扣费
Rust CLI：负责真正操作电脑
工具层：负责读文件、写文件、运行命令、打开程序、调用 PowerShell
权限层：负责决定哪些操作允许，哪些操作必须确认
```

所以答案是：

**可以正常操控 PC，但前提是你的 CLI 实现了本地工具执行能力。**

---

## 你的 CLI 可以分成 4 个等级

### 第 1 级：脚本生成器

这是我们前面讨论的版本。

```text
用户输入：帮我写一个清理临时目录的 PowerShell 脚本
AI 返回：clean_temp.ps1 的代码
CLI：把代码保存到文件
```

这个等级 **不会真正操控 PC**，只是生成脚本文件。

优点是安全，适合第一版。

---

### 第 2 级：半自动执行器

这个版本就开始有操控能力了。

```text
用户输入：帮我查看 C 盘还有多少空间
AI 生成命令：Get-PSDrive C
CLI 展示命令
用户确认
CLI 执行 PowerShell
CLI 把结果发回 AI
AI 解释结果
```

流程类似：

```text
你
 ↓
Rust CLI
 ↓
AI：我要执行这个命令
 ↓
Rust CLI：展示命令，问你是否允许
 ↓
你确认
 ↓
Rust CLI 真正执行命令
 ↓
执行结果返回给 AI
 ↓
AI 决定下一步
```

这个时候它已经像一个真正的 agent 了。

---

### 第 3 级：本地 Agent

这个版本可以循环工作。

比如你说：

```text
帮我创建一个 Rust 项目，实现一个扫描大文件的工具，然后编译测试。
```

它可以做：

```text
1. 创建目录
2. 写 Cargo.toml
3. 写 src/main.rs
4. 执行 cargo check
5. 读取错误
6. 修改代码
7. 再执行 cargo test
8. 最后告诉你结果
```

这就很接近 Codex CLI / Claude Code 的工作方式了。OpenAI 的 Codex 安全文档也提到，这类 agent 一般需要两层控制：一层是 sandbox 限制它技术上能碰什么，另一层是 approval policy 决定什么时候必须停下来请求用户批准。([OpenAI开发者][3])

---

### 第 4 级：完整 PC 操作员

这个版本不只操作代码目录，还能操作整个 PC。

例如：

```text
打开浏览器
下载文件
移动文件
打开 Excel
点击按钮
截图分析
填写表单
运行安装程序
管理 Windows 服务
```

这个就不只是 CLI coding agent，而是 **computer-use agent / desktop automation agent**。

技术上可以做，但不建议一开始就做，因为安全风险会大很多。

---

## “套壳其他 AI 代理 CLI”可以做，但不是最佳路线

比如你的 Rust CLI 里面可以这样做：

```text
Rust CLI
  ↓
std::process::Command 调用 codex 或 claude
  ↓
捕获它们的输出
  ↓
再包装成你自己的界面
```

这确实是“套壳”。

优点：

```text
开发快
不用自己实现 agent loop
不用自己设计工具调用协议
可以直接复用 Codex / Claude Code 的能力
```

缺点：

```text
依赖用户本机必须安装那个 CLI
依赖用户已经登录
输出格式可能变
权限机制你不好完全控制
商业限制和费用不好统一
你自己的产品差异化较弱
```

所以如果你只是自己用，套壳可以；如果你想做成自己的工具，我建议你直接调用模型 API，然后自己写工具层。

---

## 真正的核心不是“调用 AI”，而是“工具层”

一个本地 PC Agent 大概长这样：

```text
┌──────────────────────────┐
│        Rust CLI           │
├──────────────────────────┤
│  对话管理 Conversation    │
├──────────────────────────┤
│  AI API Client            │
├──────────────────────────┤
│  Tool Registry 工具注册表 │
├──────────────────────────┤
│  Permission 权限确认      │
├──────────────────────────┤
│  Sandbox 安全沙盒         │
├──────────────────────────┤
│  Executor 执行器          │
└──────────────────────────┘
```

工具层可以先做这些：

```text
read_file(path)
write_file(path, content)
list_dir(path)
run_command(command, args, cwd)
ask_user(question)
create_file(path, content)
replace_in_file(path, old, new)
```

后面再加：

```text
run_powershell(script)
open_url(url)
take_screenshot()
click(x, y)
type_text(text)
find_window(title)
```

不过我建议你先不要做鼠标点击、键盘输入、自动安装软件这些能力。第一版先做 **文件 + 命令行 + PowerShell**，已经够强了。

---

## 它能不能“正常操控 PC”？

可以，但要分清楚边界。

### 它可以做的

只要当前用户权限允许，Rust CLI 就可以：

```text
读写文件
创建目录
删除文件
运行命令
运行 PowerShell
调用 bat / exe
启动浏览器
启动其他程序
读取命令输出
编译代码
操作 Git
调用系统 API
```

也就是说，它和你在终端里手动操作 PC 的能力差不多。

---

### 它不能天然做到的

它不会因为接了 AI API，就自动拥有这些能力：

```text
绕过系统权限
绕过管理员权限
绕过杀毒软件
控制没有权限访问的目录
稳定理解所有 GUI 界面
保证生成脚本 100% 正确
保证不会误删文件
```

AI 只是会“建议下一步做什么”。真正执行的是你的 CLI。

---

## 我建议你做成这种模式

不要让 AI 直接返回一整段危险命令让 Rust 执行，而是让 AI 返回结构化动作。

例如 AI 不要返回：

```powershell
Remove-Item -Recurse C:\Users\xxx\Downloads\*
```

而是返回 JSON：

```json
{
  "action": "run_command",
  "command": "powershell",
  "args": [
    "-NoProfile",
    "-Command",
    "Get-ChildItem $env:USERPROFILE\\Downloads | Sort-Object Length -Descending | Select-Object -First 20"
  ],
  "risk": "low",
  "explain": "只查看下载目录最大的 20 个文件，不删除。"
}
```

你的 Rust CLI 收到后做判断：

```text
是否是允许的 action？
是否包含危险命令？
是否访问了禁止目录？
是否需要用户确认？
是否在 workspace 里执行？
```

这样就比“AI 想执行什么就执行什么”安全很多。

---

## 推荐的第一版能力

我建议你第一版不要叫“PC 自动控制器”，而是叫：

```text
本地脚本 Agent
```

功能范围：

```text
1. 根据中文需求生成脚本
2. 保存脚本文件
3. 支持 dry-run 预览
4. 支持用户确认后运行脚本
5. 读取运行结果
6. 如果报错，把错误发给 AI 自动修复
7. 所有危险操作都必须确认
```

这样第一版就很实用。

例如：

```bash
pcai gen "写一个 PowerShell，列出桌面上超过 100MB 的文件"
pcai run ./generated/find_large_files.ps1
pcai fix ./generated/find_large_files.ps1 --last-error
```

后面再进化成：

```bash
pcai "帮我清理这个 Rust 项目的 warning"
pcai "帮我把这个目录下的图片按日期分类"
pcai "帮我检查为什么这个 Python 脚本运行失败"
pcai "帮我生成一个 Windows 定时任务"
```

---

## 最合理的最终形态

我觉得你的 CLI 可以做成这样：

```text
pcai
 ├── chat        和本机 agent 对话
 ├── gen         只生成脚本，不执行
 ├── run         执行脚本，但需要确认
 ├── fix         根据报错修复脚本
 ├── explain     解释某个脚本在干什么
 ├── audit       检查脚本是否危险
 ├── apply       应用 AI 修改文件
 └── config      配置模型、API key、安全策略
```

运行时像这样：

```text
用户：帮我写一个脚本，把 Downloads 里超过 500MB 的文件列出来，但不要删除

AI：我准备创建文件：
  generated/list_big_downloads.ps1

CLI：是否写入？yes

用户：yes

CLI：已写入

用户：运行一下

AI：我准备执行：
  powershell -NoProfile -ExecutionPolicy Bypass -File generated/list_big_downloads.ps1

CLI：这个命令只读取文件列表，不删除文件。是否执行？yes

用户：yes

CLI：执行完成，结果如下...
```

这就是一个比较健康的 PC Agent。

---

## 关键结论

你这个项目可以做，而且方向很清楚：

```text
不是只有“套壳其他 AI CLI”这一种做法。
更好的方式是：Rust CLI + 模型 API + 本地工具层 + 权限控制。
```

它可以操控 PC，但不是 API key 在操控 PC，而是你的 Rust 程序在操控 PC。AI 只负责决定“下一步想做什么”，Rust CLI 负责判断“能不能做、要不要问用户、怎么安全执行”。

## 2026-06-21 项目实现状态映射

当前项目不是只做理论 CLI，已经有三条执行路线：

```text
Route A：套壳本机 CLI
一龙 PC 节点
  ↓
只允许已发现的 codex / copilot / claude / gemini
  ↓
本机 CLI 自己调用模型、读写文件、执行命令

Route B：本机 API runtime
一龙 PC 节点
  ↓
读取用户本机 OpenAI-compatible API 配置
  ↓
节点自己的工具层执行 list/read/write/run_command

Route C：服务器 runtime
一龙 PC 节点
  ↓
调用我们服务器托管的模型能力
  ↓
节点自己的工具层执行 list/read/write/run_command
```

2026-06-24 增量：Route C 不是无限开放的“服务器免费模型”。服务端已经有运营保护层：

```text
ELON_SERVER_AGENT_RUNTIME_ENABLED
  可无发布暂停 Route C

ELON_SERVER_AGENT_RUNTIME_DAILY_CALL_LIMIT
  限制平台每天最多服务器模型调用次数

ELON_SERVER_AGENT_RUNTIME_PER_USER_DAILY_CALL_LIMIT
  限制单个用户每天最多服务器模型调用次数

ELON_SERVER_AGENT_RUNTIME_DUPLICATE_WINDOW_SECS
  限制同一用户在短时间内重复提交同一请求，默认 5 秒，可设 0 关闭

ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS
  默认只允许服务器默认 agent；需要显式 allowlist 才能选其他 agent

Route C1 agent usage_mode
  只允许 server_api_key；user_api_key_proxy / CLI 类 / Copilot 类 agent 即使被误配为默认或 allowlist，也不会被 Route C1 调用

Route C1 agentPolicy 状态
  /api/agent/runtime/status 结构化暴露 default_agent_only / allowlist / any，Win 客户端 Route C1 保护状态会展示 agent 策略
```

Win 客户端的 Route C1 状态会区分平台预算耗尽、个人额度耗尽、agent 模式不允许，并显示重试时间、平台剩余额度、个人剩余额度、并发、分钟级请求限制、重复请求防抖窗口和 agent 策略。

已完成的边界：

```text
项目绑定、开发频道、Route A/B/C1/C2/C3 选择
read_only / project_write / full_access 权限字段
Route B/C1/C2 本机工具白名单
Route B/C1/C2 工具调用时间线
Route B/C1/C2 任务恢复契约会暴露 tool_approval_recovery，用 active_waiter / no_active_waiter / lost_after_restart / closed_by_terminal_task / unavailable 区分当前可审批、历史回放和重启后失效审批；PC 项目运行面板会把这条状态显示到推荐恢复卡和继续草稿
Route C1 平台日预算 + 用户日预算 + 重复请求防抖 + agent 选择保护 + server_api_key-only 硬门槛 + 结构化 blockingReasons
Win 客户端维护面板展示安装状态、开始菜单健康、日志入口、修复客户端入口、更新/卸载动作和下一步建议
客户端维护按钮已经改成后端 `maintenance_actions` 契约驱动；PC 设置页和节点注册页都会按 `kind/target/enabled/tone/confirmation` 生成可点击的日志、诊断、配置、修复、更新和卸载动作，避免前端硬编码按钮和后端能力漂移
本机 7799 管理 API token 保护
Route A CLI 名称、路径、参数、cwd fail-closed 校验
legacy relay 同步拒绝任意 CLI 和内置 runtime
Route B/C1/C2 即使开启 full_access，也不会放宽本机 run_command 命令白名单；Git 推送会继续拒绝 --force / --delete / --mirror / --all / --tags / +refspec / :branch 等高危参数，正常 HEAD:main 推送仍可审批执行
```

还不能说完全等同 Codex Desktop。按 2026-06-24 当前主线，下一步主要补：

```text
跨节点重启后可继续审批的审批状态落库
PC UI 任务恢复入口继续产品化
原 CLI TTY 接管仍仅支持有限连续性，不支持重新 attach 原 TTY
```

我建议你下一步直接做 **v0.1：生成脚本 + 预览 + 写入 + 手动确认运行**。这个版本安全、可控，而且已经能完成很多真实 PC 自动化任务。

[1]: https://developers.openai.com/codex/cli "CLI – Codex | OpenAI Developers"
[2]: https://claude.com/product/claude-code "Claude Code by Anthropic | AI Coding Agent, Terminal, IDE"
[3]: https://developers.openai.com/codex/agent-approvals-security "Agent approvals & security – Codex | OpenAI Developers"
