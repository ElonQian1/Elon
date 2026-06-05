//! POST /api/agent-balloon/ensure — 为当前用户自动创建"手机控制"项目空间
//!
//! 幂等：同一用户多次调用只创建一次，返回相同的 project_id。
//! 还会在项目 workspace 里写入 AGENTS.md，让服务器 CLI（Codex/Claude）
//! 知道这个项目是手机自动化项目、需要生成 JSON 脚本格式。

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

const BALLOON_PROJECT_NAME: &str = "手机控制";

/// 悬浮球项目 AGENTS.md：让 Codex/Claude 知道这是手机自动化项目
const AGENTS_MD: &str = r#"# 手机控制助手

## 项目用途
这是用户手机助手（悬浮球语音）的执行项目。
每次用户通过语音说出手机操控指令，系统会把指令发到这里，由 AI 生成对应的执行方案。

## 响应规则

### 操控手机的指令（打开应用、发消息、搜索等）
直接以 JSON 格式返回执行脚本，不加任何多余文字：
```json
{
  "steps": [
    {"type": "LAUNCH_APP",     "params": {"package": "com.tencent.mm"}},
    {"type": "FIND_AND_TAP",   "params": {"text": "搜索"}},
    {"type": "INPUT_TEXT",     "params": {"text": "奶茶店"}},
    {"type": "FIND_AND_TAP",   "params": {"text": "搜索按钮"}},
    {"type": "WAIT",           "params": {"ms": 1000}},
    {"type": "GLOBAL_ACTION",  "params": {"action": "BACK"}}
  ]
}
```

### 步骤类型说明
- `LAUNCH_APP` — 启动应用，params.package 为包名
- `FIND_AND_TAP` — 找到界面元素并点击，params.text 为界面文字
- `INPUT_TEXT` — 在当前输入框输入文字
- `GLOBAL_ACTION` — 系统导航，action 可为 BACK / HOME / RECENTS
- `WAIT` — 等待，ms 为毫秒数
- `SCROLL` — 滑动，params.direction 为 UP/DOWN

### 常用应用包名
- 微信: com.tencent.mm
- QQ: com.tencent.mobileqq
- 小红书: com.xingin.xhs
- 抖音: com.ss.android.ugc.aweme
- 淘宝: com.taobao.taobao
- 京东: com.jingdong.app.mall
- 支付宝: com.eg.android.AlipayGphone
- 微博: com.sina.weibo
- B站: tv.danmaku.bili
- 设置: com.android.settings

### 普通聊天（非操控指令）
正常对话回复，无需 JSON。

## 重要
- 操控指令必须返回纯 JSON，不加 markdown 代码块标记
- 如果不确定某个应用的包名，先生成 LAUNCH_APP 步骤并用 text 参数描述，客户端会尝试匹配
- 步骤要完整，包含启动应用、执行操作、必要时返回等完整流程
"#;

/// POST /api/agent-balloon/ensure
///
/// 确保当前用户有一个名为"手机控制"的专属项目空间，
/// 并在 workspace 写入 AGENTS.md 让 CLI 知道脚本格式。
/// 返回 { "project_id": "...", "created": true/false }
pub async fn ensure_balloon_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let (project_id, created) = match state
        .store
        .ensure_balloon_project_for_user(&user.id, BALLOON_PROJECT_NAME)
    {
        Ok(v) => v,
        Err(e) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("创建手机控制项目失败: {e}"),
            );
        }
    };

    // 写入 AGENTS.md（幂等：每次 ensure 都覆盖写，保证最新格式）
    let workspace = state.get_project_workspace(&project_id);
    if let Err(e) = write_agents_md(&workspace) {
        tracing::warn!("写入 balloon AGENTS.md 失败（非致命）: {e}");
    }

    Json(json!({
        "project_id": project_id,
        "created": created,
    }))
    .into_response()
}

fn write_agents_md(workspace: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(workspace)?;
    std::fs::write(workspace.join("AGENTS.md"), AGENTS_MD)?;
    Ok(())
}
