(function () {
  const DOWNLOAD_URL = '/api/node-agent/download/windows-client';
  const VERSION_URL = '/api/node-agent/version';
  const LAUNCH_URL = 'elon-node://open';

  function createNodeController(deps) {
    const { state, els, $, clean, escapeHtml, renderMembers, setHeader, setComposer, setRails, setNodeMode } = deps;
    state.nodeProbeSeq = state.nodeProbeSeq || 0;
    state.nodePollTimer = state.nodePollTimer || null;
    state.nodeLocalOnline = false;

    function renderChannels(channelButton) {
      const onlineCount = state.nodes.filter((node) => node.online).length;
      els.channelList.innerHTML = `
        <div class="channel-section">本机</div>
        ${channelButton({ id: 'local-node', kind: 'node', glyph: 'PC', title: '节点注册', sub: '下载、启动和注册 Win 端', active: true })}
        <div class="channel-section">我的节点</div>
        ${state.nodes.map((node) => channelButton({
          id: node.node_id || node.agent_id || '',
          kind: 'node-list',
          glyph: node.online ? '●' : '○',
          title: clean(node.display_name || node.device_name || node.short_id || node.node_id || 'PC 节点'),
          sub: node.online ? '在线' : '离线',
          online: !!node.online
        })).join('') || '<div class="empty-state">暂无节点</div>'}
        <div class="channel-section">状态</div>
        <div class="empty-state">${onlineCount}/${state.nodes.length} 台在线</div>`;
    }

    function selectNode() {
      state.activeKind = 'node';
      state.activeProjectId = '';
      state.activeChannelId = '';
      state.activePeer = null;
      setRails('node');
      els.workspaceName.textContent = 'PC 节点';
      els.workspaceMeta.textContent = '下载、启动、注册';
      setHeader('PC', '本机节点', '让这台电脑成为可接收任务的 Win 端节点');
      setComposer(false, '节点管理页中操作', false);
      deps.renderChannels();
      renderNodeMain();
      renderMembers('我的节点', state.nodes.map((node) => ({
        name: clean(node.display_name || node.device_name || node.short_id || node.node_id || 'PC 节点'),
        sub: node.online ? '在线' : '离线'
      })));
    }

    function renderNodeMain() {
      setNodeMode(true);
      els.messageList.innerHTML = `
        <div class="node-page">
          <section class="node-setup-hero">
            <div>
              <div class="node-kicker">一龙 Win 端</div>
              <h2>连接本机 PC 节点</h2>
              <p>首次使用需要下载并确认安装；安装后点击“启动 Win 端”，浏览器会拉起本机程序，本页检测到 7799 服务后自动嵌入管理页。</p>
            </div>
            <span class="node-status-chip checking" id="nodeLocalStatus">检测中</span>
          </section>
          <section class="node-actions-panel">
            <a class="node-action primary" id="downloadNodeClient" href="${DOWNLOAD_URL}" download>下载 Win 端</a>
            <button class="node-action" type="button" id="openNodeFrame">启动 Win 端</button>
            <button class="node-action" type="button" id="retryNodeProbe">重新检测</button>
          </section>
          <div class="node-version-line" id="nodeVersionLine">正在读取最新 Win 端版本...</div>
          <div class="node-local-surface" id="nodeLocalSurface">
            <div class="node-loading">正在检测 ${escapeHtml(state.nodeAdminUrl)}</div>
          </div>
        </div>`;
      bindNodeActions();
      loadNodePackageVersion();
      probeLocalNode();
    }

    function bindNodeActions() {
      $('openNodeFrame')?.addEventListener('click', openNodeWindow);
      $('retryNodeProbe')?.addEventListener('click', () => probeLocalNode());
      $('downloadNodeClient')?.addEventListener('click', () => {
        renderNodeSetup('下载后解压压缩包，双击「一龙PC节点.exe」。它会自动安装、注册一键唤起入口，并打开 PC 网页。');
        setStatus('下载已开始', 'checking');
        startInstallPolling();
      });
    }

    function openNodeWindow() {
      if (state.nodeLocalOnline) {
        window.open(state.nodeAdminUrl, '_blank', 'noopener');
        return;
      }
      renderNodeSetup('如果浏览器询问是否打开“一龙PC节点”，请选择允许；如果没有反应，说明这台电脑还没安装 Win 端，请先下载。');
      setStatus('等待启动', 'checking');
      setLaunchButton('等待启动...');
      launchInstalledClient();
      startInstallPolling();
      window.setTimeout(() => probeLocalNode(true), 900);
    }

    function launchInstalledClient() {
      try {
        if (document.body && document.createElement) {
          const frame = document.createElement('iframe');
          frame.style.display = 'none';
          frame.setAttribute('aria-hidden', 'true');
          frame.src = LAUNCH_URL;
          document.body.appendChild(frame);
          window.setTimeout(() => frame.remove(), 2000);
          return;
        }
      } catch (_) {
        // Fall through to window.open below.
      }
      window.open(LAUNCH_URL, '_blank', 'noopener');
    }

    function startInstallPolling() {
      if (state.nodePollTimer) window.clearInterval(state.nodePollTimer);
      let attempts = 0;
      state.nodePollTimer = window.setInterval(() => {
        attempts += 1;
        if (state.activeKind !== 'node' || attempts > 24) {
          window.clearInterval(state.nodePollTimer);
          state.nodePollTimer = null;
          return;
        }
        probeLocalNode(true);
      }, 3500);
    }

    async function probeLocalNode(quiet) {
      const seq = ++state.nodeProbeSeq;
      if (!quiet) {
        setStatus('检测中', 'checking');
        const surface = $('nodeLocalSurface');
        if (surface) surface.innerHTML = `<div class="node-loading">正在检测 ${escapeHtml(state.nodeAdminUrl)}</div>`;
      }
      try {
        const status = await localNodeApi('/api/status', 2200);
        if (seq !== state.nodeProbeSeq || state.activeKind !== 'node') return;
        if (state.nodePollTimer) {
          window.clearInterval(state.nodePollTimer);
          state.nodePollTimer = null;
        }
        renderNodeConnected(status);
      } catch (error) {
        if (seq !== state.nodeProbeSeq || state.activeKind !== 'node') return;
        renderNodeSetup(error && error.name === 'AbortError'
          ? '本机节点暂时没有响应。'
          : '没有检测到正在运行的本机节点。');
      }
    }

    async function localNodeApi(path, timeoutMs) {
      const controller = new AbortController();
      const timer = window.setTimeout(() => controller.abort(), timeoutMs);
      try {
        const resp = await fetch(localNodeApiUrl(path), { cache: 'no-store', signal: controller.signal });
        const text = await resp.text();
        const data = text ? JSON.parse(text) : {};
        if (!resp.ok) throw new Error(data.error || data.message || `HTTP ${resp.status}`);
        return data;
      } finally {
        window.clearTimeout(timer);
      }
    }

    function localNodeApiUrl(path) {
      const base = state.nodeAdminUrl.endsWith('/') ? state.nodeAdminUrl : `${state.nodeAdminUrl}/`;
      return new URL(String(path || '').replace(/^\//, ''), base).toString();
    }

    function renderNodeConnected(status) {
      state.nodeLocalOnline = true;
      setStatus('已连接', 'online');
      setLaunchButton('打开本机页面');
      const surface = $('nodeLocalSurface');
      const line = $('nodeVersionLine');
      if (line) {
        const name = clean(status.device_name) || '本机节点';
        const logged = status.logged_in ? '已登录' : '未登录';
        line.textContent = `${name} · ${logged} · ${status.connected ? '云端在线' : '等待连接云端'}`;
      }
      if (surface) {
        surface.innerHTML = `<iframe class="node-frame" src="${escapeHtml(state.nodeAdminUrl)}" title="一龙 PC 节点本地管理"></iframe>`;
      }
    }

    function renderNodeSetup(reason) {
      state.nodeLocalOnline = false;
      setStatus('未连接', 'offline');
      setLaunchButton('启动 Win 端');
      const surface = $('nodeLocalSurface');
      if (!surface) return;
      surface.innerHTML = `
        <div class="node-setup-card">
          <h3>还没有可用的本机节点</h3>
          <p>${escapeHtml(reason || '请先安装并启动一龙 Win 端。')}</p>
          <div class="node-step-list">
            <div><strong>1</strong><span>下载 Win 端压缩包并解压。</span></div>
            <div><strong>2</strong><span>双击「一龙PC节点.exe」，它会自动安装、开机自启，并注册网页一键唤起。</span></div>
            <div><strong>3</strong><span>安装后点击“启动 Win 端”，再在本机页面登录一龙账号并注册 PC 节点。</span></div>
          </div>
          <p class="node-safe-note">浏览器不能静默安装或强行启动本地程序；已安装后可以通过 elon-node://open 唤起，启动后本页会自动检测并嵌入管理页。</p>
        </div>`;
    }

    async function loadNodePackageVersion() {
      const line = $('nodeVersionLine');
      if (!line) return;
      try {
        const resp = await fetch(VERSION_URL, { cache: 'no-store' });
        if (!resp.ok) throw new Error('version unavailable');
        const data = await resp.json();
        const size = formatBytes(data.windowsClientFileSize || data.fileSize);
        line.textContent = `最新 Win 端：v${clean(data.version || 'latest')}${size ? ` · ${size}` : ''}`;
      } catch (_) {
        line.textContent = 'Win 端下载包暂时无法读取版本；仍可尝试下载最新客户端。';
      }
    }

    function setStatus(text, mode) {
      const el = $('nodeLocalStatus');
      if (!el) return;
      el.textContent = text;
      el.className = `node-status-chip ${mode || ''}`;
    }

    function setLaunchButton(text) {
      const button = $('openNodeFrame');
      if (button) button.textContent = text;
    }

    function formatBytes(value) {
      const bytes = Number(value || 0);
      if (!bytes) return '';
      if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
      if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
      return `${bytes} B`;
    }

    return { renderChannels, selectNode, openNodeWindow };
  }

  window.ElonPcNode = { create: createNodeController };
})();
