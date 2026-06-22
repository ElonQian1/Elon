(function () {
  const DOWNLOAD_URL = '/api/node-agent/download/windows-client';
  const VERSION_URL = '/api/node-agent/version';
  const LAUNCH_URL = 'elon-node://open';
  const LOCAL_ADMIN_HEADER_FALLBACK = 'X-Elon-Local-Admin-Token';

  function createNodeController(deps) {
    const { state, els, $, clean, escapeHtml, renderMembers, setHeader, setComposer, setRails, setNodeMode } = deps;
    state.nodeProbeSeq = state.nodeProbeSeq || 0;
    state.nodePollTimer = state.nodePollTimer || null;
    state.nodeLocalOnline = false;
    state.activeNodeId = state.activeNodeId || '';
    const nativeAdmin = window.ElonPcNodeAdmin && window.ElonPcNodeAdmin.create({
      state, $, clean, escapeHtml,
      localNodeApi: adminNodeApi,
      ensureLocalNodeLogin: deps.ensureLocalNodeLogin,
      openSettings: deps.openSettings,
      loadBaseData: deps.loadBaseData,
      renderChannels: deps.renderChannels,
      probeLocalNode,
      formatBytes
    });

    function renderChannels(channelButton) {
      const onlineCount = state.nodes.filter((node) => node.online).length;
      els.channelList.innerHTML = `
        <div class="channel-section">本机</div>
        ${channelButton({ id: 'local-node', kind: 'node', glyph: 'PC', title: '节点注册', sub: '下载、启动和注册 Win 端', active: !state.activeNodeId })}
        <div class="channel-section">我的节点</div>
        ${state.nodes.map((node) => channelButton({
          id: node.node_id || node.agent_id || '',
          kind: 'node-list',
          glyph: node.online ? '●' : '○',
          title: clean(node.display_name || node.device_name || node.short_id || node.node_id || 'PC 节点'),
          sub: nodeSummaryLine(node),
          online: !!node.online,
          active: sameNode(node, state.activeNodeId)
        })).join('') || '<div class="empty-state">暂无节点</div>'}
        <div class="channel-section">状态</div>
        <div class="empty-state">${onlineCount}/${state.nodes.length} 台在线</div>`;
      els.channelList.querySelectorAll('[data-peer-kind="node"]').forEach((btn) => {
        btn.addEventListener('click', selectLocalNode);
      });
      els.channelList.querySelectorAll('[data-peer-kind="node-list"]').forEach((btn) => {
        btn.addEventListener('click', () => selectNodeDetail(btn.dataset.itemId));
      });
    }

    function selectNode() {
      if (state.activeNodeId && findNode(state.activeNodeId)) return selectNodeDetail(state.activeNodeId);
      return selectLocalNode();
    }

    function selectLocalNode() {
      state.activeKind = 'node';
      state.activeProjectId = '';
      state.activeChannelId = '';
      state.activeChannelKind = '';
      state.activePeer = null;
      state.activeNodeId = '';
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

    function selectNodeDetail(nodeId) {
      const selected = findNode(nodeId);
      if (!selected) return selectLocalNode();
      state.activeKind = 'node';
      state.activeProjectId = '';
      state.activeChannelId = '';
      state.activeChannelKind = '';
      state.activePeer = null;
      state.activeNodeId = selected.node_id || selected.agent_id || '';
      stopNodePolling();
      setRails('node');
      els.workspaceName.textContent = 'PC 节点';
      els.workspaceMeta.textContent = '我的节点';
      setHeader(selected.online ? '●' : '○', nodeName(selected), nodeSummaryLine(selected));
      setComposer(false, '节点详情只读', false);
      deps.renderChannels();
      renderNodeDetail(selected);
      renderMembers('节点摘要', [
        { name: '状态', sub: selected.online ? '在线' : '离线' },
        { name: '项目容量', sub: capacityText(selected) },
        { name: '硬盘服务', sub: selected.storage_ready ? '可用' : '未启用' },
        { name: '开发运行时', sub: selected.workspace_provision_ready ? '可创建工作区' : '未就绪' }
      ]);
    }

    function renderNodeMain() {
      setNodeMode(true);
      els.messageList.innerHTML = `
        <div class="node-page">
          <section class="node-setup-hero">
            <div>
              <div class="node-kicker">一龙 Win 端</div>
              <h2>连接本机 PC 节点</h2>
              <p>首次使用需要下载并确认安装；安装后点击“启动 Win 端”，浏览器会拉起本机程序，本页检测到 7799 服务后直接显示原生节点管理面板。</p>
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
        if (state.activeKind !== 'node' || state.activeNodeId || attempts > 24) {
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
        if (seq !== state.nodeProbeSeq || state.activeKind !== 'node' || state.activeNodeId) return;
        if (state.nodePollTimer) {
          window.clearInterval(state.nodePollTimer);
          state.nodePollTimer = null;
        }
        rememberLocalAdmin(status);
        renderNodeConnected(status);
      } catch (error) {
        if (seq !== state.nodeProbeSeq || state.activeKind !== 'node' || state.activeNodeId) return;
        renderNodeSetup(error && error.name === 'AbortError'
          ? '本机节点暂时没有响应。'
          : '没有检测到正在运行的本机节点。');
      }
    }

    async function localNodeApi(path, optionsOrTimeout, timeoutMs) {
      const request = typeof optionsOrTimeout === 'object' && optionsOrTimeout !== null
        ? Object.assign({}, optionsOrTimeout)
        : {};
      const timeout = typeof optionsOrTimeout === 'number' ? optionsOrTimeout : (timeoutMs || 8000);
      const needsAdmin = localNodeNeedsAdmin(path);
      if (needsAdmin && !state.localAdminToken) await refreshLocalAdminToken(timeout);
      applyLocalAdminHeaders(request, needsAdmin);
      const controller = new AbortController();
      const timer = window.setTimeout(() => controller.abort(), timeout);
      try {
        let resp = await fetch(localNodeApiUrl(path), Object.assign({ cache: 'no-store' }, request, {
          signal: controller.signal
        }));
        if (needsAdmin && resp.status === 403) {
          state.localAdminToken = '';
          await refreshLocalAdminToken(timeout);
          applyLocalAdminHeaders(request, needsAdmin);
          resp = await fetch(localNodeApiUrl(path), Object.assign({ cache: 'no-store' }, request, {
            signal: controller.signal
          }));
        }
        const text = await resp.text();
        const data = text ? JSON.parse(text) : {};
        rememberLocalAdmin(data);
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

    function localNodeNeedsAdmin(path) {
      return String(path || '').replace(/^\/+/, '') !== 'api/status';
    }

    function applyLocalAdminHeaders(request, needsAdmin) {
      const headers = Object.assign({}, request.headers || {});
      if (request.body && !Object.keys(headers).some((key) => key.toLowerCase() === 'content-type')) {
        headers['Content-Type'] = 'application/json';
      }
      if (needsAdmin && state.localAdminToken) {
        headers[state.localAdminTokenHeader || LOCAL_ADMIN_HEADER_FALLBACK] = state.localAdminToken;
      }
      request.headers = headers;
    }

    function rememberLocalAdmin(data) {
      const token = clean(data && data.local_admin_token);
      const header = clean(data && data.local_admin_token_header);
      if (token) state.localAdminToken = token;
      if (header) state.localAdminTokenHeader = header;
    }

    async function refreshLocalAdminToken(timeout) {
      const controller = new AbortController();
      const timer = window.setTimeout(() => controller.abort(), timeout || 8000);
      try {
        const resp = await fetch(localNodeApiUrl('/api/status'), {
          cache: 'no-store',
          signal: controller.signal
        });
        const text = await resp.text();
        const data = text ? JSON.parse(text) : {};
        if (!resp.ok) throw new Error(data.error || data.message || `HTTP ${resp.status}`);
        rememberLocalAdmin(data);
        return data;
      } finally {
        window.clearTimeout(timer);
      }
    }

    async function adminNodeApi(path, options) {
      if (typeof deps.localNodeApi === 'function') return deps.localNodeApi(path, options || {});
      return localNodeApi(path, options || {}, 8000);
    }

    function renderNodeConnected(status) {
      state.nodeLocalOnline = true;
      setStatus('已连接', 'online');
      setLaunchButton('高级本机页');
      const surface = $('nodeLocalSurface');
      const line = $('nodeVersionLine');
      if (line) {
        const name = clean(status.device_name) || '本机节点';
        const logged = status.logged_in ? '已登录' : '未登录';
        line.textContent = `${name} · ${logged} · ${status.connected ? '云端在线' : '等待连接云端'}`;
      }
      if (surface) {
        if (nativeAdmin) nativeAdmin.render(surface, status);
        else renderNodeConnectedFallback(surface, status);
      }
    }

    function renderNodeConnectedFallback(surface, status) {
      surface.innerHTML = `
        <div class="node-setup-card">
          <h3>${escapeHtml(clean(status.device_name) || '本机节点已连接')}</h3>
          <p>本机节点服务已启动，但当前页面缺少原生管理模块。请刷新页面，或打开高级本机页继续操作。</p>
          <div class="node-admin-actions">
            <button class="node-action primary" type="button" id="nodeFallbackAdvanced">打开高级本机页</button>
          </div>
        </div>`;
      $('nodeFallbackAdvanced')?.addEventListener('click', () => window.open(state.nodeAdminUrl, '_blank', 'noopener'));
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
          <p class="node-safe-note">浏览器不能静默安装或强行启动本地程序；已安装后可以通过 elon-node://open 唤起，启动后本页会自动检测并显示节点管理面板。</p>
        </div>`;
    }

    function renderNodeDetail(node) {
      setNodeMode(true);
      const hardware = node.hardware || {};
      const storage = node.storage || {};
      const runtime = node.dev_runtime || {};
      const warnings = (node.capacity_warnings || []).concat(runtime.issues || []).map(clean).filter(Boolean);
      const toolchains = (runtime.toolchains || []).filter(Boolean);
      const models = (node.models || []).filter(Boolean);
      const remainingSlots = Number(node.project_slots_remaining);
      els.messageList.innerHTML = `
        <div class="node-page node-detail-page">
          <section class="node-detail-hero">
            <div>
              <div class="node-kicker">我的节点</div>
              <h2>${escapeHtml(nodeName(node))}</h2>
              <p>${escapeHtml(node.node_id || node.agent_id || '')}</p>
            </div>
            <span class="node-status-chip ${node.online ? 'online' : 'offline'}">${node.online ? '在线' : '离线'}</span>
          </section>
          <section class="node-detail-metrics">
            ${metricCard('连接', node.online ? '在线' : '离线', connectionText(node))}
            ${metricCard('容量', capacityText(node), node.can_accept_project ? '可接新项目' : '暂不可接新项目')}
            ${metricCard('硬盘', node.storage_ready ? '可用' : '未启用', storageStatusText(node))}
            ${metricCard('运行时', runtimeStatusText(node), cliStatusText(node))}
          </section>
          ${warnings.length ? `<section class="node-warning-list">${warnings.map((item) => `<div>${escapeHtml(item)}</div>`).join('')}</section>` : ''}
          <section class="node-detail-grid">
            ${detailPanel('基础信息', [
              ['显示名称', nodeName(node)],
              ['设备名', clean(node.device_name) || '未上报'],
              ['节点 ID', clean(node.node_id || node.agent_id) || '未知'],
              ['短 ID', clean(node.short_id) || '未知'],
              ['注册时间', formatDateTime(node.created_at) || '未知'],
              ['最近连接', formatUnixTime(node.connected_at) || '未连接']
            ])}
            ${detailPanel('硬件画像', [
              ['系统', clean(hardware.os) || '未上报'],
              ['架构', clean(hardware.arch) || '未上报'],
              ['CPU', hardware.cpu_brand ? `${hardware.cpu_brand}${hardware.cpu_cores ? ` · ${hardware.cpu_cores} 核` : ''}` : '未上报'],
              ['内存', formatBytes(hardware.memory_total_bytes) || '未上报'],
              ['显卡', (hardware.gpu_names || []).join('、') || '未上报'],
              ['显存', formatBytes(hardware.gpu_memory_total_bytes) || '未上报']
            ])}
            ${detailPanel('项目与硬盘', [
              ['项目数量', capacityText(node)],
              ['剩余名额', Number.isFinite(remainingSlots) ? `${remainingSlots}` : '未知'],
              ['可用磁盘', formatBytes(node.disk_free_bytes || storage.disk_free_bytes) || '未上报'],
              ['工作目录', clean(storage.root_path) || '未配置'],
              ['Git 地址', clean(storage.git_base_url) || (storage.relay_git_url_enabled ? '云端中继可用' : '未配置')],
              ['跨 PC 仓库', node.storage_repo_url_configured ? '已配置' : '未配置']
            ])}
            ${detailPanel('开发运行时', [
              ['工作区根目录', clean(runtime.workspace_root_path) || '未配置'],
              ['目录可写', runtime.workspace_root_writable ? '是' : '否'],
              ['Git', runtime.git_ready ? '可用' : '未就绪'],
              ['创建工作区', node.workspace_provision_ready ? '可用' : '未就绪'],
              ['开发环境', runtime.dev_env_ready ? '就绪' : '未就绪'],
              ['AI Agent', node.ai_cli_ready ? '就绪' : '未就绪'],
              ['Route A 本机 CLI', routeReady(node, runtime, 'route_a_ready') ? '可用' : '未就绪'],
              ['Route B 本机 API runtime', routeReady(node, runtime, 'api_runtime_ready') ? '可用' : '未就绪'],
              ['Route C 服务器模型', routeReady(node, runtime, 'server_runtime_ready') ? '可用' : '未就绪'],
              ['Route C 保护', routeCProtectionText(runtime)]
            ])}
          </section>
          <section class="node-detail-grid compact">
            ${listPanel('模型能力', models.length ? models.map(modelLine) : ['暂无模型能力'])}
            ${listPanel('允许的 CLI', (node.allowed_clis || []).length ? node.allowed_clis : ['未连接本机 CLI'])}
            ${listPanel('允许目录', (node.allowed_cwds || []).length ? node.allowed_cwds : ['未配置目录白名单'])}
            ${listPanel('工具链', toolchains.length ? toolchains.map(toolchainLine) : ['未上报工具链'])}
          </section>
          <section class="node-actions-panel">
            <button class="node-action" type="button" id="backToLocalNode">回到节点注册</button>
          </section>
        </div>`;
      $('backToLocalNode')?.addEventListener('click', selectLocalNode);
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

    function stopNodePolling() {
      if (!state.nodePollTimer) return;
      window.clearInterval(state.nodePollTimer);
      state.nodePollTimer = null;
    }

    function findNode(nodeId) {
      return state.nodes.find((node) => sameNode(node, nodeId));
    }

    function sameNode(node, nodeId) {
      const id = clean(nodeId);
      return !!id && [node.node_id, node.agent_id].some((value) => String(value || '') === id);
    }

    function nodeName(node) {
      return clean(node.display_name || node.device_name || node.label || node.short_id || node.node_id || 'PC 节点');
    }

    function nodeSummaryLine(node) {
      const status = node.online ? '在线' : '离线';
      const capacity = clean(node.capacity_label);
      return capacity ? `${status} · ${capacity}` : status;
    }

    function connectionText(node) {
      if (!node.online) return '节点未连接云端';
      if (node.registry_online && node.cli_connected) return '节点与 CLI 均在线';
      if (node.registry_online) return '节点在线';
      if (node.cli_connected) return 'CLI 在线';
      return '在线';
    }

    function capacityText(node) {
      const count = Number(node.project_count || 0);
      const limit = Number(node.project_limit || 0);
      if (limit > 0) return `${count}/${limit} 个项目`;
      return `${count} 个项目`;
    }

    function storageStatusText(node) {
      if (node.storage_ready) return '可保存项目代码';
      return node.storage && node.storage.enabled ? '等待 CLI 连接' : '未启用硬盘节点';
    }

    function runtimeStatusText(node) {
      if (node.workspace_provision_ready) return '可创建工作区';
      if (node.cli_project_ready) return 'CLI 可用';
      return '未就绪';
    }

    function cliStatusText(node) {
      const clis = (node.allowed_clis || []).map(clean).filter(Boolean);
      if (clis.length) return clis.join(' / ');
      const runtime = node.dev_runtime || {};
      if (routeReady(node, runtime, 'server_runtime_ready')) return 'Route C 服务器模型';
      if (routeReady(node, runtime, 'api_runtime_ready')) return 'Route B API runtime';
      return '未连接本机 CLI';
    }

    function routeReady(node, runtime, key) {
      return !!(node[key] || (runtime && runtime[key]));
    }

    function routeCProtectionText(runtime) {
      const status = (runtime && (runtime.server_runtime_status || runtime.serverRuntimeStatus)) || null;
      if (!status) return '未上报';
      const stateText = clean(status.status);
      const policy = status.policy || {};
      if (policy.enabled === false || stateText === 'disabled') return '已关闭 · 运维开关保护';
      if (stateText === 'missing_token') return '未登录 · 不会调用服务器模型';
      if (stateText === 'http_error') return `云端返回 ${clean(status.httpStatus) || '错误'} · 不会启用`;
      if (stateText === 'unavailable') return `${clean(status.reason) || '云端不可用'} · 不会启用`;
      const limits = status.limits || {};
      const admission = status.admission || {};
      const rpm = numberField(limits, 'maxRequestsPerMinute', 'max_requests_per_minute')
        || numberField(admission, 'maxRequestsPerMinute', 'max_requests_per_minute');
      const perUser = numberField(limits, 'maxConcurrentPerUser', 'max_concurrent_per_user')
        || numberField(admission, 'maxConcurrentPerUser', 'max_concurrent_per_user');
      const global = numberField(limits, 'maxConcurrentGlobal', 'max_concurrent_global')
        || numberField(admission, 'maxConcurrentGlobal', 'max_concurrent_global');
      const remaining = numberField(admission, 'remainingRequestsPerMinute', 'remaining_requests_per_minute');
      const parts = [];
      if (rpm) parts.push(`${rpm}/分钟`);
      if (perUser || global) parts.push(`并发 ${perUser || '?'} / ${global || '?'}`);
      if (Number.isFinite(remaining)) parts.push(`剩余 ${remaining}`);
      return parts.length ? `已保护 · ${parts.join(' · ')}` : '已保护 · 限额策略已上报';
    }

    function numberField(object, camelName, snakeName) {
      if (!object || (!Object.prototype.hasOwnProperty.call(object, camelName) && !Object.prototype.hasOwnProperty.call(object, snakeName))) return null;
      const value = object[camelName] ?? object[snakeName];
      const number = Number(value);
      return Number.isFinite(number) ? number : null;
    }

    function detailPanel(title, rows) {
      return `<div class="node-info-panel"><h3>${escapeHtml(title)}</h3><div class="node-kv-list">
        ${rows.map(([label, value]) => `<div><span>${escapeHtml(label)}</span><strong>${escapeHtml(clean(value) || '未上报')}</strong></div>`).join('')}
      </div></div>`;
    }

    function listPanel(title, items) {
      return `<div class="node-info-panel"><h3>${escapeHtml(title)}</h3><div class="node-pill-row">
        ${items.map((item) => `<span class="node-pill">${escapeHtml(clean(item) || '未知')}</span>`).join('')}
      </div></div>`;
    }

    function metricCard(label, value, sub) {
      return `<div class="node-metric"><span>${escapeHtml(label)}</span><strong>${escapeHtml(value || '未知')}</strong><em>${escapeHtml(sub || '')}</em></div>`;
    }

    function modelLine(model) {
      const name = clean(model.display_name || model.model_id || '模型');
      const provider = clean(model.provider);
      const context = Number(model.context_len || 0);
      return `${name}${provider ? ` · ${provider}` : ''}${context ? ` · ${context} ctx` : ''}`;
    }

    function toolchainLine(toolchain) {
      const name = clean(toolchain.name || '工具链');
      const version = clean(toolchain.version);
      return `${name}${toolchain.available ? ' 可用' : ' 不可用'}${version ? ` · ${version}` : ''}`;
    }

    function formatUnixTime(value) {
      const timestamp = Number(value || 0);
      if (!timestamp) return '';
      return formatDateTime(new Date(timestamp * 1000).toISOString());
    }

    function formatDateTime(value) {
      if (!value) return '';
      const date = new Date(value);
      if (Number.isNaN(date.getTime())) return clean(value);
      return date.toLocaleString('zh-CN', { hour12: false });
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
