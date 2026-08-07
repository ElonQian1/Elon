(function () {
  'use strict';

  const row = document.getElementById('aiProviderAccountsRow');
  const mask = document.getElementById('aiProviderAccountsMask');
  if (!row || !mask) return;

  const nodeSelect = document.getElementById('aiProviderNodeSelect');
  const status = document.getElementById('aiProviderAccountsStatus');
  const list = document.getElementById('aiProviderAccountsList');
  const refresh = document.getElementById('aiProviderAccountsRefresh');
  const close = document.getElementById('aiProviderAccountsClose');
  const diagnostics = document.getElementById('aiProviderAccountsDiagnostics');
  const localWebList = document.getElementById('localWebProviderList');
  let nodes = [];
  let pollTimer = 0;

  function renderLocalWebProviders() {
    if (!localWebList) return;
    localWebList.innerHTML = '';
    const registry = window.ElonLocalWebProviders;
    const providers = registry && typeof registry.list === 'function' ? registry.list() : [];
    providers.forEach((provider) => {
      const card = document.createElement('section');
      card.className = 'ai-provider-card ai-provider-local-web-card';
      const title = document.createElement('h3');
      title.textContent = provider.label;
      const state = document.createElement('div');
      state.className = 'ai-provider-card-state';
      state.textContent = '登录状态由 ChatGPT 官方页面确认';
      const detail = document.createElement('p');
      detail.className = 'ai-provider-card-detail';
      detail.textContent = provider.detail;
      const steps = document.createElement('ol');
      steps.className = 'ai-provider-login-steps';
      ['打开官方 ChatGPT', '本人完成登录或真人验证', '在官方页面开始聊天'].forEach((label) => {
        const step = document.createElement('li');
        step.textContent = label;
        steps.appendChild(step);
      });
      const capabilities = document.createElement('div');
      capabilities.className = 'ai-provider-capabilities';
      capabilities.append(
        capabilityBadge('本机浏览器会话', provider.capabilities.localBrowserSession),
        capabilityBadge('PWA 原生投影', provider.capabilities.nativeProjectionInPwa),
        capabilityBadge('APK 原生投影', provider.capabilities.nativeProjectionInApk)
      );
      const actions = document.createElement('div');
      actions.className = 'ai-provider-card-actions';
      const open = document.createElement('button');
      open.type = 'button';
      open.textContent = '登录或继续使用 ChatGPT';
      open.addEventListener('click', () => {
        window.open(provider.officialUrl, '_blank', 'noopener,noreferrer');
      });
      const apk = document.createElement('a');
      apk.href = '/app/download';
      apk.textContent = '获取 APK 增强模式';
      actions.append(open, apk);
      card.append(title, state, detail, steps, capabilities, actions);
      localWebList.appendChild(card);
    });
  }

  function capabilityBadge(label, available) {
    const badge = document.createElement('span');
    badge.className = 'ai-provider-capability ' + (available ? 'available' : 'unavailable');
    badge.textContent = label + ' · ' + (available ? '可用' : '受限');
    return badge;
  }

  function authToken() {
    return String(
      window.__ELON_UI_TUNER_PREVIEW_AUTH__?.token
      || localStorage.getItem('lodex_token')
      || localStorage.getItem('elon_token')
      || ''
    );
  }

  async function requestJson(url, options) {
    const config = Object.assign({}, options || {});
    const headers = Object.assign({}, config.headers || {});
    const token = authToken();
    if (token) headers.Authorization = 'Bearer ' + token;
    if (config.body) headers['Content-Type'] = 'application/json';
    config.headers = headers;
    const response = await fetch(url, config);
    const text = await response.text();
    let value = {};
    try { value = text ? JSON.parse(text) : {}; } catch { value = {}; }
    if (!response.ok) throw new Error(value.error || value.message || text || ('HTTP ' + response.status));
    return value;
  }

  function relayUrl(nodeId, providerId, tail) {
    const parts = ['/api/pc-relay', nodeId, 'api', 'ai-provider-accounts'];
    if (providerId) parts.push(providerId);
    if (tail) parts.push.apply(parts, tail);
    return parts.map((part, index) => index === 0 ? part : encodeURIComponent(part)).join('/');
  }

  function selectedNode() {
    return nodes.find((node) => node.id === nodeSelect.value) || null;
  }

  async function loadNodes() {
    clearTimeout(pollTimer);
    status.textContent = '正在读取我的 Win 节点…';
    list.innerHTML = '';
    const result = await requestJson('/api/me/nodes', { cache: 'no-store' });
    nodes = (result.nodes || []).map((node) => ({
      id: String(node.node_id || node.agent_id || ''),
      label: String(node.display_name || node.device_name || node.label || node.short_id || 'Win 节点'),
      online: !!node.online
    })).filter((node) => node.id).sort((a, b) => Number(b.online) - Number(a.online));
    nodeSelect.innerHTML = '';
    nodes.forEach((node) => {
      const option = document.createElement('option');
      option.value = node.id;
      option.textContent = node.label + ' · ' + (node.online ? '在线' : '离线');
      nodeSelect.appendChild(option);
    });
    const online = nodes.find((node) => node.online);
    if (!online) {
      status.textContent = nodes.length ? '没有在线 Win 节点。' : '尚未绑定 Win 节点。';
      return;
    }
    nodeSelect.value = online.id;
    await loadAccounts();
  }

  async function loadAccounts() {
    clearTimeout(pollTimer);
    const node = selectedNode();
    if (!node || !node.online) {
      list.innerHTML = '';
      status.textContent = '请选择在线 Win 节点。';
      return;
    }
    status.textContent = '正在读取 ' + node.label + ' 的账号状态…';
    const result = await requestJson(relayUrl(node.id), { cache: 'no-store' });
    renderProviders(result.providers || []);
    status.textContent = '凭据只保存在 ' + node.label + ' 的官方 CLI 中，不会上传到一龙云端。';
    const active = (result.providers || []).find((provider) => isActive(provider.active_login));
    if (active) schedulePoll(node, active);
  }

  function isActive(attempt) {
    return !!attempt && (attempt.state === 'starting' || attempt.state === 'waiting_for_user');
  }

  function providerState(provider) {
    if (provider.implementation_state === 'reserved') return '接口已保留';
    if (isActive(provider.active_login)) return '等待用户完成登录';
    if (provider.active_login?.state === 'failed') return '上次登录失败';
    if (provider.active_login?.state === 'canceled') return '登录已取消，可重新发起';
    if (provider.active_login?.state === 'expired') return '登录已过期，可重新发起';
    if (provider.cli?.logged_in === true) return '已登录';
    if (!provider.cli?.runnable) return 'CLI 未安装或不可运行';
    return '未登录';
  }

  function providerDetail(provider) {
    const attempt = provider.active_login;
    if (provider.implementation_state === 'reserved') return provider.reason || '';
    if (isActive(attempt) && provider.id === 'codex_cli') {
      return '请在 OpenAI 官方页面完成验证。' + (attempt.user_code ? '\n设备码：' + attempt.user_code : '');
    }
    if (isActive(attempt) && provider.id === 'gemini_cli') {
      return 'Google 官方登录已在所选 Win 节点启动，请回到该电脑完成浏览器授权。';
    }
    if (attempt?.error) return attempt.error + (['failed', 'canceled', 'expired'].includes(attempt.state) ? '\n可以安全地重新发起登录。' : '');
    if (provider.cli?.logged_in === true && provider.credential_vault?.backup_supported) {
      return '凭据由官方 CLI 保存；只有明确同意后才会进入现有 Codex 加密保险箱。';
    }
    if (provider.cli?.logged_in === true) return '凭据由官方 CLI 保存在 Win 节点。';
    return provider.cli?.detail || provider.cli?.reason || '通过官方协议绑定所选 Win 节点。';
  }

  function primaryLabel(provider) {
    if (provider.implementation_state === 'reserved') return '等待官方接口';
    if (isActive(provider.active_login)) return '取消登录';
    if (provider.cli?.logged_in === true && provider.logout_supported === false) return '请在 CLI 退出';
    if (provider.cli?.logged_in === true) return '退出登录';
    if (!provider.cli?.runnable) return 'CLI 不可用';
    return '登录';
  }

  function renderProviders(providers) {
    list.innerHTML = '';
    providers.forEach((provider) => {
      const card = document.createElement('section');
      card.className = 'ai-provider-card';
      const title = document.createElement('h3');
      title.textContent = provider.label || provider.id;
      const state = document.createElement('div');
      state.className = 'ai-provider-card-state';
      state.textContent = providerState(provider);
      const detail = document.createElement('p');
      detail.className = 'ai-provider-card-detail';
      detail.textContent = providerDetail(provider);
      const actions = document.createElement('div');
      actions.className = 'ai-provider-card-actions';

      const attempt = provider.active_login;
      const officialUrl = attempt && (attempt.verification_url || attempt.auth_url);
      if (isActive(attempt) && (attempt.user_code || isSafeLoginUrl(officialUrl))) {
        const official = document.createElement('button');
        official.type = 'button';
        official.textContent = attempt.user_code ? '复制验证码并打开' : '打开官方登录';
        official.addEventListener('click', () => openOfficialLogin(attempt));
        actions.appendChild(official);
      }

      const primary = document.createElement('button');
      primary.type = 'button';
      primary.textContent = primaryLabel(provider);
      primary.disabled = provider.implementation_state !== 'available'
        || (!provider.cli?.runnable && !isActive(provider.active_login))
        || (provider.cli?.logged_in === true && provider.logout_supported === false);
      primary.addEventListener('click', () => runProviderAction(provider, primary));
      actions.appendChild(primary);
      card.append(title, state, detail, actions);
      list.appendChild(card);
    });
  }

  function isSafeLoginUrl(url) {
    try { return new URL(url).protocol === 'https:'; } catch { return false; }
  }

  async function openOfficialLogin(attempt) {
    if (attempt.user_code && navigator.clipboard) {
      await navigator.clipboard.writeText(attempt.user_code).catch(() => {});
    }
    const url = attempt.verification_url || attempt.auth_url;
    if (isSafeLoginUrl(url)) window.open(url, '_blank', 'noopener,noreferrer');
  }

  async function runProviderAction(provider, button) {
    const node = selectedNode();
    if (!node?.online) return;
    button.disabled = true;
    status.textContent = '正在处理 ' + provider.label + '…';
    try {
      const attempt = provider.active_login;
      if (isActive(attempt)) {
        await requestJson(relayUrl(node.id, provider.id, ['logins', attempt.login_id, 'cancel']), {
          method: 'POST', body: '{}'
        });
      } else if (provider.cli?.logged_in === true) {
        await requestJson(relayUrl(node.id, provider.id, ['logout']), { method: 'POST', body: '{}' });
      } else {
        const flow = provider.id === 'codex_cli' ? 'device_code' : 'agent';
        const result = await requestJson(relayUrl(node.id, provider.id, ['login']), {
          method: 'POST', body: JSON.stringify({
            flow,
            request_id: 'mobile-web:' + (crypto.randomUUID ? crypto.randomUUID() : Date.now())
          })
        });
        if (provider.id === 'codex_cli') await openOfficialLogin(result.attempt || {});
        if (provider.id === 'gemini_cli') {
          status.textContent = '请回到 ' + node.label + ' 完成 Google 官方登录。';
        }
      }
      await loadAccounts();
    } catch (error) {
      status.textContent = '操作失败：' + String(error.message || error).slice(0, 300);
      button.disabled = false;
    }
  }

  function schedulePoll(node, provider) {
    const attempt = provider.active_login;
    if (!attempt) return;
    pollTimer = window.setTimeout(async () => {
      try {
        const result = await requestJson(
          relayUrl(node.id, provider.id, ['logins', attempt.login_id]),
          { cache: 'no-store' }
        );
        if (isActive(result.attempt)) schedulePoll(node, Object.assign({}, provider, { active_login: result.attempt }));
        else await loadAccounts();
      } catch (error) {
        status.textContent = '登录状态刷新失败：' + String(error.message || error).slice(0, 240);
      }
    }, 2000);
  }

  async function openPanel() {
    mask.classList.add('active');
    renderLocalWebProviders();
    try { await loadNodes(); }
    catch (error) { status.textContent = '加载失败：' + String(error.message || error).slice(0, 300); }
  }

  async function loadDiagnostics() {
    const node = selectedNode();
    if (!node?.online) return;
    status.textContent = '正在读取脱敏恢复诊断…';
    try {
      const result = await requestJson(relayUrl(node.id, null, ['diagnostics']), { cache: 'no-store' });
      const retryable = (result.latest_attempts || []).filter((attempt) => attempt.retryable);
      status.textContent = '脱敏日志保留 ' + (result.journal?.retention_hours || 24)
        + ' 小时；可重试任务 ' + retryable.length
        + ' 个。验证码、授权地址和厂商 token 均不会显示。';
    } catch (error) {
      status.textContent = '诊断失败：' + String(error.message || error).slice(0, 240);
    }
  }

  function closePanel() {
    clearTimeout(pollTimer);
    mask.classList.remove('active');
  }

  row.addEventListener('click', openPanel);
  refresh.addEventListener('click', () => loadNodes().catch((error) => {
    status.textContent = '刷新失败：' + String(error.message || error).slice(0, 300);
  }));
  diagnostics?.addEventListener('click', loadDiagnostics);
  nodeSelect.addEventListener('change', () => loadAccounts().catch((error) => {
    status.textContent = '加载失败：' + String(error.message || error).slice(0, 300);
  }));
  close.addEventListener('click', closePanel);
  mask.addEventListener('click', (event) => { if (event.target === mask) closePanel(); });
})();
