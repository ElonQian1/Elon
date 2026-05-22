use axum::response::Html;

pub async fn web_page() -> Html<&'static str> {
    Html(WEB_HTML)
}

const WEB_HTML: &str = r###"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>一龙项目工作台</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f7f8f5;
      --panel: #ffffff;
      --ink: #1c241f;
      --muted: #68726c;
      --line: #d9ded7;
      --brand: #0a7066;
      --brand-strong: #07584f;
      --soft: #eef5f2;
      --warn: #8a5a0a;
      --warn-bg: #fff7e6;
      --danger: #a61b13;
      --danger-bg: #fff0ed;
      font-family: Inter, "PingFang SC", "Microsoft YaHei", system-ui, sans-serif;
    }
    * { box-sizing: border-box; }
    body { margin: 0; min-height: 100vh; background: var(--bg); color: var(--ink); }
    button, input, select, textarea { font: inherit; }
    button {
      height: 38px;
      border: 1px solid transparent;
      border-radius: 8px;
      padding: 0 13px;
      background: var(--brand);
      color: white;
      font-weight: 740;
      cursor: pointer;
    }
    button.secondary { background: white; color: var(--ink); border-color: var(--line); }
    button.ghost { background: transparent; color: var(--brand-strong); border-color: transparent; }
    button:disabled { opacity: .55; cursor: not-allowed; }
    input, select, textarea {
      width: 100%;
      border: 1px solid var(--line);
      border-radius: 8px;
      background: white;
      color: var(--ink);
      outline: none;
    }
    input, select { height: 40px; padding: 0 11px; }
    textarea { min-height: 92px; resize: none; padding: 12px; line-height: 1.5; }
    input:focus, select:focus, textarea:focus {
      border-color: var(--brand);
      box-shadow: 0 0 0 3px rgba(10,112,102,.12);
    }
    label { display: block; margin-bottom: 7px; color: var(--muted); font-size: 12px; font-weight: 760; }
    .login {
      min-height: 100vh;
      display: grid;
      place-items: center;
      padding: 24px;
    }
    .login-panel {
      width: min(420px, 100%);
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 26px;
      display: grid;
      gap: 16px;
      box-shadow: 0 14px 36px rgba(26, 35, 30, .08);
    }
    .brand { display: flex; align-items: center; gap: 10px; font-size: 20px; font-weight: 800; }
    .mark {
      width: 34px;
      height: 34px;
      border-radius: 8px;
      display: grid;
      place-items: center;
      background: linear-gradient(135deg, var(--brand), #d09031);
      color: white;
      font-weight: 900;
    }
    .muted { color: var(--muted); font-size: 13px; line-height: 1.5; }
    .error-line { min-height: 18px; color: var(--danger); font-size: 13px; }
    .shell { min-height: 100vh; display: grid; grid-template-columns: 310px minmax(0, 1fr); }
    aside {
      border-right: 1px solid var(--line);
      background: #fbfcfa;
      padding: 20px;
      display: grid;
      grid-template-rows: auto auto minmax(0, 1fr) auto;
      gap: 16px;
    }
    .user-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
    .user-name { min-width: 0; }
    .user-name strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .user-name span { display: block; color: var(--muted); font-size: 12px; margin-top: 3px; }
    .project-tools { display: grid; gap: 10px; }
    .project-list { overflow: auto; display: grid; align-content: start; gap: 9px; }
    .project-card {
      width: 100%;
      text-align: left;
      border: 1px solid var(--line);
      background: white;
      color: var(--ink);
      height: auto;
      min-height: 86px;
      padding: 12px;
      display: grid;
      gap: 6px;
    }
    .project-card.active { border-color: var(--brand); background: var(--soft); }
    .project-title { font-weight: 800; overflow-wrap: anywhere; }
    .project-meta { color: var(--muted); font-size: 12px; display: flex; gap: 8px; flex-wrap: wrap; }
    main { min-height: 100vh; display: grid; grid-template-rows: auto minmax(0, 1fr) auto; }
    header {
      min-height: 74px;
      padding: 17px 24px;
      border-bottom: 1px solid var(--line);
      background: rgba(255,255,255,.86);
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
    }
    h1 { margin: 0; font-size: 19px; letter-spacing: 0; }
    .header-sub { margin-top: 4px; color: var(--muted); font-size: 13px; }
    .header-actions { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
    .messages {
      overflow: auto;
      padding: 22px 24px;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .empty {
      margin: auto;
      max-width: 480px;
      text-align: center;
      color: var(--muted);
      line-height: 1.6;
    }
    .message {
      width: min(820px, 100%);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 13px 14px;
      background: var(--panel);
      line-height: 1.55;
      white-space: pre-wrap;
      overflow-wrap: anywhere;
    }
    .message.user { align-self: flex-end; background: #e9f4f1; border-color: #c7ded8; }
    .message.progress { background: var(--warn-bg); border-color: #efd7a6; color: var(--warn); }
    .message.error { background: var(--danger-bg); border-color: #f0b6ae; color: var(--danger); }
    .meta { display: block; margin-bottom: 5px; color: var(--muted); font-size: 12px; font-weight: 800; }
    .composer {
      border-top: 1px solid var(--line);
      background: white;
      padding: 15px 24px 18px;
      display: grid;
      gap: 10px;
    }
    .composer-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 10px; align-items: end; }
    a { color: var(--brand-strong); font-weight: 800; }
    dialog {
      width: min(470px, calc(100vw - 28px));
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 0;
      box-shadow: 0 22px 70px rgba(25, 35, 30, .22);
    }
    dialog::backdrop { background: rgba(20,28,24,.42); }
    .modal-body { padding: 22px; display: grid; gap: 14px; }
    .modal-title { font-weight: 820; font-size: 18px; }
    .modal-actions { display: flex; justify-content: flex-end; gap: 10px; padding-top: 6px; }
    .hidden { display: none !important; }
    @media (max-width: 860px) {
      .shell { grid-template-columns: 1fr; }
      aside { border-right: 0; border-bottom: 1px solid var(--line); max-height: 44vh; }
      main { min-height: 56vh; }
      header, .messages, .composer { padding-left: 16px; padding-right: 16px; }
      .composer-row { grid-template-columns: 1fr; }
      .composer-row button { width: 100%; }
    }
  </style>
</head>
<body>
  <section id="loginView" class="login">
    <form id="loginForm" class="login-panel">
      <div class="brand"><div class="mark">龙</div><div>一龙项目工作台</div></div>
      <div class="muted">使用管理员创建的账号登录。</div>
      <div><label for="accountInput">账号</label><input id="accountInput" autocomplete="username" /></div>
      <div><label for="passwordInput">密码</label><input id="passwordInput" type="password" autocomplete="current-password" /></div>
      <div id="loginError" class="error-line"></div>
      <button id="loginBtn" type="submit">登录</button>
    </form>
  </section>

  <section id="appView" class="shell hidden">
    <aside>
      <div class="brand"><div class="mark">龙</div><div>项目</div></div>
      <div class="user-row">
        <div class="user-name"><strong id="userName">未登录</strong><span id="userAccount"></span></div>
        <button id="logoutBtn" class="ghost" type="button">退出</button>
      </div>
      <div class="project-tools">
        <button id="newProjectBtn" type="button">新建项目</button>
        <button id="refreshProjectsBtn" class="secondary" type="button">刷新项目</button>
      </div>
      <div id="projectList" class="project-list"></div>
      <div class="muted" id="runtimeText">连接服务中</div>
    </aside>
    <main>
      <header>
        <div>
          <h1 id="projectTitle">选择项目</h1>
          <div id="projectSub" class="header-sub">登录后可找回已有项目，也可以新建项目</div>
        </div>
        <div class="header-actions">
          <select id="agentSelect"><option value="">默认模型</option></select>
          <button id="reconnectBtn" class="secondary" type="button">重连</button>
        </div>
      </header>
      <section id="messages" class="messages" aria-live="polite">
        <div class="empty">请选择左侧项目，或新建一个项目。</div>
      </section>
      <form id="composer" class="composer">
        <div class="composer-row">
          <textarea id="messageInput" placeholder="描述要开发或修改的 APK"></textarea>
          <button id="sendBtn" type="submit">发送</button>
        </div>
      </form>
    </main>
  </section>

  <dialog id="newProjectDialog">
    <form id="newProjectForm" class="modal-body" method="dialog">
      <div class="modal-title">新建项目</div>
      <div><label for="projectNameInput">项目名称</label><input id="projectNameInput" /></div>
      <div><label for="projectDescInput">项目描述</label><textarea id="projectDescInput"></textarea></div>
      <div><label for="templateSelect">模板</label><select id="templateSelect"><option value="android">Android APK</option></select></div>
      <div id="newProjectError" class="error-line"></div>
      <div class="modal-actions">
        <button id="cancelNewProjectBtn" class="secondary" type="button">取消</button>
        <button id="createProjectBtn" type="submit">创建</button>
      </div>
    </form>
  </dialog>

  <script>
    const loginView = document.getElementById("loginView");
    const appView = document.getElementById("appView");
    const loginForm = document.getElementById("loginForm");
    const accountInput = document.getElementById("accountInput");
    const passwordInput = document.getElementById("passwordInput");
    const loginBtn = document.getElementById("loginBtn");
    const loginError = document.getElementById("loginError");
    const userName = document.getElementById("userName");
    const userAccount = document.getElementById("userAccount");
    const logoutBtn = document.getElementById("logoutBtn");
    const projectList = document.getElementById("projectList");
    const newProjectBtn = document.getElementById("newProjectBtn");
    const refreshProjectsBtn = document.getElementById("refreshProjectsBtn");
    const newProjectDialog = document.getElementById("newProjectDialog");
    const newProjectForm = document.getElementById("newProjectForm");
    const cancelNewProjectBtn = document.getElementById("cancelNewProjectBtn");
    const createProjectBtn = document.getElementById("createProjectBtn");
    const projectNameInput = document.getElementById("projectNameInput");
    const projectDescInput = document.getElementById("projectDescInput");
    const templateSelect = document.getElementById("templateSelect");
    const newProjectError = document.getElementById("newProjectError");
    const projectTitle = document.getElementById("projectTitle");
    const projectSub = document.getElementById("projectSub");
    const runtimeText = document.getElementById("runtimeText");
    const messages = document.getElementById("messages");
    const agentSelect = document.getElementById("agentSelect");
    const reconnectBtn = document.getElementById("reconnectBtn");
    const composer = document.getElementById("composer");
    const messageInput = document.getElementById("messageInput");
    const sendBtn = document.getElementById("sendBtn");

    let token = localStorage.getItem("elon_token") || "";
    let currentUser = null;
    let projects = [];
    let currentProject = null;
    let socket = null;
    let busy = false;

    function showLogin() {
      loginView.classList.remove("hidden");
      appView.classList.add("hidden");
      if (socket) socket.close();
    }
    function showApp() {
      loginView.classList.add("hidden");
      appView.classList.remove("hidden");
    }
    function api(path, options = {}) {
      const headers = { ...(options.headers || {}) };
      if (token) headers.Authorization = `Bearer ${token}`;
      if (options.body && !headers["Content-Type"]) headers["Content-Type"] = "application/json";
      return fetch(path, { ...options, headers });
    }
    function setBusy(value) {
      busy = value;
      sendBtn.disabled = value || !currentProject;
    }
    function append(role, title, text, links = []) {
      const node = document.createElement("div");
      node.className = `message ${role || ""}`.trim();
      const meta = document.createElement("span");
      meta.className = "meta";
      meta.textContent = title;
      node.appendChild(meta);
      node.appendChild(document.createTextNode(text || ""));
      for (const link of links) {
        const line = document.createElement("div");
        const a = document.createElement("a");
        a.href = withToken(link.href);
        a.target = "_blank";
        a.rel = "noreferrer";
        a.textContent = link.label;
        line.appendChild(a);
        node.appendChild(line);
      }
      messages.appendChild(node);
      messages.scrollTop = messages.scrollHeight;
    }
    function withToken(href) {
      try {
        const url = new URL(href, location.origin);
        if (url.pathname.includes("/api/projects/") && !url.searchParams.has("token")) {
          url.searchParams.set("token", token);
        }
        return url.toString();
      } catch {
        return href;
      }
    }
    function renderProjects() {
      if (!projects.length) {
        projectList.innerHTML = '<div class="muted">暂无项目</div>';
        return;
      }
      projectList.innerHTML = "";
      for (const project of projects) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = `project-card ${currentProject && currentProject.id === project.id ? "active" : ""}`;
        btn.innerHTML = `
          <span class="project-title"></span>
          <span class="project-meta">
            <span>${escapeHtml(project.role || "")}</span>
            <span>${escapeHtml(project.template || "")}</span>
            <span>${escapeHtml(project.last_task_status || project.status || "")}</span>
          </span>
        `;
        btn.querySelector(".project-title").textContent = project.name;
        btn.addEventListener("click", () => selectProject(project));
        projectList.appendChild(btn);
      }
    }
    function selectProject(project) {
      currentProject = project;
      projectTitle.textContent = project.name;
      projectSub.textContent = `${project.role} · ${project.status} · ${project.updated_at || ""}`;
      messages.innerHTML = "";
      append("progress", "项目", "已进入项目工作区。");
      renderProjects();
      connectProjectSocket();
      setBusy(false);
    }
    async function loadMe() {
      const res = await api("/api/me");
      if (!res.ok) throw new Error("登录已过期");
      const data = await res.json();
      currentUser = data.user;
      userName.textContent = currentUser.nickname || currentUser.id;
      userAccount.textContent = currentUser.account || currentUser.id;
    }
    async function loadProjects() {
      const res = await api("/api/me/projects");
      if (!res.ok) throw new Error("项目加载失败");
      const data = await res.json();
      projects = data.projects || [];
      if (currentProject) {
        currentProject = projects.find(p => p.id === currentProject.id) || null;
      }
      renderProjects();
      if (!currentProject && projects.length) selectProject(projects[0]);
      if (!projects.length) {
        messages.innerHTML = '<div class="empty">还没有项目。</div>';
        projectTitle.textContent = "项目工作台";
        projectSub.textContent = "新建项目后就可以开始对话开发";
      }
    }
    async function loadAgents() {
      if (!currentUser) return;
      try {
        const res = await fetch(`/api/user/${encodeURIComponent(currentUser.id)}/agent`);
        const data = await res.json();
        const selected = agentSelect.value;
        agentSelect.innerHTML = '<option value="">默认模型</option>';
        for (const item of data.available_agents || []) {
          const option = document.createElement("option");
          option.value = item.name || "";
          option.textContent = item.label || item.name || "unknown";
          agentSelect.appendChild(option);
        }
        agentSelect.value = [...agentSelect.options].some(opt => opt.value === selected) ? selected : "";
      } catch {
        agentSelect.innerHTML = '<option value="">默认模型</option>';
      }
    }
    async function refreshRuntime() {
      try {
        const res = await fetch("/readyz");
        const data = await res.json();
        runtimeText.textContent = `${data.backend} · ${data.cli_options.length} CLI · ${data.api_agents} API`;
      } catch {
        runtimeText.textContent = "服务未连接";
      }
    }
    async function boot() {
      if (!token) return showLogin();
      try {
        await loadMe();
        showApp();
        await Promise.all([loadProjects(), loadAgents(), refreshRuntime()]);
      } catch {
        localStorage.removeItem("elon_token");
        token = "";
        showLogin();
      }
    }
    function connectProjectSocket() {
      if (socket) socket.close();
      if (!currentProject || !token) return;
      const scheme = location.protocol === "https:" ? "wss" : "ws";
      socket = new WebSocket(`${scheme}://${location.host}/ws/projects/${encodeURIComponent(currentProject.id)}?token=${encodeURIComponent(token)}`);
      socket.addEventListener("open", () => append("progress", "连接", "项目通道已连接。"));
      socket.addEventListener("close", () => setBusy(false));
      socket.addEventListener("error", () => append("error", "连接", "项目通道连接失败。"));
      socket.addEventListener("message", event => {
        try {
          const data = JSON.parse(event.data);
          if (data.type === "progress") append("progress", "进度", data.message || "");
          else if (data.type === "tool_call") append("progress", "工具", data.tool || "");
          else if (data.type === "tool_result") append("progress", "结果", `${data.tool || "tool"}\n${data.result || ""}`);
          else if (data.type === "done") {
            setBusy(false);
            const links = [];
            if (data.apk_url) links.push({ href: data.apk_url, label: "下载 APK" });
            if (data.image_url) links.push({ href: data.image_url, label: "查看图片" });
            append("", "完成", data.message || "", links);
            loadProjects();
          } else if (data.type === "error") {
            setBusy(false);
            append("error", "错误", data.message || "unknown error");
            loadProjects();
          }
        } catch {
          append("error", "解析失败", event.data);
        }
      });
    }
    loginForm.addEventListener("submit", async event => {
      event.preventDefault();
      loginBtn.disabled = true;
      loginError.textContent = "";
      try {
        const res = await fetch("/api/auth/login", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            account: accountInput.value.trim(),
            password: passwordInput.value,
            device_name: "web"
          })
        });
        const data = await res.json();
        if (!res.ok) throw new Error(data.error || "登录失败");
        token = data.token;
        localStorage.setItem("elon_token", token);
        currentUser = data.user;
        await boot();
      } catch (err) {
        loginError.textContent = err.message;
      } finally {
        loginBtn.disabled = false;
      }
    });
    logoutBtn.addEventListener("click", () => {
      localStorage.removeItem("elon_token");
      token = "";
      currentUser = null;
      currentProject = null;
      projects = [];
      showLogin();
    });
    refreshProjectsBtn.addEventListener("click", () => loadProjects());
    newProjectBtn.addEventListener("click", () => {
      newProjectError.textContent = "";
      projectNameInput.value = "";
      projectDescInput.value = "";
      newProjectDialog.showModal();
      setTimeout(() => projectNameInput.focus(), 50);
    });
    cancelNewProjectBtn.addEventListener("click", () => newProjectDialog.close());
    newProjectForm.addEventListener("submit", async event => {
      event.preventDefault();
      createProjectBtn.disabled = true;
      newProjectError.textContent = "";
      try {
        const res = await api("/api/projects", {
          method: "POST",
          body: JSON.stringify({
            name: projectNameInput.value.trim(),
            description: projectDescInput.value.trim(),
            template: templateSelect.value
          })
        });
        const data = await res.json();
        if (!res.ok) throw new Error(data.error || "创建失败");
        newProjectDialog.close();
        await loadProjects();
        const project = projects.find(p => p.id === data.project.id) || data.project;
        selectProject(project);
      } catch (err) {
        newProjectError.textContent = err.message;
      } finally {
        createProjectBtn.disabled = false;
      }
    });
    reconnectBtn.addEventListener("click", () => {
      refreshRuntime();
      loadAgents();
      connectProjectSocket();
    });
    composer.addEventListener("submit", event => {
      event.preventDefault();
      const text = messageInput.value.trim();
      if (!text || busy) return;
      if (!currentProject) {
        append("error", "项目", "请先选择或新建项目。");
        return;
      }
      if (!socket || socket.readyState !== WebSocket.OPEN) {
        append("error", "连接", "项目通道未连接，正在重连。");
        connectProjectSocket();
        return;
      }
      const payload = { message: text };
      if (agentSelect.value) payload.agent = agentSelect.value;
      socket.send(JSON.stringify(payload));
      append("user", "你", text);
      messageInput.value = "";
      setBusy(true);
    });
    function escapeHtml(value) {
      return String(value)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
    }
    boot();
  </script>
</body>
</html>"###;
