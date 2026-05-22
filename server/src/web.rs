use axum::response::Html;

pub async fn web_page() -> Html<&'static str> {
    Html(WEB_HTML)
}

const WEB_HTML: &str = r###"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>一龙 Web 工作台</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f6f7f4;
      --panel: #ffffff;
      --ink: #1d2520;
      --muted: #68726d;
      --line: #d9ded8;
      --brand: #0b6f66;
      --brand-strong: #07594f;
      --soft: #eef4f2;
      --danger: #b42318;
      font-family: Inter, "PingFang SC", "Microsoft YaHei", system-ui, sans-serif;
    }
    * { box-sizing: border-box; }
    body { margin: 0; min-height: 100vh; background: var(--bg); color: var(--ink); }
    .shell { display: grid; grid-template-columns: 280px minmax(0, 1fr); min-height: 100vh; }
    aside {
      border-right: 1px solid var(--line);
      background: #fbfcfa;
      padding: 22px;
      display: flex;
      flex-direction: column;
      gap: 18px;
    }
    main { display: grid; grid-template-rows: auto minmax(0, 1fr) auto; min-height: 100vh; }
    .brand { display: flex; align-items: center; gap: 10px; font-weight: 760; font-size: 20px; letter-spacing: 0; }
    .mark {
      width: 34px;
      height: 34px;
      border-radius: 8px;
      background: linear-gradient(135deg, var(--brand), #d38f32);
      display: grid;
      place-items: center;
      color: white;
      font-weight: 800;
    }
    .field { display: grid; gap: 7px; }
    label { color: var(--muted); font-size: 12px; font-weight: 680; }
    input, select, textarea {
      width: 100%;
      border: 1px solid var(--line);
      border-radius: 8px;
      background: white;
      color: var(--ink);
      font: inherit;
      outline: none;
    }
    input, select { height: 40px; padding: 0 11px; }
    textarea { resize: none; min-height: 84px; padding: 12px; line-height: 1.5; }
    input:focus, select:focus, textarea:focus {
      border-color: var(--brand);
      box-shadow: 0 0 0 3px rgba(11, 111, 102, 0.12);
    }
    .status-list { display: grid; gap: 8px; padding-top: 4px; }
    .status-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      color: var(--muted);
      font-size: 13px;
    }
    .pill {
      border-radius: 999px;
      padding: 4px 9px;
      background: var(--soft);
      color: var(--brand-strong);
      font-size: 12px;
      font-weight: 700;
      white-space: nowrap;
    }
    .pill.off { background: #f8e8e5; color: var(--danger); }
    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      padding: 18px 26px;
      border-bottom: 1px solid var(--line);
      background: rgba(255, 255, 255, 0.82);
      backdrop-filter: blur(10px);
    }
    h1 { margin: 0; font-size: 19px; letter-spacing: 0; }
    .header-sub { margin-top: 3px; color: var(--muted); font-size: 13px; }
    .actions { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }
    button {
      height: 40px;
      border: 1px solid transparent;
      border-radius: 8px;
      padding: 0 14px;
      background: var(--brand);
      color: white;
      font: inherit;
      font-weight: 760;
      cursor: pointer;
    }
    button.secondary { background: white; color: var(--ink); border-color: var(--line); }
    button:disabled { cursor: not-allowed; opacity: 0.52; }
    .messages {
      overflow: auto;
      padding: 24px 26px;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .message {
      width: min(820px, 100%);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 14px 15px;
      line-height: 1.56;
      background: var(--panel);
      white-space: pre-wrap;
      word-break: break-word;
      overflow-wrap: anywhere;
    }
    .message.user { align-self: flex-end; background: #e8f3f0; border-color: #c7ded8; }
    .message.error { background: #fff0ed; border-color: #f1b8af; color: #842015; }
    .message.progress { background: #fff8eb; border-color: #efd6a6; color: #64420a; }
    .meta { display: block; color: var(--muted); font-size: 12px; font-weight: 760; margin-bottom: 5px; }
    .composer {
      border-top: 1px solid var(--line);
      background: white;
      padding: 16px 26px 20px;
      display: grid;
      gap: 10px;
    }
    .composer-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 10px; align-items: end; }
    a { color: var(--brand-strong); font-weight: 760; }
    @media (max-width: 820px) {
      .shell { grid-template-columns: 1fr; }
      aside { border-right: 0; border-bottom: 1px solid var(--line); }
      main { min-height: 68vh; }
      header, .messages, .composer { padding-left: 16px; padding-right: 16px; }
      .composer-row { grid-template-columns: 1fr; }
      button { width: 100%; }
    }
  </style>
</head>
<body>
  <div class="shell">
    <aside>
      <div class="brand"><div class="mark">龙</div><div>一龙 Web</div></div>
      <div class="field"><label for="userId">用户 ID</label><input id="userId" autocomplete="off" /></div>
      <div class="field"><label for="projectId">项目 ID</label><input id="projectId" autocomplete="off" /></div>
      <div class="field"><label for="agentSelect">模型</label><select id="agentSelect"><option value="">默认</option></select></div>
      <div class="status-list">
        <div class="status-row"><span>HTTP</span><span id="httpStatus" class="pill off">checking</span></div>
        <div class="status-row"><span>WebSocket</span><span id="wsStatus" class="pill off">closed</span></div>
        <div class="status-row"><span>CLI</span><span id="cliStatus" class="pill off">unknown</span></div>
      </div>
    </aside>
    <main>
      <header>
        <div><h1>Web 工作台</h1><div id="runtimeText" class="header-sub">连接服务中</div></div>
        <div class="actions">
          <button id="reconnectBtn" class="secondary" type="button">重连</button>
          <button id="clearBtn" class="secondary" type="button">清空</button>
        </div>
      </header>
      <section id="messages" class="messages" aria-live="polite"></section>
      <form id="composer" class="composer">
        <div class="composer-row">
          <textarea id="messageInput" placeholder="描述你要开发或修改的 App，也可以直接和服务对话"></textarea>
          <button id="sendBtn" type="submit">发送</button>
        </div>
      </form>
    </main>
  </div>
  <script>
    const userIdInput = document.getElementById("userId");
    const projectIdInput = document.getElementById("projectId");
    const agentSelect = document.getElementById("agentSelect");
    const httpStatus = document.getElementById("httpStatus");
    const wsStatus = document.getElementById("wsStatus");
    const cliStatus = document.getElementById("cliStatus");
    const runtimeText = document.getElementById("runtimeText");
    const messages = document.getElementById("messages");
    const messageInput = document.getElementById("messageInput");
    const sendBtn = document.getElementById("sendBtn");
    const reconnectBtn = document.getElementById("reconnectBtn");
    const clearBtn = document.getElementById("clearBtn");
    const composer = document.getElementById("composer");
    let socket = null;
    let busy = false;

    function id(prefix) { return `${prefix}_${Math.random().toString(36).slice(2, 10)}`; }
    userIdInput.value = localStorage.getItem("elon_web_user_id") || id("web");
    projectIdInput.value = localStorage.getItem("elon_web_project_id") || id("project");
    userIdInput.addEventListener("change", persistIdentity);
    projectIdInput.addEventListener("change", persistIdentity);

    function persistIdentity() {
      localStorage.setItem("elon_web_user_id", userIdInput.value.trim() || id("web"));
      localStorage.setItem("elon_web_project_id", projectIdInput.value.trim() || id("project"));
      loadAgents();
    }
    function setPill(el, text, ok) {
      el.textContent = text;
      el.classList.toggle("off", !ok);
    }
    function append(role, title, text, links = []) {
      const node = document.createElement("div");
      node.className = `message ${role}`;
      const meta = document.createElement("span");
      meta.className = "meta";
      meta.textContent = title;
      node.appendChild(meta);
      node.appendChild(document.createTextNode(text || ""));
      for (const link of links) {
        const line = document.createElement("div");
        const a = document.createElement("a");
        a.href = link.href;
        a.target = "_blank";
        a.rel = "noreferrer";
        a.textContent = link.label;
        line.appendChild(a);
        node.appendChild(line);
      }
      messages.appendChild(node);
      messages.scrollTop = messages.scrollHeight;
    }
    async function refreshRuntime() {
      try {
        const res = await fetch("/readyz");
        const data = await res.json();
        setPill(httpStatus, data.status || "ok", res.ok);
        setPill(cliStatus, data.local_cli_enabled ? "enabled" : "off", data.local_cli_enabled);
        runtimeText.textContent = `${data.backend} · ${data.cli_options.length} CLI · ${data.api_agents} API`;
      } catch (err) {
        setPill(httpStatus, "failed", false);
        runtimeText.textContent = "服务未连接";
      }
    }
    async function loadAgents() {
      const userId = encodeURIComponent(userIdInput.value.trim() || "web");
      try {
        const res = await fetch(`/api/user/${userId}/agent`);
        const data = await res.json();
        const selected = agentSelect.value;
        agentSelect.innerHTML = '<option value="">默认</option>';
        for (const item of data.available_agents || []) {
          const option = document.createElement("option");
          option.value = item.name || "";
          option.textContent = item.label || item.name || "unknown";
          agentSelect.appendChild(option);
        }
        agentSelect.value = [...agentSelect.options].some(opt => opt.value === selected) ? selected : "";
      } catch (err) {
        agentSelect.innerHTML = '<option value="">默认</option>';
      }
    }
    function connect() {
      if (socket) socket.close();
      const scheme = location.protocol === "https:" ? "wss" : "ws";
      socket = new WebSocket(`${scheme}://${location.host}/ws`);
      setPill(wsStatus, "connecting", false);
      socket.addEventListener("open", () => setPill(wsStatus, "open", true));
      socket.addEventListener("close", () => {
        setPill(wsStatus, "closed", false);
        busy = false;
        sendBtn.disabled = false;
      });
      socket.addEventListener("error", () => setPill(wsStatus, "error", false));
      socket.addEventListener("message", event => {
        try {
          const data = JSON.parse(event.data);
          if (data.type === "progress") {
            append("progress", "进度", data.message || "");
          } else if (data.type === "tool_call") {
            append("progress", "工具", data.tool || "");
          } else if (data.type === "tool_result") {
            append("progress", "结果", `${data.tool || "tool"}\n${data.result || ""}`);
          } else if (data.type === "done") {
            busy = false;
            sendBtn.disabled = false;
            const links = [];
            if (data.apk_url) links.push({ href: data.apk_url, label: "下载 APK" });
            if (data.image_url) links.push({ href: data.image_url, label: "查看图片" });
            append("", "完成", data.message || "", links);
          } else if (data.type === "error") {
            busy = false;
            sendBtn.disabled = false;
            append("error", "错误", data.message || "unknown error");
          }
        } catch (err) {
          append("error", "解析失败", event.data);
        }
      });
    }
    composer.addEventListener("submit", event => {
      event.preventDefault();
      const text = messageInput.value.trim();
      if (!text || busy) return;
      if (!socket || socket.readyState !== WebSocket.OPEN) {
        append("error", "连接", "WebSocket 未连接");
        connect();
        return;
      }
      const payload = {
        user_id: userIdInput.value.trim() || "web",
        project_id: projectIdInput.value.trim() || "project",
        message: text
      };
      if (agentSelect.value) payload.agent = agentSelect.value;
      socket.send(JSON.stringify(payload));
      append("user", "你", text);
      messageInput.value = "";
      busy = true;
      sendBtn.disabled = true;
    });
    reconnectBtn.addEventListener("click", () => {
      refreshRuntime();
      loadAgents();
      connect();
    });
    clearBtn.addEventListener("click", () => { messages.innerHTML = ""; });
    refreshRuntime();
    loadAgents();
    connect();
  </script>
</body>
</html>"###;
