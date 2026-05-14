/// 管理后台模块：提供 Web UI 和 REST API 用于运行时配置 AI 代理参数
///
/// 路由：
///   GET  /admin                       → 管理页面 HTML
///   GET  /api/admin/agents            → 列出所有代理（key 脱敏）
///   POST /api/admin/agents            → 新增或更新代理
///   DELETE /api/admin/agents/:name    → 删除代理
///   POST /api/admin/default/:name     → 设置默认代理
///   GET  /api/admin/agents/:name/key  → 查看某代理的完整 API key
///
/// 鉴权：所有 API 需要请求头 `Authorization: Bearer <ADMIN_TOKEN>`

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::types::{AgentConfig, AppState};

// ─────────────────────────────────────────────
// 鉴权工具函数
// ─────────────────────────────────────────────

fn check_auth(headers: &HeaderMap, token: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim_start_matches("Bearer ").trim() == token)
        .unwrap_or(false)
}

fn auth_error() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "无效的管理员令牌，请在页面顶部输入正确的 ADMIN_TOKEN"})),
    )
        .into_response()
}

/// 将 API key 脱敏：仅显示前4个字符和后4个字符（按字符而非字节，兼容中文）
fn mask_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        return "••••••••".into();
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{}••••{}", head, tail)
}

// ─────────────────────────────────────────────
// 路由处理函数
// ─────────────────────────────────────────────

/// 返回管理后台 HTML 页面
pub async fn admin_page() -> Html<&'static str> {
    Html(ADMIN_HTML)
}

/// 列出所有 AI 代理配置（API key 脱敏）
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let config = state.agents_config.read().await;
    let mut agents: Vec<serde_json::Value> = config
        .agents
        .values()
        .map(|a| {
            serde_json::json!({
                "name": a.name,
                "api_base": a.api_base,
                "api_key_masked": mask_key(&a.api_key),
                "model": a.model,
                "is_default": a.name == config.default_agent,
            })
        })
        .collect();

    // 按名称排序，让 UI 稳定
    agents.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    Json(serde_json::json!({
        "agents": agents,
        "default_agent": config.default_agent,
    }))
    .into_response()
}

/// 新增或更新 AI 代理配置
#[derive(Deserialize)]
pub struct UpsertAgentReq {
    pub name: String,
    pub api_base: String,
    /// 传空字符串表示不修改现有密钥
    pub api_key: String,
    pub model: String,
    pub set_as_default: bool,
}

pub async fn upsert_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<UpsertAgentReq>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let name = req.name.to_lowercase().trim().to_string();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "代理名称不能为空"})),
        )
            .into_response();
    }
    // 只允许字母、数字、连字符
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "代理名称只能包含字母、数字、连字符(-_)"})),
        )
            .into_response();
    }

    let mut config = state.agents_config.write().await;

    // API key 为空时保留原有密钥
    let api_key = if req.api_key.trim().is_empty() {
        config
            .agents
            .get(&name)
            .map(|a| a.api_key.clone())
            .unwrap_or_default()
    } else {
        req.api_key.trim().to_string()
    };

    let is_new = !config.agents.contains_key(&name);
    config.agents.insert(
        name.clone(),
        AgentConfig {
            name: name.clone(),
            api_base: req.api_base.trim().to_string(),
            api_key,
            model: req.model.trim().to_string(),
        },
    );

    if req.set_as_default || (is_new && config.agents.len() == 1) {
        config.default_agent = name.clone();
    }

    if let Err(e) = config.save_to_file(&state.config_path) {
        tracing::error!("保存代理配置到文件失败: {}", e);
    }

    tracing::info!(
        "管理后台：{} 代理 '{}'",
        if is_new { "新增" } else { "更新" },
        name
    );

    Json(serde_json::json!({"ok": true, "name": name})).into_response()
}

/// 删除 AI 代理
pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let mut config = state.agents_config.write().await;

    if config.agents.len() <= 1 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "至少需要保留一个 AI 代理，无法删除"})),
        )
            .into_response();
    }

    if config.agents.remove(&name).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "代理不存在"})),
        )
            .into_response();
    }

    // 如果删掉的是默认代理，自动切换到第一个
    if config.default_agent == name {
        config.default_agent = config.agents.keys().next().unwrap().clone();
        tracing::info!("默认代理已切换为 '{}'", config.default_agent);
    }

    if let Err(e) = config.save_to_file(&state.config_path) {
        tracing::error!("保存代理配置到文件失败: {}", e);
    }

    tracing::info!("管理后台：删除代理 '{}'", name);
    Json(serde_json::json!({"ok": true})).into_response()
}

/// 设置默认代理
pub async fn set_default_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let mut config = state.agents_config.write().await;

    if !config.agents.contains_key(&name) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "代理不存在"})),
        )
            .into_response();
    }

    config.default_agent = name.clone();

    if let Err(e) = config.save_to_file(&state.config_path) {
        tracing::error!("保存代理配置到文件失败: {}", e);
    }

    tracing::info!("管理后台：默认代理设为 '{}'", name);
    Json(serde_json::json!({"ok": true})).into_response()
}

/// 查看指定代理的完整 API key（需要 Bearer token）
pub async fn get_agent_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let config = state.agents_config.read().await;
    match config.agents.get(&name) {
        Some(a) => Json(serde_json::json!({"name": a.name, "api_key": a.api_key})).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "代理不存在"})),
        )
            .into_response(),
    }
}

// ─────────────────────────────────────────────
// 内嵌管理后台 HTML（无需任何外部 CDN）
// ─────────────────────────────────────────────

const ADMIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>一龙 AI 配置中心</title>
<style>
  :root {
    --bg: #0f1117;
    --card: #1a1d27;
    --border: #2a2d3e;
    --accent: #6c63ff;
    --accent-hover: #8b85ff;
    --danger: #ff4d6d;
    --success: #4ade80;
    --warn: #fbbf24;
    --text: #e2e8f0;
    --text-dim: #94a3b8;
    --input-bg: #0d1018;
    --shadow: 0 4px 24px rgba(0,0,0,0.4);
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: var(--bg); color: var(--text); font-family: 'Segoe UI', system-ui, sans-serif; min-height: 100vh; }
  header { background: var(--card); border-bottom: 1px solid var(--border); padding: 16px 32px; display: flex; align-items: center; gap: 16px; }
  header h1 { font-size: 20px; font-weight: 700; color: var(--accent); }
  header .subtitle { color: var(--text-dim); font-size: 13px; }
  .token-badge { margin-left: auto; display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--text-dim); }
  .token-badge button { background: var(--border); border: none; color: var(--text-dim); padding: 4px 12px; border-radius: 6px; cursor: pointer; font-size: 12px; }
  .token-badge button:hover { background: var(--accent); color: #fff; }
  .container { max-width: 1100px; margin: 0 auto; padding: 32px 24px; }
  .toolbar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
  .toolbar h2 { font-size: 16px; color: var(--text-dim); font-weight: 500; }
  .btn { display: inline-flex; align-items: center; gap: 6px; padding: 8px 18px; border-radius: 8px; font-size: 14px; font-weight: 500; cursor: pointer; border: none; transition: background .15s, transform .1s; }
  .btn:active { transform: scale(0.97); }
  .btn-primary { background: var(--accent); color: #fff; }
  .btn-primary:hover { background: var(--accent-hover); }
  .btn-danger { background: transparent; color: var(--danger); border: 1px solid var(--danger); }
  .btn-danger:hover { background: var(--danger); color: #fff; }
  .btn-success { background: transparent; color: var(--success); border: 1px solid var(--success); }
  .btn-success:hover { background: var(--success); color: #000; }
  .btn-ghost { background: transparent; color: var(--text-dim); border: 1px solid var(--border); }
  .btn-ghost:hover { border-color: var(--text-dim); color: var(--text); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 20px; }
  .card { background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 20px; box-shadow: var(--shadow); transition: border-color .2s; }
  .card:hover { border-color: var(--accent); }
  .card-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 14px; }
  .card-name { font-size: 17px; font-weight: 700; letter-spacing: .4px; }
  .badge-default { background: var(--accent); color: #fff; font-size: 10px; padding: 2px 8px; border-radius: 20px; font-weight: 600; letter-spacing: .5px; }
  .card-row { display: flex; flex-direction: column; gap: 3px; margin-bottom: 10px; }
  .card-label { font-size: 11px; color: var(--text-dim); text-transform: uppercase; letter-spacing: .6px; }
  .card-value { font-size: 13px; color: var(--text); word-break: break-all; }
  .card-value code { background: var(--input-bg); padding: 2px 6px; border-radius: 4px; font-family: monospace; font-size: 12px; }
  .key-row { display: flex; align-items: center; gap: 6px; }
  .key-text { font-family: monospace; font-size: 12px; color: var(--text-dim); flex: 1; }
  .icon-btn { background: none; border: none; cursor: pointer; color: var(--text-dim); padding: 2px 4px; border-radius: 4px; line-height: 1; font-size: 14px; }
  .icon-btn:hover { color: var(--text); background: var(--border); }
  .card-actions { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 16px; padding-top: 14px; border-top: 1px solid var(--border); }
  .card-actions .btn { padding: 6px 14px; font-size: 12px; }
  /* Modal */
  .modal-overlay { display: none; position: fixed; inset: 0; background: rgba(0,0,0,.7); z-index: 100; align-items: center; justify-content: center; }
  .modal-overlay.open { display: flex; }
  .modal { background: var(--card); border: 1px solid var(--border); border-radius: 16px; width: 480px; max-width: 95vw; padding: 28px; box-shadow: 0 24px 64px rgba(0,0,0,.6); }
  .modal h3 { font-size: 18px; margin-bottom: 20px; }
  .form-group { margin-bottom: 16px; }
  .form-group label { display: block; font-size: 12px; color: var(--text-dim); margin-bottom: 6px; text-transform: uppercase; letter-spacing: .5px; }
  .form-group input { width: 100%; background: var(--input-bg); border: 1px solid var(--border); color: var(--text); padding: 10px 14px; border-radius: 8px; font-size: 14px; outline: none; transition: border-color .15s; }
  .form-group input:focus { border-color: var(--accent); }
  .form-group .input-wrap { position: relative; display: flex; }
  .form-group .input-wrap input { padding-right: 40px; }
  .form-group .input-wrap button { position: absolute; right: 10px; top: 50%; transform: translateY(-50%); background: none; border: none; cursor: pointer; color: var(--text-dim); font-size: 16px; }
  .checkbox-row { display: flex; align-items: center; gap: 8px; margin-bottom: 16px; font-size: 13px; color: var(--text-dim); }
  .checkbox-row input { width: 15px; height: 15px; accent-color: var(--accent); }
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 8px; }
  /* Token modal */
  .token-modal { max-width: 420px; }
  .token-modal p { color: var(--text-dim); font-size: 13px; margin-bottom: 16px; line-height: 1.6; }
  /* Toast */
  #toast { position: fixed; bottom: 24px; right: 24px; background: var(--card); border: 1px solid var(--border); border-radius: 10px; padding: 12px 20px; font-size: 14px; box-shadow: var(--shadow); transition: opacity .3s; opacity: 0; pointer-events: none; z-index: 999; }
  #toast.show { opacity: 1; }
  #toast.ok { border-color: var(--success); color: var(--success); }
  #toast.err { border-color: var(--danger); color: var(--danger); }
  /* Loading */
  .loader { border: 3px solid var(--border); border-top-color: var(--accent); border-radius: 50%; width: 36px; height: 36px; animation: spin .7s linear infinite; margin: 60px auto; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .empty { text-align: center; color: var(--text-dim); padding: 60px 0; font-size: 15px; }
</style>
</head>
<body>

<header>
  <h1>🐉 一龙 AI 配置中心</h1>
  <span class="subtitle">管理 AI 代理参数，修改后立即生效（无需重启服务器）</span>
  <div class="token-badge">
    <span id="tokenLabel">未登录</span>
    <button onclick="showTokenModal()">更换令牌</button>
  </div>
</header>

<div class="container">
  <div class="toolbar">
    <h2 id="agentCount">已配置 0 个 AI 代理</h2>
    <button class="btn btn-primary" onclick="openAddModal()">＋ 添加代理</button>
  </div>
  <div id="agentGrid" class="grid"><div class="loader"></div></div>
</div>

<!-- 编辑/新增 Modal -->
<div class="modal-overlay" id="editModal">
  <div class="modal">
    <h3 id="modalTitle">添加 AI 代理</h3>
    <div class="form-group">
      <label>代理名称（唯一标识，如 deepseek / openai）</label>
      <input id="fName" placeholder="deepseek" autocomplete="off" />
    </div>
    <div class="form-group">
      <label>API 地址（Base URL）</label>
      <input id="fBase" placeholder="https://api.deepseek.com/v1" autocomplete="off" />
    </div>
    <div class="form-group">
      <label>API 密钥 <span style="color:var(--text-dim);font-weight:400">（留空则保留原密钥）</span></label>
      <div class="input-wrap">
        <input id="fKey" type="password" placeholder="sk-..." autocomplete="new-password" />
        <button type="button" onclick="toggleKeyVis('fKey',this)" title="显示/隐藏">👁</button>
      </div>
    </div>
    <div class="form-group">
      <label>模型名称</label>
      <input id="fModel" placeholder="deepseek-chat" autocomplete="off" />
    </div>
    <div class="checkbox-row">
      <input type="checkbox" id="fDefault" />
      <label for="fDefault">设为默认代理</label>
    </div>
    <div class="modal-actions">
      <button class="btn btn-ghost" onclick="closeEditModal()">取消</button>
      <button class="btn btn-primary" onclick="saveAgent()">保存</button>
    </div>
  </div>
</div>

<!-- 令牌 Modal -->
<div class="modal-overlay" id="tokenModal">
  <div class="modal token-modal">
    <h3>输入管理员令牌</h3>
    <p>令牌在服务器 .env 文件中通过 <code style="background:var(--input-bg);padding:2px 6px;border-radius:4px">ADMIN_TOKEN</code> 设置，默认值为 <code style="background:var(--input-bg);padding:2px 6px;border-radius:4px">elon-admin</code>。</p>
    <div class="form-group">
      <label>管理员令牌</label>
      <div class="input-wrap">
        <input id="tokenInput" type="password" placeholder="elon-admin" autocomplete="off" />
        <button type="button" onclick="toggleKeyVis('tokenInput',this)" title="显示/隐藏">👁</button>
      </div>
    </div>
    <div class="modal-actions">
      <button class="btn btn-primary" onclick="applyToken()">确认登录</button>
    </div>
  </div>
</div>

<div id="toast"></div>

<script>
// ─── 状态 ───────────────────────────────────
let token = localStorage.getItem('elon_admin_token') || '';
let editingName = null; // null = 新增，否则为被编辑的代理名

// ─── 初始化 ───────────────────────────────────
window.addEventListener('DOMContentLoaded', () => {
  if (!token) {
    showTokenModal();
  } else {
    updateTokenLabel();
    loadAgents();
  }
});

// ─── 令牌管理 ─────────────────────────────────
function showTokenModal() {
  document.getElementById('tokenInput').value = token;
  document.getElementById('tokenModal').classList.add('open');
  setTimeout(() => document.getElementById('tokenInput').focus(), 100);
}
function applyToken() {
  const v = document.getElementById('tokenInput').value.trim();
  if (!v) return toast('请输入令牌', 'err');
  token = v;
  localStorage.setItem('elon_admin_token', token);
  document.getElementById('tokenModal').classList.remove('open');
  updateTokenLabel();
  loadAgents();
}
function updateTokenLabel() {
  const masked = token.length > 6
    ? token.slice(0, 2) + '••••' + token.slice(-2)
    : '••••';
  document.getElementById('tokenLabel').textContent = '令牌: ' + masked;
}

// ─── 加载代理列表 ────────────────────────────
async function loadAgents() {
  const grid = document.getElementById('agentGrid');
  grid.innerHTML = '<div class="loader"></div>';
  try {
    const res = await apiFetch('GET', '/api/admin/agents');
    if (!res.ok) {
      const j = await res.json().catch(() => ({}));
      if (res.status === 401) { grid.innerHTML = '<p class="empty">令牌无效，请点击右上角更换令牌</p>'; return; }
      grid.innerHTML = `<p class="empty">加载失败: ${j.error || res.status}</p>`;
      return;
    }
    const data = await res.json();
    renderAgents(data);
  } catch(e) {
    grid.innerHTML = `<p class="empty">网络错误: ${e.message}</p>`;
  }
}

function renderAgents(data) {
  const grid = document.getElementById('agentGrid');
  const agents = data.agents || [];
  document.getElementById('agentCount').textContent = `已配置 ${agents.length} 个 AI 代理`;
  if (agents.length === 0) {
    grid.innerHTML = '<p class="empty">还没有配置任何代理，点击右上角「添加代理」开始</p>';
    return;
  }
  grid.innerHTML = agents.map(a => `
    <div class="card" id="card-${a.name}">
      <div class="card-header">
        <span class="card-name">${esc(a.name)}</span>
        ${a.is_default ? '<span class="badge-default">DEFAULT</span>' : ''}
      </div>
      <div class="card-row">
        <span class="card-label">API 地址</span>
        <span class="card-value">${esc(a.api_base || '未设置')}</span>
      </div>
      <div class="card-row">
        <span class="card-label">模型</span>
        <span class="card-value"><code>${esc(a.model || '未设置')}</code></span>
      </div>
      <div class="card-row">
        <span class="card-label">API 密钥</span>
        <div class="key-row">
          <span class="key-text" id="key-${a.name}">${esc(a.api_key_masked)}</span>
          <button class="icon-btn" onclick="revealKey('${esc(a.name)}')" title="查看完整密钥">👁</button>
        </div>
      </div>
      <div class="card-actions">
        <button class="btn btn-ghost" onclick="openEditModal('${esc(a.name)}')">✏️ 编辑</button>
        ${!a.is_default ? `<button class="btn btn-success" onclick="setDefault('${esc(a.name)}')">⭐ 设为默认</button>` : ''}
        <button class="btn btn-danger" onclick="deleteAgent('${esc(a.name)}')">🗑 删除</button>
      </div>
    </div>
  `).join('');
}

// ─── 查看完整 Key ───────────────────────────
async function revealKey(name) {
  const el = document.getElementById('key-' + name);
  if (el.dataset.revealed === '1') {
    el.dataset.revealed = '0';
    el.textContent = el.dataset.masked;
    return;
  }
  try {
    const res = await apiFetch('GET', `/api/admin/agents/${encodeURIComponent(name)}/key`);
    const j = await res.json();
    if (!res.ok) return toast(j.error || '获取失败', 'err');
    el.dataset.masked = el.textContent;
    el.textContent = j.api_key;
    el.dataset.revealed = '1';
  } catch(e) {
    toast('网络错误', 'err');
  }
}

// ─── 新增/编辑 Modal ─────────────────────────
function openAddModal() {
  editingName = null;
  document.getElementById('modalTitle').textContent = '添加 AI 代理';
  document.getElementById('fName').value = '';
  document.getElementById('fName').disabled = false;
  document.getElementById('fBase').value = '';
  document.getElementById('fKey').value = '';
  document.getElementById('fModel').value = '';
  document.getElementById('fDefault').checked = false;
  document.getElementById('editModal').classList.add('open');
  setTimeout(() => document.getElementById('fName').focus(), 100);
}
async function openEditModal(name) {
  editingName = name;
  document.getElementById('modalTitle').textContent = '编辑 AI 代理：' + name;
  // 从页面上读取已知信息
  const card = document.getElementById('card-' + name);
  const rows = card.querySelectorAll('.card-row');
  document.getElementById('fName').value = name;
  document.getElementById('fName').disabled = true; // 编辑时不允许改名
  document.getElementById('fBase').value = rows[0].querySelector('.card-value').textContent.trim();
  document.getElementById('fKey').value = '';
  document.getElementById('fModel').value = rows[1].querySelector('code').textContent.trim();
  document.getElementById('fDefault').checked = !!card.querySelector('.badge-default');
  document.getElementById('editModal').classList.add('open');
}
function closeEditModal() {
  document.getElementById('editModal').classList.remove('open');
}

async function saveAgent() {
  const name = document.getElementById('fName').value.trim();
  const api_base = document.getElementById('fBase').value.trim();
  const api_key = document.getElementById('fKey').value;
  const model = document.getElementById('fModel').value.trim();
  const set_as_default = document.getElementById('fDefault').checked;

  if (!name) return toast('代理名称不能为空', 'err');
  if (!api_base) return toast('API 地址不能为空', 'err');
  if (!model) return toast('模型名称不能为空', 'err');
  if (!editingName && !api_key.trim()) return toast('新增代理必须填写 API 密钥', 'err');

  try {
    const res = await apiFetch('POST', '/api/admin/agents', { name, api_base, api_key, model, set_as_default });
    const j = await res.json();
    if (!res.ok) return toast(j.error || '保存失败', 'err');
    closeEditModal();
    toast('保存成功', 'ok');
    loadAgents();
  } catch(e) {
    toast('网络错误', 'err');
  }
}

// ─── 删除代理 ────────────────────────────────
async function deleteAgent(name) {
  if (!confirm(`确定删除代理 "${name}" 吗？此操作无法撤销。`)) return;
  try {
    const res = await apiFetch('DELETE', `/api/admin/agents/${encodeURIComponent(name)}`);
    const j = await res.json();
    if (!res.ok) return toast(j.error || '删除失败', 'err');
    toast(`已删除代理 "${name}"`, 'ok');
    loadAgents();
  } catch(e) {
    toast('网络错误', 'err');
  }
}

// ─── 设置默认代理 ───────────────────────────
async function setDefault(name) {
  try {
    const res = await apiFetch('POST', `/api/admin/default/${encodeURIComponent(name)}`);
    const j = await res.json();
    if (!res.ok) return toast(j.error || '设置失败', 'err');
    toast(`已将 "${name}" 设为默认代理`, 'ok');
    loadAgents();
  } catch(e) {
    toast('网络错误', 'err');
  }
}

// ─── 工具函数 ────────────────────────────────
function apiFetch(method, path, body) {
  return fetch(path, {
    method,
    headers: {
      'Authorization': 'Bearer ' + token,
      'Content-Type': 'application/json',
    },
    body: body ? JSON.stringify(body) : undefined,
  });
}

function toggleKeyVis(id, btn) {
  const el = document.getElementById(id);
  if (el.type === 'password') { el.type = 'text'; btn.textContent = '🙈'; }
  else { el.type = 'password'; btn.textContent = '👁'; }
}

let _toastTimer;
function toast(msg, type) {
  const el = document.getElementById('toast');
  el.textContent = msg;
  el.className = 'show ' + (type || '');
  clearTimeout(_toastTimer);
  _toastTimer = setTimeout(() => el.className = '', 3000);
}

function esc(s) {
  return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;').replace(/'/g,'&#39;');
}

// Enter 键支持
document.addEventListener('keydown', e => {
  if (e.key === 'Enter') {
    if (document.getElementById('tokenModal').classList.contains('open')) applyToken();
    else if (document.getElementById('editModal').classList.contains('open')) saveAgent();
  }
  if (e.key === 'Escape') {
    closeEditModal();
    document.getElementById('tokenModal').classList.remove('open');
  }
});
</script>
</body>
</html>"#;
