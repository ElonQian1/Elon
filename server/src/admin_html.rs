//! 嵌入式 Admin 控制台静态 HTML/CSS/JS 资源（从 admin.rs 抽出）。
//! 
//! 这里只是一个超大的内嵌字符串常量，没有任何运行时逻辑；放在独立模块里
//! 让 admin.rs 只保留鉴权与 HTTP handlers。

pub(crate) 
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
  /* Tabs */
  .tab-bar { display: flex; gap: 4px; background: var(--card); border-bottom: 1px solid var(--border); padding: 0 32px; }
  .tab-btn { padding: 12px 20px; font-size: 14px; font-weight: 500; color: var(--text-dim); background: none; border: none; border-bottom: 2px solid transparent; cursor: pointer; transition: color .15s; }
  .tab-btn:hover { color: var(--text); }
  .tab-btn.active { color: var(--accent); border-bottom-color: var(--accent); }
  .tab-pane { display: none; }
  .tab-pane.active { display: block; }
  /* User cards */
  .user-card { background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 16px 20px; display: flex; flex-wrap: wrap; align-items: center; gap: 12px; }
  .user-card:hover { border-color: var(--accent); }
  .user-id { font-size: 14px; font-weight: 700; min-width: 160px; }
  .user-tag { font-size: 11px; padding: 3px 10px; border-radius: 20px; }
  .tag-custom { background: rgba(108,99,255,.2); color: var(--accent); }
  .tag-default { background: var(--border); color: var(--text-dim); }
  .user-detail { font-size: 12px; color: var(--text-dim); margin-left: auto; }
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

<div class="tab-bar">
  <button class="tab-btn active" onclick="switchTab('agents',this)">🤖 AI 代理</button>
  <button class="tab-btn" onclick="switchTab('users',this)">👥 用户列表</button>
  <button class="tab-btn" onclick="switchTab('projects',this)">📦 用户项目</button>
</div>

<!-- ── AI 代理标签页 ──────────────────────── -->
<div id="tab-agents" class="tab-pane active">
<div class="container">
  <div class="toolbar">
    <h2 id="agentCount">已配置 0 个 AI 代理</h2>
    <button class="btn btn-primary" onclick="openAddModal()">＋ 添加代理</button>
  </div>
  <div id="agentGrid" class="grid"><div class="loader"></div></div>
</div>
</div>

<!-- ── 用户列表标签页 ─────────────────────── -->
<div id="tab-users" class="tab-pane">
<div class="container">
  <div class="toolbar">
    <h2 id="userCount">用户列表</h2>
    <div style="display:flex;gap:10px;flex-wrap:wrap">
      <button class="btn btn-primary" onclick="openUserModal()">＋ 创建用户</button>
      <button class="btn btn-ghost" onclick="loadUsers()">↻ 刷新</button>
    </div>
  </div>
  <div id="userList" style="display:flex;flex-direction:column;gap:10px"><div class="loader"></div></div>
</div>
</div>

<!-- ── 用户项目标签页 ─────────────────────── -->
<div id="tab-projects" class="tab-pane">
<div class="container">
  <div class="toolbar">
    <h2 id="projectCount">用户项目列表</h2>
    <button class="btn btn-ghost" onclick="loadProjects()">↻ 刷新</button>
  </div>
  <div id="projectTableWrap" style="overflow-x:auto">
    <table id="projectTable" style="width:100%;border-collapse:collapse;font-size:13px">
      <thead>
        <tr style="background:var(--card);color:var(--text-dim);text-align:left">
          <th style="padding:8px 12px">项目名</th>
          <th style="padding:8px 12px">创建者</th>
          <th style="padding:8px 12px">设备 / APK版本</th>
          <th style="padding:8px 12px">类型/模板</th>
          <th style="padding:8px 12px">服务器路径</th>
          <th style="padding:8px 12px">任务状态</th>
          <th style="padding:8px 12px">APK 下载</th>
          <th style="padding:8px 12px">更新时间</th>
        </tr>
      </thead>
      <tbody id="projectTableBody"><tr><td colspan="8" style="text-align:center;padding:24px"><div class="loader"></div></td></tr></tbody>
    </table>
  </div>
</div>
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

<!-- 创建用户 Modal -->
<div class="modal-overlay" id="userModal">
  <div class="modal">
    <h3>创建用户</h3>
    <div class="form-group">
      <label>账号（手机号或邮箱）</label>
      <input id="uAccount" placeholder="friend@example.com" autocomplete="off" />
    </div>
    <div class="form-group">
      <label>昵称</label>
      <input id="uNickname" placeholder="小王" autocomplete="off" />
    </div>
    <div class="form-group">
      <label>初始密码</label>
      <div class="input-wrap">
        <input id="uPassword" type="password" placeholder="至少 6 位" autocomplete="new-password" />
        <button type="button" onclick="toggleKeyVis('uPassword',this)" title="显示/隐藏">👁</button>
      </div>
    </div>
    <div class="form-group">
      <label>角色</label>
      <input id="uRole" value="user" autocomplete="off" />
    </div>
    <div class="modal-actions">
      <button class="btn btn-ghost" onclick="closeUserModal()">取消</button>
      <button class="btn btn-primary" onclick="createUser()">创建</button>
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
    else if (document.getElementById('userModal').classList.contains('open')) createUser();
    else if (document.getElementById('editModal').classList.contains('open')) saveAgent();
  }
  if (e.key === 'Escape') {
    closeEditModal();
    closeUserModal();
    document.getElementById('tokenModal').classList.remove('open');
  }
});

// ─── 标签切换 ────────────────────────────
function switchTab(name, btn) {
  document.querySelectorAll('.tab-pane').forEach(el => el.classList.remove('active'));
  document.querySelectorAll('.tab-btn').forEach(el => el.classList.remove('active'));
  document.getElementById('tab-' + name).classList.add('active');
  btn.classList.add('active');
  if (name === 'users') loadUsers();
  if (name === 'projects') loadProjects();
}

// ─── 用户列表 ────────────────────────────
function openUserModal() {
  document.getElementById('uAccount').value = '';
  document.getElementById('uNickname').value = '';
  document.getElementById('uPassword').value = '';
  document.getElementById('uRole').value = 'user';
  document.getElementById('userModal').classList.add('open');
  setTimeout(() => document.getElementById('uAccount').focus(), 100);
}
function closeUserModal() {
  document.getElementById('userModal').classList.remove('open');
}
async function createUser() {
  const account = document.getElementById('uAccount').value.trim();
  const nickname = document.getElementById('uNickname').value.trim();
  const password = document.getElementById('uPassword').value;
  const role = document.getElementById('uRole').value.trim() || 'user';
  if (!account) return toast('账号不能为空', 'err');
  if (password.length < 6) return toast('密码至少 6 位', 'err');
  try {
    const res = await apiFetch('POST', '/api/admin/users', { account, nickname, password, role });
    const j = await res.json();
    if (!res.ok) return toast(j.error || '创建失败', 'err');
    closeUserModal();
    toast('用户已创建', 'ok');
    loadUsers();
  } catch(e) {
    toast('网络错误', 'err');
  }
}
async function loadUsers() {
  const list = document.getElementById('userList');
  list.innerHTML = '<div class="loader"></div>';
  try {
    const res = await apiFetch('GET', '/api/admin/users');
    if (!res.ok) {
      const j = await res.json().catch(() => ({}));
      list.innerHTML = `<p class="empty">${j.error || '加载失败'}</p>`;
      return;
    }
    const data = await res.json();
    renderUsers(data);
  } catch(e) {
    list.innerHTML = `<p class="empty">网络错误: ${e.message}</p>`;
  }
}

// ─── 用户项目 ────────────────────────────
async function loadProjects() {
  const tbody = document.getElementById('projectTableBody');
  tbody.innerHTML = '<tr><td colspan="7" style="text-align:center;padding:24px"><div class="loader"></div></td></tr>';
  try {
    const res = await apiFetch('GET', '/api/admin/projects');
    if (!res.ok) {
      const j = await res.json().catch(() => ({}));
      tbody.innerHTML = `<tr><td colspan="7" style="padding:20px;color:#e76f51">${esc(j.error || '加载失败')}</td></tr>`;
      return;
    }
    const data = await res.json();
    renderProjects(data);
  } catch(e) {
    tbody.innerHTML = `<tr><td colspan="7" style="padding:20px;color:#e76f51">网络错误: ${esc(e.message)}</td></tr>`;
  }
}

function renderProjects(data) {
  const tbody = document.getElementById('projectTableBody');
  const projects = data.projects || [];
  document.getElementById('projectCount').textContent = `共 ${projects.length} 个用户项目`;
  if (projects.length === 0) {
    tbody.innerHTML = '<tr><td colspan="8" style="text-align:center;padding:32px;color:var(--text-dim)">暂无项目</td></tr>';
    return;
  }
  const statusColor = s => s === 'active' ? '#52b788' : s === 'error' ? '#e76f51' : '#aaa';
  tbody.innerHTML = projects.map(p => {
    const apkCell = p.last_apk_url
      ? `<a href="${esc(p.last_apk_url)}" target="_blank" style="color:var(--accent)">⬇ 下载</a>`
      : '<span style="color:var(--text-dim)">—</span>';
    const taskStatus = p.last_task_status
      ? `<span style="color:${statusColor(p.last_task_status)}">${esc(p.last_task_status)}</span>`
      : '<span style="color:var(--text-dim)">—</span>';
    const path = p.workspace_dir || '—';
    const typeLabel = [p.source_type, p.template].filter(Boolean).join(' / ');
    const deviceCell = p.last_device_name
      ? `${esc(p.last_device_name)}<br><span style="font-size:11px;color:var(--text-dim)">${esc(p.last_apk_version || '')}</span>`
      : '<span style="color:var(--text-dim)">—</span>';
    return `<tr style="border-top:1px solid var(--border)">
      <td style="padding:8px 12px;font-weight:500">${esc(p.name)}<br><span style="font-size:11px;color:var(--text-dim)">${esc(p.id)}</span></td>
      <td style="padding:8px 12px">${esc(p.created_by_account)}</td>
      <td style="padding:8px 12px">${deviceCell}</td>
      <td style="padding:8px 12px">${esc(typeLabel)}</td>
      <td style="padding:8px 12px;font-size:11px;word-break:break-all;max-width:240px">${esc(path)}</td>
      <td style="padding:8px 12px">${taskStatus}</td>
      <td style="padding:8px 12px">${apkCell}</td>
      <td style="padding:8px 12px;font-size:12px;color:var(--text-dim)">${esc(p.updated_at || '')}</td>
    </tr>`;
  }).join('');
}

function renderUsers(data) {
  const list = document.getElementById('userList');
  const users = data.users || [];
  document.getElementById('userCount').textContent = `共 ${users.length} 个用户`;
  if (users.length === 0) {
    list.innerHTML = '<p class="empty">暂无用户，点击「创建用户」添加朋友账号</p>';
    return;
  }
  list.innerHTML = users.map(u => {
    const nicknameText = u.nickname ? ` <span style="color:var(--accent)">${esc(u.nickname)}</span>` : '';
    const updatedText = u.updated_at ? `更新: ${esc(u.updated_at)}` : '';
    const projectText = `${Number(u.project_count || 0)} 个项目`;
    const roleText = `${esc(u.role || 'user')} · ${esc(u.status || '')}`;
    return `<div class="user-card">
      <span class="user-id">${esc(u.account || u.id)}${nicknameText}</span>
      <span class="user-tag tag-custom">${projectText}</span>
      <span class="user-tag tag-default">${roleText}</span>
      <span class="user-detail">${updatedText}</span>
    </div>`;
  }).join('');
}
</script>
</body>
</html>"#;
