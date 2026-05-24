// 网页版（APK 桌面投影）
//
// 设计原则：APK 是真理来源，网页只是把 APK 投影到浏览器上。
// 视觉、交互、文案对齐 android/app/src/main/res/layout/activity_main.xml。
// 响应式断点：< 720 手机、720~1100 平板、>= 1100 桌面（左侧栏 Tab）。

use std::sync::{Arc, OnceLock};

use axum::{extract::State, response::Html};

use crate::types::AppState;

const BRAND_PNG_B64: &str = include_str!("assets/ic_app_brand.b64");
const TAB_CHAT_PNG_B64: &str = include_str!("assets/ic_tab_chat_edit.b64");
const TAB_PROJECT_PNG_B64: &str = include_str!("assets/ic_tab_project_stack.b64");

pub async fn web_page() -> Html<&'static str> {
    static HTML: OnceLock<String> = OnceLock::new();
    Html(HTML.get_or_init(build_html).as_str())
}

pub async fn download_page(State(state): State<Arc<AppState>>) -> Html<String> {
    let public_url = state.public_url.trim_end_matches('/');
    let apk_url = format!("{public_url}/app/ElonSpeed-latest.apk");
    let page_url = format!("{public_url}/app/download");
    Html(
        DOWNLOAD_HTML_TEMPLATE
            .replace("__APK_URL__", &apk_url)
            .replace("__PAGE_URL__", &page_url)
            .replace("__BRAND_PNG_B64__", BRAND_PNG_B64.trim()),
    )
}

fn build_html() -> String {
    WEB_HTML_TEMPLATE
        .replace("__BRAND_PNG_B64__", BRAND_PNG_B64.trim())
        .replace("__TAB_CHAT_PNG_B64__", TAB_CHAT_PNG_B64.trim())
        .replace("__TAB_PROJECT_PNG_B64__", TAB_PROJECT_PNG_B64.trim())
}

const WEB_HTML_TEMPLATE: &str = r###"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
  <meta name="theme-color" content="#101010" />
  <title>一龙 · 云端开发</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #101010;
      --bg-hint: #1c1c1c;
      --panel: #1e1e1e;
      --panel-2: #242424;
      --panel-3: #2a2a2a;
      --line: #343434;
      --ink: #d0d0d0;
      --ink-strong: #ededed;
      --ink-soft: #b8b8b8;
      --ink-muted: #a9a9a9;
      --ink-faint: #8e8e8e;
      --ink-dim: #505050;
      --brand: #07c160;
      --brand-hover: #06ad55;
      --bubble-user: #95ec69;
      --bubble-user-ink: #111111;
      --bubble-ai-bg: rgba(255,255,255,0.19);
      --bubble-ai-border: rgba(255,255,255,0.18);
      --bubble-ai-ink: #f4f4f4;
      --bubble-progress-bg: rgba(255,255,255,0.15);
      --bubble-progress-border: rgba(255,255,255,0.2);
      --bubble-progress-ink: #9a9a9a;
      --bubble-error-bg: #fff1f0;
      --bubble-error-border: #ffd6d2;
      --bubble-error-ink: #c62828;
      --tab-icon: #b8b8b8;
      --tab-active: #d0d0d0;
    }
    * { box-sizing: border-box; }
    html, body {
      margin: 0; padding: 0;
      height: 100%;
      background: var(--bg);
      color: var(--ink);
      font-family: -apple-system, BlinkMacSystemFont, "PingFang SC", "Microsoft YaHei", "Hiragino Sans GB", sans-serif;
      font-size: 15px;
      -webkit-font-smoothing: antialiased;
    }
    button, input, textarea, select { font: inherit; color: inherit; }
    a { color: var(--brand); text-decoration: none; }
    a:hover { text-decoration: underline; }
    .hidden { display: none !important; }

    /* ===== 整体布局 ===== */
    .app {
      display: grid;
      min-height: 100vh;
      grid-template-rows: 50px 1fr auto 52px;
      grid-template-columns: 1fr;
      grid-template-areas:
        "toolbar"
        "content"
        "input"
        "tabs";
    }
    /* 平板 / 桌面：限定宽度居中，Tab 始终在底部（与 APK 一致） */
    @media (min-width: 720px) {
      .app { max-width: 720px; margin: 0 auto; box-shadow: 0 0 0 1px #1a1a1a; }
    }

    /* ===== Toolbar ===== */
    .toolbar {
      grid-area: toolbar;
      height: 50px;
      background: var(--bg);
      display: flex;
      align-items: center;
      padding: 0 4px;
    }
    .toolbar h1 {
      flex: 1;
      text-align: center;
      margin: 0;
      font-size: 17px;
      font-weight: normal;
      color: var(--ink);
    }
    .icon-btn {
      width: 44px; height: 44px;
      background: transparent;
      border: 0;
      padding: 9px;
      color: var(--ink-soft);
      cursor: pointer;
      border-radius: 22px;
    }
    .icon-btn:hover { background: rgba(255,255,255,0.06); color: var(--ink); }
    .icon-btn svg { width: 100%; height: 100%; display: block; }

    /* ===== Content ===== */
    .content {
      grid-area: content;
      background: var(--bg);
      position: relative;
      overflow: hidden;
      min-height: 0;
    }
    .page {
      position: absolute;
      inset: 0;
      overflow-y: auto;
      display: none;
      flex-direction: column;
    }
    .page.active { display: flex; }

    /* ===== 会话页 ===== */
    .conversation-list { flex: 0 0 auto; }
    .conversation-item {
      height: 66px;
      background: var(--panel-2);
      display: flex;
      align-items: center;
      padding: 0 14px;
      gap: 10px;
      cursor: pointer;
      border: 0;
      width: 100%;
      text-align: left;
      color: inherit;
      border-bottom: 1px solid #1a1a1a;
    }
    .conversation-item:hover { background: #2c2c2c; }
    .conversation-item.active { background: #2f2f2f; }
    .conversation-item .brand {
      width: 44px; height: 44px;
      flex: 0 0 44px;
      border-radius: 6px;
      overflow: hidden;
      background: var(--panel);
      display: flex; align-items: center; justify-content: center;
    }
    .conversation-item .brand img { width: 100%; height: 100%; object-fit: cover; }
    .conversation-item .text { flex: 1; min-width: 0; }
    .conversation-item .title {
      font-size: 16px; color: var(--ink);
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    .conversation-item .sub {
      font-size: 13px; color: var(--ink-muted);
      margin-top: 4px;
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    .conversation-item .time {
      font-size: 12px; color: #c4c4c4;
      align-self: flex-start; margin-top: 16px;
      flex-shrink: 0; padding-left: 6px;
    }
    .empty-tip {
      padding: 80px 24px;
      text-align: center;
      color: var(--ink-muted);
      font-size: 14px;
      line-height: 1.7;
    }
    .empty-tip button {
      margin-top: 16px;
      height: 38px; padding: 0 18px;
      background: var(--brand); color: #fff;
      border: 0; border-radius: 6px; cursor: pointer;
    }

    /* ===== 聊天视图 ===== */
    .chat-view { flex: 1; display: flex; flex-direction: column; min-height: 0; }
    .stage-hint {
      flex: 0 0 auto;
      min-height: 42px;
      background: var(--bg-hint);
      color: var(--ink-soft);
      font-size: 13px;
      padding: 0 16px;
      display: flex; align-items: center;
    }
    .chat-list {
      flex: 1; overflow-y: auto;
      padding: 10px 10px;
      display: flex; flex-direction: column;
      gap: 10px;
    }
    .bubble-row { display: flex; align-items: flex-end; gap: 8px; }
    .bubble-row.user { justify-content: flex-end; }
    .bubble {
      max-width: 76%;
      padding: 10px 14px;
      font-size: 15px;
      line-height: 1.55;
      border-radius: 8px;
      word-wrap: break-word;
      white-space: pre-wrap;
    }
    .bubble.user {
      background: var(--bubble-user);
      color: var(--bubble-user-ink);
      border-top-right-radius: 2px;
    }
    .bubble.ai {
      background: var(--bubble-ai-bg);
      color: var(--bubble-ai-ink);
      border: 1px solid var(--bubble-ai-border);
      border-top-left-radius: 2px;
    }
    .bubble.progress {
      background: var(--bubble-progress-bg);
      color: var(--bubble-progress-ink);
      border: 1px solid var(--bubble-progress-border);
      font-size: 13px;
      max-width: 86%;
    }
    .bubble.error {
      background: var(--bubble-error-bg);
      color: var(--bubble-error-ink);
      border: 1px solid var(--bubble-error-border);
    }
    .bubble .meta { display: block; font-size: 11px; opacity: 0.7; margin-bottom: 4px; }
    .bubble a { color: inherit; text-decoration: underline; }

    @media (min-width: 720px) {
      .bubble { max-width: 70%; }
    }

    /* ===== 项目页 ===== */
    .project-page > * + * { margin-top: 1px; }
    .project-stage {
      min-height: 64px;
      background: var(--panel-2);
      display: flex; align-items: center;
      padding: 12px 22px;
      font-size: 18px; color: var(--ink);
    }
    .project-overview {
      background: var(--panel);
      padding: 22px;
      color: var(--ink);
      font-size: 15px;
      line-height: 1.75;
      white-space: pre-wrap;
      min-height: 104px;
    }
    .project-block {
      background: var(--panel);
      padding: 18px 22px;
      margin-top: 10px !important;
    }
    .project-block h3 { margin: 0 0 4px; font-size: 16px; color: var(--ink); font-weight: normal; }
    .stage-line { padding: 8px 0; font-size: 14px; color: var(--ink-soft); }
    .project-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 10px;
      background: var(--panel);
      padding: 12px;
      margin-top: 10px !important;
    }
    .project-grid button {
      height: 54px;
      background: var(--panel-3);
      color: var(--ink);
      border: 0; border-radius: 6px;
      font-size: 15px; cursor: pointer;
    }
    .project-grid button:hover { background: #353535; }
    .project-history {
      background: var(--panel);
      padding: 22px;
      margin-top: 10px !important;
      color: var(--ink-soft);
      font-size: 13px;
      min-height: 160px;
      line-height: 1.75;
      white-space: pre-wrap;
    }

    /* ===== 我的页 ===== */
    .profile-header {
      background: var(--panel-2);
      padding: 22px;
      color: var(--ink);
      font-size: 15px;
      line-height: 1.75;
      min-height: 112px;
      white-space: pre-wrap;
    }
    .profile-row {
      min-height: 64px;
      background: var(--panel);
      display: flex; align-items: center; justify-content: space-between;
      padding: 0 22px;
      color: var(--ink);
      font-size: 16px;
      cursor: pointer;
      margin-top: 1px;
      border: 0; width: 100%; text-align: left;
    }
    .profile-row:first-of-type { margin-top: 10px; }
    .profile-row:hover { background: #252525; }
    .profile-row .arrow { color: var(--ink-faint); font-size: 18px; }
    .profile-row.danger { color: #d97a7a; }
    .profile-version {
      text-align: center;
      color: var(--ink-dim);
      font-size: 12px;
      padding: 20px 0 32px;
    }

    /* ===== 输入栏 ===== */
    .input-bar {
      grid-area: input;
      background: var(--panel);
      display: none;
      padding: 8px 10px;
      gap: 8px;
      align-items: flex-end;
    }
    .input-bar.active { display: flex; }
    .input-bar textarea {
      flex: 1;
      background: var(--panel-3);
      border: 1px solid var(--line);
      border-radius: 6px;
      color: var(--ink-strong);
      padding: 8px 14px;
      font-size: 15px;
      min-height: 42px;
      max-height: 120px;
      resize: none;
      outline: none;
      line-height: 1.4;
    }
    .input-bar textarea::placeholder { color: var(--ink-faint); }
    .input-bar .model-btn {
      width: 52px; height: 42px;
      background: var(--panel-3);
      border: 0;
      color: var(--ink);
      font-size: 13px;
      border-radius: 6px;
      cursor: pointer;
      flex-shrink: 0;
    }
    .input-bar .model-btn:hover { background: #353535; }
    .input-bar .send-btn {
      width: 64px; height: 42px;
      background: var(--brand);
      border: 0;
      color: #fff;
      font-size: 15px;
      border-radius: 6px;
      cursor: pointer;
      flex-shrink: 0;
    }
    .input-bar .send-btn:hover { background: var(--brand-hover); }
    .input-bar .send-btn:disabled { opacity: 0.55; cursor: not-allowed; }

    /* ===== Tab 栏 ===== */
    .tabs-bar {
      grid-area: tabs;
      background: var(--panel);
      display: flex;
      align-items: stretch;
    }
    .tab {
      flex: 1;
      background: transparent;
      border: 0;
      color: var(--tab-icon);
      font-size: 11px;
      display: flex; flex-direction: column;
      align-items: center; justify-content: center;
      gap: 2px;
      padding: 5px 0 4px;
      cursor: pointer;
    }
    .tab.active { color: var(--tab-active); }
    .tab .ic { width: 24px; height: 24px; display: block; }
    .tab .ic img { width: 100%; height: 100%; object-fit: contain; }
    .tab .ic svg { width: 100%; height: 100%; }

    /* ===== 模态对话框 ===== */
    .modal-mask {
      position: fixed; inset: 0;
      background: rgba(0,0,0,0.6);
      display: none;
      align-items: center; justify-content: center;
      padding: 24px;
      z-index: 100;
    }
    .modal-mask.active { display: flex; }
    .modal {
      background: var(--panel);
      border-radius: 10px;
      padding: 24px 22px;
      width: 100%; max-width: 420px;
      display: grid; gap: 14px;
    }
    .modal h2 { margin: 0; font-size: 17px; color: var(--ink); font-weight: normal; }
    .modal label { display: block; font-size: 13px; color: var(--ink-soft); margin-bottom: 6px; }
    .modal input, .modal textarea, .modal select {
      width: 100%;
      background: var(--panel-3);
      border: 1px solid var(--line);
      border-radius: 6px;
      color: var(--ink-strong);
      padding: 9px 12px;
      outline: none;
      font-size: 14px;
    }
    .modal textarea { min-height: 72px; resize: vertical; }
    .modal .actions { display: flex; gap: 10px; justify-content: flex-end; }
    .modal .btn-cancel, .modal .btn-confirm {
      height: 38px; padding: 0 16px;
      border: 0; border-radius: 6px;
      font-size: 14px; cursor: pointer;
    }
    .modal .btn-cancel { background: var(--panel-3); color: var(--ink); }
    .modal .btn-confirm { background: var(--brand); color: #fff; }
    .modal .btn-confirm:disabled { opacity: 0.55; cursor: not-allowed; }
    .modal .error-text { font-size: 13px; color: var(--bubble-error-ink); min-height: 18px; }

    /* ===== 登录页 ===== */
    .login {
      min-height: 100vh;
      display: flex; align-items: center; justify-content: center;
      padding: 24px;
      background: var(--bg);
    }
    .login-card {
      width: 100%; max-width: 380px;
      background: var(--panel);
      border-radius: 10px;
      padding: 28px 24px;
      display: grid; gap: 14px;
    }
    .login-card .brand-row { display: flex; align-items: center; gap: 12px; }
    .login-card .brand-row .icon { width: 44px; height: 44px; border-radius: 8px; overflow: hidden; }
    .login-card .brand-row .icon img { width: 100%; height: 100%; }
    .login-card .brand-row .name { font-size: 18px; color: var(--ink); }
    .login-card .brand-row .name small { display: block; font-size: 12px; color: var(--ink-faint); margin-top: 2px; }
    .auth-tabs { display: flex; background: var(--panel-3); border-radius: 6px; padding: 3px; }
    .auth-tab {
      flex: 1; height: 36px;
      background: transparent;
      border: 0;
      color: var(--ink-soft);
      border-radius: 4px;
      cursor: pointer;
    }
    .auth-tab.active { background: var(--panel-2); color: var(--ink-strong); }
    .login-card label { display: block; font-size: 13px; color: var(--ink-soft); margin-bottom: 6px; }
    .login-card input {
      width: 100%; height: 40px;
      background: var(--panel-3);
      border: 1px solid var(--line);
      border-radius: 6px;
      color: var(--ink-strong);
      padding: 0 12px;
      outline: none;
    }
    .login-card input::placeholder { color: var(--ink-faint); }
    .login-card .submit {
      height: 42px;
      background: var(--brand);
      border: 0; color: #fff;
      font-size: 15px;
      border-radius: 6px;
      cursor: pointer;
    }
    .login-card .submit:disabled { opacity: 0.55; }
    .login-card .error { font-size: 13px; color: var(--bubble-error-ink); min-height: 18px; }
    .login-card .hint { font-size: 12px; color: var(--ink-faint); text-align: center; }
  </style>
</head>
<body>

<!-- ============ 登录视图 ============ -->
<section id="loginView" class="login">
  <form id="loginForm" class="login-card">
    <div class="brand-row">
      <div class="icon"><img src="data:image/png;base64,__BRAND_PNG_B64__" alt="一龙" /></div>
      <div class="name">一龙<small>云端 APK 开发平台</small></div>
    </div>
    <div class="auth-tabs">
      <button type="button" class="auth-tab active" data-auth-mode="login">登录</button>
      <button type="button" class="auth-tab" data-auth-mode="register">注册</button>
    </div>
    <div>
      <label for="accountInput">账号</label>
      <input id="accountInput" autocomplete="username" placeholder="手机号、邮箱或账号 ID" />
    </div>
    <div id="nicknameField" class="hidden">
      <label for="nicknameInput">昵称</label>
      <input id="nicknameInput" autocomplete="nickname" placeholder="工作台展示名" />
    </div>
    <div>
      <label for="passwordInput">密码</label>
      <input id="passwordInput" type="password" autocomplete="current-password" placeholder="至少 6 位" />
    </div>
    <div id="loginError" class="error"></div>
    <button id="loginBtn" class="submit" type="submit">登录</button>
    <div class="hint">网页版是 APK 版在电脑上的体现，所有数据与 APK 互通。</div>
  </form>
</section>

<!-- ============ 主视图 ============ -->
<main id="appView" class="app hidden">
  <header class="toolbar">
    <button class="icon-btn" id="backBtn" title="返回" style="display:none">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 6l-6 6 6 6"/></svg>
    </button>
    <h1 id="topTitle">会话区</h1>
    <button class="icon-btn" id="searchBtn" title="搜索">
      <svg viewBox="0 0 26 26" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M11.6 5.8a5.8 5.8 0 1 0 0.01 0"/><path d="M16.1 16.1 L21.8 21.8"/></svg>
    </button>
    <button class="icon-btn" id="addBtn" title="新建项目">
      <svg viewBox="0 0 26 26" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><circle cx="13" cy="13" r="9.4"/><path d="M13 8.5 L13 17.5 M8.5 13 L17.5 13"/></svg>
    </button>
  </header>

  <section class="content">

    <!-- 会话页：会话列表（默认）/ 进入项目后切到聊天视图 -->
    <div id="chatPage" class="page active">
      <!-- 列表状态 -->
      <div id="conversationList" class="conversation-list"></div>
      <!-- 项目内聊天视图 -->
      <div id="chatView" class="chat-view hidden">
        <div class="stage-hint" id="stageHint">点击输入需求，开发进度会自动记录到项目页。</div>
        <div class="chat-list" id="chatList"></div>
      </div>
    </div>

    <!-- 项目页 -->
    <div id="projectPage" class="page project-page">
      <div class="project-stage" id="currentStageText">待提交需求</div>
      <div class="project-overview" id="projectOverviewText">项目管理
当前没有正在执行的开发任务。</div>
      <div class="project-block">
        <h3>开发进度</h3>
        <div class="stage-line" id="stagePlanText">1. 需求分析：等待</div>
        <div class="stage-line" id="stageCodeText">2. 开发实现：等待</div>
        <div class="stage-line" id="stageBuildText">3. 编译打包：等待</div>
        <div class="stage-line" id="stageDeliverText">4. 交付下载：等待</div>
      </div>
      <div class="project-grid">
        <button id="projectContinueBtn">继续开发</button>
        <button id="projectBuildBtn">生成 APK</button>
        <button id="projectRecordBtn">进度记录</button>
        <button id="projectSettingsBtn">模型设置</button>
      </div>
      <div class="project-history" id="projectHistoryText">暂无进度记录</div>
    </div>

    <!-- 我的页 -->
    <div id="profilePage" class="page">
      <div class="profile-header" id="userInfoText">我的开发工作台</div>
      <button class="profile-row" id="aiSettingsRow">
        <span>AI 代理设置</span><span class="arrow">›</span>
      </button>
      <button class="profile-row" id="checkUpdateRow">
        <span>检查更新</span><span class="arrow" id="updateArrow">›</span>
      </button>
      <button class="profile-row danger" id="logoutRow">
        <span>退出登录</span><span class="arrow">›</span>
      </button>
      <div class="profile-version" id="versionText">一龙网页版</div>
    </div>

  </section>

  <form id="inputBar" class="input-bar">
    <textarea id="messageInput" placeholder="描述你想开发的 App 功能" rows="1"></textarea>
    <button class="model-btn" type="button" id="modelBtn" title="选择模型">默认</button>
    <button class="send-btn" type="submit" id="sendBtn">发送</button>
  </form>

  <nav class="tabs-bar">
    <button class="tab active" data-tab="chatPage" data-title="会话区">
      <span class="ic"><img src="data:image/png;base64,__TAB_CHAT_PNG_B64__" alt="" /></span>
      <span>会话</span>
    </button>
    <button class="tab" data-tab="projectPage" data-title="项目">
      <span class="ic"><img src="data:image/png;base64,__TAB_PROJECT_PNG_B64__" alt="" /></span>
      <span>项目</span>
    </button>
    <button class="tab" data-tab="profilePage" data-title="我的">
      <span class="ic">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.55" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="10.3" r="4.1"/>
          <path d="M5.5 20c0.7 -3.2 3 -5 6.5 -5s5.8 1.8 6.5 5"/>
        </svg>
      </span>
      <span>我的</span>
    </button>
  </nav>
</main>

<!-- ============ 新建项目对话框 ============ -->
<div class="modal-mask" id="newProjectMask">
  <form class="modal" id="newProjectForm">
    <h2>新建项目</h2>
    <div>
      <label for="projectNameInput">项目名</label>
      <input id="projectNameInput" placeholder="例如：记账小助手" required maxlength="40" />
    </div>
    <div>
      <label for="projectDescInput">一句话描述（可选）</label>
      <textarea id="projectDescInput" placeholder="想做什么样的 App？给 AI 一点上下文"></textarea>
    </div>
    <div>
      <label for="templateSelect">起步模板</label>
      <select id="templateSelect">
        <option value="android_kotlin">Android（Kotlin）</option>
        <option value="android_compose">Android（Compose）</option>
        <option value="empty">空白项目</option>
      </select>
    </div>
    <div class="error-text" id="newProjectError"></div>
    <div class="actions">
      <button type="button" class="btn-cancel" id="cancelNewProjectBtn">取消</button>
      <button type="submit" class="btn-confirm" id="createProjectBtn">创建</button>
    </div>
  </form>
</div>

<!-- ============ AI 代理设置对话框 ============ -->
<div class="modal-mask" id="agentMask">
  <form class="modal" id="agentForm">
    <h2>AI 代理设置</h2>
    <div>
      <label for="agentSelect">模型</label>
      <select id="agentSelect"><option value="">默认模型</option></select>
    </div>
    <div class="error-text" id="agentError"></div>
    <div class="actions">
      <button type="button" class="btn-cancel" id="cancelAgentBtn">关闭</button>
    </div>
  </form>
</div>

<script>
(function () {
  'use strict';

  // ====== DOM 引用 ======
  const $ = (id) => document.getElementById(id);
  const loginView = $('loginView');
  const appView = $('appView');
  const loginForm = $('loginForm');
  const loginBtn = $('loginBtn');
  const loginError = $('loginError');
  const accountInput = $('accountInput');
  const nicknameInput = $('nicknameInput');
  const nicknameField = $('nicknameField');
  const passwordInput = $('passwordInput');
  const authModeButtons = document.querySelectorAll('[data-auth-mode]');

  const topTitle = $('topTitle');
  const backBtn = $('backBtn');
  const addBtn = $('addBtn');
  const searchBtn = $('searchBtn');

  const conversationList = $('conversationList');
  const chatView = $('chatView');
  const chatList = $('chatList');
  const stageHint = $('stageHint');

  const projectOverviewText = $('projectOverviewText');
  const currentStageText = $('currentStageText');
  const stagePlanText = $('stagePlanText');
  const stageCodeText = $('stageCodeText');
  const stageBuildText = $('stageBuildText');
  const stageDeliverText = $('stageDeliverText');
  const projectHistoryText = $('projectHistoryText');
  const projectContinueBtn = $('projectContinueBtn');
  const projectBuildBtn = $('projectBuildBtn');
  const projectRecordBtn = $('projectRecordBtn');
  const projectSettingsBtn = $('projectSettingsBtn');

  const userInfoText = $('userInfoText');
  const versionText = $('versionText');
  const aiSettingsRow = $('aiSettingsRow');
  const checkUpdateRow = $('checkUpdateRow');
  const updateArrow = $('updateArrow');
  const logoutRow = $('logoutRow');

  const inputBar = $('inputBar');
  const messageInput = $('messageInput');
  const sendBtn = $('sendBtn');
  const modelBtn = $('modelBtn');

  const tabs = document.querySelectorAll('.tab');
  const pages = document.querySelectorAll('.page');

  const newProjectMask = $('newProjectMask');
  const newProjectForm = $('newProjectForm');
  const projectNameInput = $('projectNameInput');
  const projectDescInput = $('projectDescInput');
  const templateSelect = $('templateSelect');
  const newProjectError = $('newProjectError');
  const createProjectBtn = $('createProjectBtn');
  const cancelNewProjectBtn = $('cancelNewProjectBtn');

  const agentMask = $('agentMask');
  const agentSelect = $('agentSelect');
  const cancelAgentBtn = $('cancelAgentBtn');

  // ====== 状态 ======
  const TOKEN_KEY = 'lodex_token';
  let token = localStorage.getItem(TOKEN_KEY) || localStorage.getItem('elon_token') || '';
  let authMode = 'login';
  let currentUser = null;
  let projects = [];
  let currentProject = null;
  let currentTab = 'chatPage';
  let socket = null;
  let busy = false;
  let selectedAgent = '';

  // ====== 工具 ======
  function api(path, options = {}) {
    const headers = Object.assign({}, options.headers || {});
    if (token) headers.Authorization = 'Bearer ' + token;
    if (options.body && !headers['Content-Type']) headers['Content-Type'] = 'application/json';
    return fetch(path, Object.assign({}, options, { headers }));
  }
  function escapeHtml(v) {
    return String(v == null ? '' : v)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
  }
  function withToken(href) {
    try {
      const url = new URL(href, location.origin);
      if (url.pathname.indexOf('/api/projects/') !== -1 && !url.searchParams.has('token')) {
        url.searchParams.set('token', token);
      }
      return url.toString();
    } catch { return href; }
  }
  function setBusy(value) {
    busy = !!value;
    sendBtn.disabled = busy || !currentProject;
  }
  function formatTime(iso) {
    if (!iso) return '';
    try {
      const d = new Date(iso);
      if (isNaN(d.getTime())) return '';
      const now = new Date();
      if (d.toDateString() === now.toDateString()) {
        return d.getHours().toString().padStart(2, '0') + ':' + d.getMinutes().toString().padStart(2, '0');
      }
      return (d.getMonth() + 1) + '/' + d.getDate();
    } catch { return ''; }
  }

  // ====== 登录/注册 ======
  function setAuthMode(mode) {
    authMode = mode === 'register' ? 'register' : 'login';
    const reg = authMode === 'register';
    authModeButtons.forEach((b) => b.classList.toggle('active', b.dataset.authMode === authMode));
    nicknameField.classList.toggle('hidden', !reg);
    passwordInput.autocomplete = reg ? 'new-password' : 'current-password';
    loginBtn.textContent = reg ? '创建账号' : '登录';
    loginError.textContent = '';
  }
  authModeButtons.forEach((b) => b.addEventListener('click', () => setAuthMode(b.dataset.authMode)));

  loginForm.addEventListener('submit', async (e) => {
    e.preventDefault();
    loginBtn.disabled = true;
    loginError.textContent = '';
    try {
      const payload = {
        account: accountInput.value.trim(),
        password: passwordInput.value,
        device_name: 'web'
      };
      if (authMode === 'register') payload.nickname = nicknameInput.value.trim();
      const res = await fetch(authMode === 'register' ? '/api/auth/register' : '/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data.error || (authMode === 'register' ? '注册失败' : '登录失败'));
      token = data.token;
      localStorage.setItem(TOKEN_KEY, token);
      localStorage.removeItem('elon_token');
      currentUser = data.user;
      await boot();
    } catch (err) {
      loginError.textContent = err.message;
    } finally {
      loginBtn.disabled = false;
    }
  });

  function showLogin() {
    if (socket) { try { socket.close(); } catch {} socket = null; }
    appView.classList.add('hidden');
    loginView.classList.remove('hidden');
  }
  function showApp() {
    loginView.classList.add('hidden');
    appView.classList.remove('hidden');
  }

  // ====== Tab 切换 ======
  function switchTab(tabId) {
    currentTab = tabId;
    tabs.forEach((t) => t.classList.toggle('active', t.dataset.tab === tabId));
    pages.forEach((p) => p.classList.toggle('active', p.id === tabId));
    // 顶栏标题
    const t = Array.prototype.find.call(tabs, (x) => x.dataset.tab === tabId);
    topTitle.textContent = t ? t.dataset.title : '';
    // 输入栏：仅"会话"页且已进入项目时显示
    const inChat = tabId === 'chatPage' && currentProject;
    inputBar.classList.toggle('active', !!inChat);
    // 返回按钮：仅"会话"页进入项目后显示
    backBtn.style.display = inChat ? '' : 'none';
    // 新建按钮：仅会话页显示
    addBtn.style.display = tabId === 'chatPage' ? '' : 'none';
    searchBtn.style.display = tabId === 'chatPage' ? '' : 'none';
  }
  tabs.forEach((t) => t.addEventListener('click', () => switchTab(t.dataset.tab)));

  backBtn.addEventListener('click', () => {
    // 退出当前项目，回到会话列表
    if (socket) { try { socket.close(); } catch {} socket = null; }
    currentProject = null;
    chatView.classList.add('hidden');
    conversationList.classList.remove('hidden');
    inputBar.classList.remove('active');
    backBtn.style.display = 'none';
    resetProjectPage();
    renderConversationList();
  });

  // ====== 会话/项目列表 ======
  function renderConversationList() {
    if (!projects.length) {
      conversationList.innerHTML =
        '<div class="empty-tip">还没有项目<br/>点击右上角 ＋ 新建你的第一个 App'
        + '<br/><button id="emptyCreateBtn">新建项目</button></div>';
      const b = $('emptyCreateBtn');
      if (b) b.addEventListener('click', openNewProject);
      return;
    }
    conversationList.innerHTML = '';
    projects.forEach((p) => {
      const item = document.createElement('button');
      item.type = 'button';
      item.className = 'conversation-item' + (currentProject && currentProject.id === p.id ? ' active' : '');
      const status = p.last_task_status || p.status || '准备就绪';
      item.innerHTML =
        '<span class="brand"><img src="data:image/png;base64,__BRAND_PNG_B64__" alt="" /></span>'
        + '<span class="text">'
        + '<span class="title"></span>'
        + '<span class="sub"></span>'
        + '</span>'
        + '<span class="time"></span>';
      item.querySelector('.title').textContent = p.name || '未命名项目';
      item.querySelector('.sub').textContent = status;
      item.querySelector('.time').textContent = formatTime(p.updated_at);
      item.addEventListener('click', () => selectProject(p));
      conversationList.appendChild(item);
    });
  }

  function selectProject(project) {
    currentProject = project;
    conversationList.classList.add('hidden');
    chatView.classList.remove('hidden');
    chatList.innerHTML = '';
    appendBubble('progress', '已进入项目「' + (project.name || '') + '」，可以开始描述需求。');
    inputBar.classList.add('active');
    backBtn.style.display = '';
    topTitle.textContent = project.name || '会话';
    renderProjectPage(project);
    connectSocket();
    setBusy(false);
  }

  // ====== 项目页渲染 ======
  function resetProjectPage() {
    currentStageText.textContent = '待提交需求';
    projectOverviewText.textContent = '项目管理\n当前没有正在执行的开发任务。';
    stagePlanText.textContent = '1. 需求分析：等待';
    stageCodeText.textContent = '2. 开发实现：等待';
    stageBuildText.textContent = '3. 编译打包：等待';
    stageDeliverText.textContent = '4. 交付下载：等待';
    projectHistoryText.textContent = '暂无进度记录';
  }
  function renderProjectPage(p) {
    if (!p) { resetProjectPage(); return; }
    currentStageText.textContent = p.last_task_status || p.status || '准备就绪';
    const overview = [
      '项目：' + (p.name || ''),
      p.description ? '描述：' + p.description : '',
      '模板：' + (p.template || ''),
      '角色：' + (p.role || ''),
      '更新：' + (p.updated_at || '')
    ].filter(Boolean).join('\n');
    projectOverviewText.textContent = overview;
  }

  // ====== 聊天气泡 ======
  function appendBubble(kind, text, links) {
    const row = document.createElement('div');
    row.className = 'bubble-row' + (kind === 'user' ? ' user' : '');
    const bubble = document.createElement('div');
    bubble.className = 'bubble ' + (kind === 'user' ? 'user' :
      kind === 'progress' ? 'progress' :
      kind === 'error' ? 'error' : 'ai');
    bubble.textContent = text || '';
    if (links && links.length) {
      links.forEach((lk) => {
        const a = document.createElement('a');
        a.href = withToken(lk.href);
        a.target = '_blank';
        a.rel = 'noreferrer';
        a.textContent = '\n' + lk.label;
        bubble.appendChild(a);
      });
    }
    row.appendChild(bubble);
    chatList.appendChild(row);
    chatList.scrollTop = chatList.scrollHeight;
  }

  // ====== WebSocket ======
  function connectSocket() {
    if (socket) { try { socket.close(); } catch {} socket = null; }
    if (!currentProject || !token) return;
    const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
    const url = scheme + '://' + location.host + '/ws/projects/' +
      encodeURIComponent(currentProject.id) + '?token=' + encodeURIComponent(token);
    try {
      socket = new WebSocket(url);
    } catch (e) {
      appendBubble('error', '连接失败：' + e.message);
      return;
    }
    socket.addEventListener('open', () => appendBubble('progress', '项目通道已连接。'));
    socket.addEventListener('close', () => setBusy(false));
    socket.addEventListener('error', () => appendBubble('error', '项目通道连接异常。'));
    socket.addEventListener('message', (ev) => {
      try {
        const data = JSON.parse(ev.data);
        if (data.type === 'progress') appendBubble('progress', data.message || '');
        else if (data.type === 'tool_call') appendBubble('progress', '工具：' + (data.tool || ''));
        else if (data.type === 'tool_result') appendBubble('progress', (data.tool || 'tool') + '\n' + (data.result || ''));
        else if (data.type === 'done') {
          setBusy(false);
          const links = [];
          if (data.apk_url) links.push({ href: data.apk_url, label: '下载 APK' });
          if (data.image_url) links.push({ href: data.image_url, label: '查看图片' });
          appendBubble('ai', data.message || '已完成。', links);
          loadProjects();
        } else if (data.type === 'error') {
          setBusy(false);
          appendBubble('error', data.message || 'unknown error');
          loadProjects();
        }
      } catch {
        appendBubble('error', '解析失败：' + ev.data);
      }
    });
  }

  inputBar.addEventListener('submit', (e) => {
    e.preventDefault();
    const text = messageInput.value.trim();
    if (!text || busy) return;
    if (!currentProject) { appendBubble('error', '请先选择或新建项目。'); return; }
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      appendBubble('error', '项目通道未连接，正在重连…');
      connectSocket();
      return;
    }
    const payload = { message: text };
    if (selectedAgent) payload.agent = selectedAgent;
    socket.send(JSON.stringify(payload));
    appendBubble('user', text);
    messageInput.value = '';
    messageInput.style.height = 'auto';
    setBusy(true);
  });
  messageInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      inputBar.requestSubmit();
    }
  });
  messageInput.addEventListener('input', () => {
    messageInput.style.height = 'auto';
    messageInput.style.height = Math.min(messageInput.scrollHeight, 120) + 'px';
  });

  // ====== 加载用户/项目 ======
  async function loadMe() {
    const res = await api('/api/me');
    if (res.status === 401 || res.status === 403) {
      const err = new Error('登录已过期');
      err.authFailed = true;
      throw err;
    }
    if (!res.ok) throw new Error('加载用户信息失败：HTTP ' + res.status);
    const data = await res.json();
    currentUser = data.user;
    const lines = [
      '用户：' + (currentUser.nickname || currentUser.id),
      '账号：' + (currentUser.account || currentUser.id),
      '用户 ID：' + currentUser.id
    ];
    userInfoText.textContent = lines.join('\n');
  }
  async function loadProjects() {
    const res = await api('/api/me/projects');
    if (!res.ok) throw new Error('项目加载失败');
    const data = await res.json();
    projects = data.projects || [];
    if (currentProject) {
      const refreshed = projects.find((p) => p.id === currentProject.id);
      if (refreshed) {
        currentProject = refreshed;
        renderProjectPage(refreshed);
      }
    }
    renderConversationList();
  }
  async function loadAgents() {
    if (!currentUser) return;
    try {
      const res = await fetch('/api/user/' + encodeURIComponent(currentUser.id) + '/agent');
      const data = await res.json();
      agentSelect.innerHTML = '<option value="">默认模型</option>';
      (data.available_agents || []).forEach((it) => {
        const opt = document.createElement('option');
        opt.value = it.name || '';
        opt.textContent = it.label || it.name || 'unknown';
        agentSelect.appendChild(opt);
      });
      agentSelect.value = selectedAgent;
    } catch {
      agentSelect.innerHTML = '<option value="">默认模型</option>';
    }
  }
  agentSelect.addEventListener('change', () => {
    selectedAgent = agentSelect.value;
    modelBtn.textContent = selectedAgent ? (agentSelect.selectedOptions[0].textContent || '已选') : '默认';
  });
  async function loadVersion() {
    try {
      const res = await fetch('/app/version.json');
      const data = await res.json();
      versionText.textContent = '一龙 · 当前版本 v' + (data.version_name || data.version || '?') +
        ' (build ' + (data.version_code || data.build || '?') + ')';
    } catch {
      versionText.textContent = '一龙网页版';
    }
  }

  // ====== 新建项目 ======
  function openNewProject() {
    newProjectError.textContent = '';
    projectNameInput.value = '';
    projectDescInput.value = '';
    newProjectMask.classList.add('active');
    setTimeout(() => projectNameInput.focus(), 50);
  }
  function closeNewProject() { newProjectMask.classList.remove('active'); }
  addBtn.addEventListener('click', openNewProject);
  cancelNewProjectBtn.addEventListener('click', closeNewProject);
  newProjectMask.addEventListener('click', (e) => { if (e.target === newProjectMask) closeNewProject(); });
  newProjectForm.addEventListener('submit', async (e) => {
    e.preventDefault();
    createProjectBtn.disabled = true;
    newProjectError.textContent = '';
    try {
      const res = await api('/api/projects', {
        method: 'POST',
        body: JSON.stringify({
          name: projectNameInput.value.trim(),
          description: projectDescInput.value.trim(),
          template: templateSelect.value
        })
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data.error || '创建失败');
      closeNewProject();
      await loadProjects();
      const created = projects.find((p) => p.id === data.project.id) || data.project;
      if (created) selectProject(created);
    } catch (err) {
      newProjectError.textContent = err.message;
    } finally {
      createProjectBtn.disabled = false;
    }
  });

  // ====== AI 代理设置弹窗 ======
  function openAgentDialog() {
    loadAgents();
    agentMask.classList.add('active');
  }
  aiSettingsRow.addEventListener('click', openAgentDialog);
  modelBtn.addEventListener('click', openAgentDialog);
  cancelAgentBtn.addEventListener('click', () => agentMask.classList.remove('active'));
  agentMask.addEventListener('click', (e) => { if (e.target === agentMask) agentMask.classList.remove('active'); });

  // ====== 检查更新 ======
  checkUpdateRow.addEventListener('click', async () => {
    updateArrow.textContent = '检查中…';
    try {
      const res = await fetch('/app/version.json');
      const data = await res.json();
      const v = data.version_name || data.version || '?';
      const b = data.version_code || data.build || '?';
      updateArrow.textContent = '最新 v' + v + ' (' + b + ')';
    } catch {
      updateArrow.textContent = '检查失败';
    }
  });

  // ====== 退出登录 ======
  logoutRow.addEventListener('click', () => {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem('elon_token');
    token = '';
    currentUser = null;
    currentProject = null;
    projects = [];
    if (socket) { try { socket.close(); } catch {} socket = null; }
    switchTab('chatPage');
    showLogin();
  });

  // ====== 其他按钮（占位） ======
  searchBtn.addEventListener('click', () => {
    appendBubble('progress', '搜索功能开发中。');
  });
  projectContinueBtn.addEventListener('click', () => {
    if (!currentProject) { switchTab('chatPage'); return; }
    switchTab('chatPage');
    messageInput.focus();
  });
  projectBuildBtn.addEventListener('click', () => {
    if (!currentProject || !socket || socket.readyState !== WebSocket.OPEN) {
      switchTab('chatPage');
      appendBubble('error', '请先进入项目并等待通道连接。');
      return;
    }
    socket.send(JSON.stringify({ message: '请执行打包，生成最新 APK 下载链接。' }));
    appendBubble('user', '请执行打包，生成最新 APK 下载链接。');
    switchTab('chatPage');
    setBusy(true);
  });
  projectRecordBtn.addEventListener('click', () => switchTab('chatPage'));
  projectSettingsBtn.addEventListener('click', openAgentDialog);

  // ====== 启动 ======
  async function boot() {
    if (!token) { showLogin(); return; }
    try {
      await loadMe();
    } catch (err) {
      if (err && err.authFailed) {
        localStorage.removeItem(TOKEN_KEY);
        localStorage.removeItem('elon_token');
        token = '';
      }
      loginError.textContent = (err && err.message) ? err.message : '加载失败，请重试';
      showLogin();
      return;
    }
    showApp();
    switchTab('chatPage');
    // 后续数据加载失败不踢回登录页，仅在控制台告警
    Promise.all([
      loadProjects().catch((e) => console.warn('loadProjects failed:', e)),
      loadVersion().catch((e) => console.warn('loadVersion failed:', e)),
    ]).then(() => loadAgents()).catch((e) => console.warn('loadAgents failed:', e));
  }
  setAuthMode('login');
  boot();
})();
</script>
</body>
</html>
"###;

const DOWNLOAD_HTML_TEMPLATE: &str = r###"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
  <meta name="theme-color" content="#101010" />
  <title>一龙 APK 下载</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #101010;
      --panel: #1f1f1f;
      --panel-2: #292929;
      --line: #373737;
      --ink: #f2f2f2;
      --ink-soft: #b8b8b8;
      --ink-muted: #8e8e8e;
      --brand: #07c160;
      --brand-hover: #06ad55;
    }
    * { box-sizing: border-box; }
    html, body {
      margin: 0;
      min-height: 100%;
      background: var(--bg);
      color: var(--ink);
      font-family: -apple-system, BlinkMacSystemFont, "PingFang SC", "Microsoft YaHei", sans-serif;
      font-size: 15px;
      line-height: 1.55;
    }
    body {
      display: flex;
      justify-content: center;
      padding: 28px 16px;
    }
    main {
      width: min(100%, 520px);
      display: flex;
      flex-direction: column;
      gap: 18px;
    }
    .brand {
      display: flex;
      align-items: center;
      gap: 12px;
      min-height: 64px;
    }
    .brand img {
      width: 52px;
      height: 52px;
      border-radius: 10px;
      display: block;
    }
    h1 {
      margin: 0;
      font-size: 24px;
      font-weight: 650;
      letter-spacing: 0;
    }
    .sub {
      margin-top: 2px;
      color: var(--ink-soft);
      font-size: 13px;
    }
    .section {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 18px;
    }
    .status {
      color: var(--ink-soft);
      min-height: 24px;
    }
    .fixed-url {
      margin-top: 12px;
      padding: 12px;
      background: var(--panel-2);
      border-radius: 6px;
      color: var(--ink);
      word-break: break-all;
      user-select: all;
    }
    .actions {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 10px;
      margin-top: 16px;
    }
    .actions a,
    .actions button {
      height: 44px;
      border: 0;
      border-radius: 6px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      text-align: center;
      color: #fff;
      background: var(--brand);
      text-decoration: none;
      cursor: pointer;
      font: inherit;
    }
    .actions button.secondary {
      background: var(--panel-2);
      color: var(--ink);
    }
    .actions a:hover,
    .actions button:hover { background: var(--brand-hover); }
    .actions button.secondary:hover { background: #343434; }
    textarea {
      width: 100%;
      min-height: 128px;
      resize: vertical;
      background: var(--panel-2);
      border: 1px solid var(--line);
      border-radius: 6px;
      color: var(--ink);
      padding: 12px;
      outline: none;
    }
    .tip {
      color: var(--ink-muted);
      font-size: 12px;
      text-align: center;
    }
    @media (max-width: 420px) {
      body { padding: 18px 12px; }
      h1 { font-size: 21px; }
      .actions { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <main>
    <header class="brand">
      <img src="data:image/png;base64,__BRAND_PNG_B64__" alt="一龙" />
      <div>
        <h1>一龙 APK 下载</h1>
        <div class="sub">每次升级后仍使用同一个固定 APK 下载地址</div>
      </div>
    </header>

    <section class="section">
      <div id="versionStatus" class="status">正在读取最新版本...</div>
      <div class="fixed-url" id="apkUrl">__APK_URL__</div>
      <div class="actions">
        <a id="downloadBtn" href="__APK_URL__">下载最新 APK</a>
        <button type="button" class="secondary" id="copyUrlBtn">复制下载地址</button>
      </div>
    </section>

    <section class="section">
      <textarea id="promoText" readonly>我正在用「一龙」云端 APK 开发平台，手机里直接提需求，云端帮你改代码、打包并生成安装包。

下载地址：__APK_URL__</textarea>
      <div class="actions">
        <button type="button" id="copyPromoBtn">复制推广语</button>
        <button type="button" class="secondary" id="shareBtn">系统分享</button>
      </div>
    </section>

    <div class="tip">下载页地址：__PAGE_URL__</div>
  </main>

  <script>
    (function () {
      const apkUrl = '__APK_URL__';
      const status = document.getElementById('versionStatus');
      const promoText = document.getElementById('promoText');

      async function copyText(text) {
        if (navigator.clipboard && window.isSecureContext) {
          await navigator.clipboard.writeText(text);
          return;
        }
        const el = document.createElement('textarea');
        el.value = text;
        el.style.position = 'fixed';
        el.style.left = '-9999px';
        document.body.appendChild(el);
        el.focus();
        el.select();
        document.execCommand('copy');
        document.body.removeChild(el);
      }

      function toast(text) {
        status.textContent = text;
        window.setTimeout(loadVersion, 1500);
      }

      async function loadVersion() {
        try {
          const res = await fetch('/app/version.json', { cache: 'no-store' });
          const data = await res.json();
          const version = data.versionName || data.version_name || '?';
          const code = data.versionCode || data.version_code || '?';
          const size = data.fileSize ? ' · ' + (data.fileSize / 1048576).toFixed(1) + ' MB' : '';
          status.textContent = '最新版本：v' + version + ' (build ' + code + ')' + size;
        } catch {
          status.textContent = '最新版本信息暂不可用，下载地址保持固定可用。';
        }
      }

      document.getElementById('copyUrlBtn').addEventListener('click', async () => {
        await copyText(apkUrl);
        toast('下载地址已复制');
      });

      document.getElementById('copyPromoBtn').addEventListener('click', async () => {
        await copyText(promoText.value);
        toast('推广语已复制');
      });

      document.getElementById('shareBtn').addEventListener('click', async () => {
        if (navigator.share) {
          await navigator.share({ title: '一龙 APK 下载', text: promoText.value, url: apkUrl });
        } else {
          await copyText(promoText.value);
          toast('当前浏览器不支持系统分享，已复制推广语');
        }
      });

      loadVersion();
    })();
  </script>
</body>
</html>
"###;
