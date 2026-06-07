(function () {
  const PAGE_LIMIT = 50;
  const state = {
    loaded: false,
    loading: false,
    query: '',
    projects: [],
    joinedIds: new Set(),
    selectedId: '',
    busyId: '',
    status: '',
    error: false
  };

  function bridge() {
    return window.ElonWebApp || {};
  }

  function escapeHtml(value) {
    const app = bridge();
    if (typeof app.escapeHtml === 'function') return app.escapeHtml(value);
    return String(value == null ? '' : value)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function token() {
    const app = bridge();
    return typeof app.getToken === 'function' ? app.getToken() : '';
  }

  function callApi(path, options) {
    const app = bridge();
    if (typeof app.api === 'function') return app.api(path, options || {});
    const headers = Object.assign({}, (options && options.headers) || {});
    const t = token();
    if (t) headers.Authorization = 'Bearer ' + t;
    if (options && options.body && !headers['Content-Type']) headers['Content-Type'] = 'application/json';
    return fetch(path, Object.assign({}, options || {}, { headers }));
  }

  function currentProjectIds() {
    const app = bridge();
    if (typeof app.getProjects !== 'function') return new Set();
    return new Set((app.getProjects() || []).map((p) => p && p.id).filter(Boolean));
  }

  function localProjectById(id) {
    const app = bridge();
    if (typeof app.getProjects !== 'function') return null;
    return (app.getProjects() || []).find((p) => p && p.id === id) || null;
  }

  function setStatus(message, error) {
    state.status = message || '';
    state.error = !!error;
    const el = document.getElementById('projectPlazaStatus');
    if (el) {
      el.textContent = state.status;
      el.className = 'project-plaza-status' + (state.error ? ' error' : '');
    }
  }

  function hueFor(value) {
    const text = String(value || 'project');
    let hash = 0;
    for (let i = 0; i < text.length; i += 1) hash = (hash * 31 + text.charCodeAt(i)) % 360;
    return 28 + (hash % 210);
  }

  function formatTime(value) {
    if (!value) return '未知';
    const d = new Date(value);
    if (Number.isNaN(d.getTime())) return String(value).replace('T', ' ').slice(0, 16);
    const pad = (n) => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function joinModeLabel(mode) {
    switch (mode) {
      case 'approval': return '需审批';
      case 'readonly': return '只读体验';
      case 'invite': return '邀请制';
      default: return '开放加入';
    }
  }

  function roleActionLabel(project) {
    if (state.joinedIds.has(project.id)) return '进入项目';
    if (project.join_mode === 'approval') return '申请加入';
    if (project.join_mode === 'readonly') return '只读加入';
    if (project.join_mode === 'invite') return '邀请制';
    return '加入项目';
  }

  function looksLikeCodeName(value) {
    const text = String(value || '').trim();
    return /^[A-Za-z0-9._-]{3,24}$/.test(text) && /[A-Za-z]/.test(text);
  }

  function projectIdentity(project) {
    const name = String(project.name || '').trim() || '未命名项目';
    const description = String(project.description || '').trim();
    if (description && looksLikeCodeName(name) && Array.from(description).length <= 24) {
      return {
        title: description,
        subtitle: '项目代号：' + name
      };
    }
    return {
      title: name,
      subtitle: description
    };
  }

  function projectInitial(title) {
    const chars = Array.from(String(title || '').trim());
    return (chars[0] || 'P').toUpperCase();
  }

  function avatarUrl(project) {
    const ownerId = String(project.owner_id || '').trim();
    return ownerId ? '/api/users/' + encodeURIComponent(ownerId) + '/avatar' : '';
  }

  function ensureModal() {
    if (document.getElementById('projectPlazaMask')) return;
    document.body.insertAdjacentHTML('beforeend', `
      <div class="project-plaza-mask" id="projectPlazaMask" role="dialog" aria-modal="true" aria-labelledby="projectPlazaTitle">
        <div class="project-plaza-sheet">
          <div class="project-plaza-header">
            <div>
              <div class="project-plaza-title" id="projectPlazaTitle">项目广场</div>
              <div class="project-plaza-count" id="projectPlazaCount">公开项目</div>
            </div>
            <button class="project-plaza-close" type="button" data-plaza-action="close" title="关闭">×</button>
          </div>
          <div class="project-plaza-toolbar">
            <input class="project-plaza-search" id="projectPlazaSearch" placeholder="搜索项目名称或说明" autocomplete="off" />
            <button class="project-plaza-refresh" type="button" data-plaza-action="refresh" title="刷新" aria-label="刷新">↻</button>
          </div>
          <div class="project-plaza-body">
            <div class="project-plaza-list" id="projectPlazaList"></div>
            <div class="project-plaza-detail" id="projectPlazaDetail"></div>
          </div>
        </div>
      </div>
    `);
    const mask = document.getElementById('projectPlazaMask');
    const search = document.getElementById('projectPlazaSearch');
    mask.addEventListener('click', (event) => {
      if (event.target === mask) closePlaza();
    });
    search.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') loadProjects(search.value.trim());
    });
    let timer = null;
    search.addEventListener('input', () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => loadProjects(search.value.trim()), 320);
    });
    mask.addEventListener('click', handleAction);
  }

  function openPlaza() {
    ensureModal();
    const mask = document.getElementById('projectPlazaMask');
    const search = document.getElementById('projectPlazaSearch');
    mask.classList.add('active');
    if (!state.loaded) loadProjects(state.query);
    else render();
    window.setTimeout(() => search && search.focus(), 60);
  }

  function closePlaza() {
    const mask = document.getElementById('projectPlazaMask');
    if (mask) mask.classList.remove('active');
  }

  async function loadJoinedIds() {
    const ids = currentProjectIds();
    if (!token()) {
      state.joinedIds = ids;
      return;
    }
    try {
      const res = await callApi('/api/store/joined');
      if (!res.ok) throw new Error('http ' + res.status);
      const data = await res.json();
      (data.projects || []).forEach((p) => { if (p && p.id) ids.add(p.id); });
    } catch (e) {
      console.warn('load joined projects failed:', e);
    }
    state.joinedIds = ids;
  }

  async function loadProjects(query) {
    state.query = query || '';
    state.loading = true;
    state.loaded = true;
    state.status = '';
    state.error = false;
    render();
    try {
      const params = new URLSearchParams({ limit: String(PAGE_LIMIT), offset: '0' });
      if (state.query) params.set('q', state.query);
      const res = await fetch('/api/store/projects?' + params.toString(), { cache: 'no-store' });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data.error || '加载失败');
      state.projects = data.projects || [];
      await loadJoinedIds();
      if (!state.projects.some((p) => p.id === state.selectedId)) {
        state.selectedId = state.projects[0] ? state.projects[0].id : '';
      }
    } catch (e) {
      state.projects = [];
      setStatus(e.message || '加载失败', true);
    } finally {
      state.loading = false;
      render();
    }
  }

  function render() {
    const count = document.getElementById('projectPlazaCount');
    const list = document.getElementById('projectPlazaList');
    const detail = document.getElementById('projectPlazaDetail');
    if (!count || !list || !detail) return;
    count.textContent = state.loading
      ? '正在加载公开项目'
      : `公开项目 ${state.projects.length} 个`;
    list.innerHTML = renderList();
    detail.innerHTML = renderDetail();
    setStatus(state.status, state.error);
  }

  function renderList() {
    if (state.loading) {
      return '<div class="project-plaza-empty">加载中...</div>';
    }
    if (!state.projects.length) {
      return `<div class="project-plaza-empty">${state.query ? '没有匹配的公开项目' : '暂无公开项目'}</div>`;
    }
    return state.projects.map((project) => renderCard(project)).join('');
  }

  function renderCard(project) {
    const joined = state.joinedIds.has(project.id);
    const active = project.id === state.selectedId;
    const disabled = state.busyId === project.id || project.join_mode === 'invite';
    const action = joined ? 'open' : project.join_mode === 'approval' ? 'select-apply' : 'join';
    const identity = projectIdentity(project);
    const avatar = avatarUrl(project);
    return `
      <button class="project-plaza-card${active ? ' active' : ''}" type="button" data-plaza-action="select" data-id="${escapeHtml(project.id)}">
        <div class="project-plaza-accent" style="--plaza-hue:${hueFor(project.id)}"></div>
        <div class="project-plaza-card-main" style="--plaza-hue:${hueFor(project.id)}">
          <div class="project-plaza-card-title">
            <span class="project-plaza-avatar" aria-hidden="true">
              <span>${escapeHtml(projectInitial(identity.title))}</span>
              ${avatar ? `<img src="${escapeHtml(avatar)}" alt="" loading="lazy" onerror="this.remove()" />` : ''}
            </span>
            <span class="project-plaza-title-stack">
              <span class="project-plaza-name">${escapeHtml(identity.title)}</span>
              ${identity.subtitle ? `<span class="project-plaza-subtitle">${escapeHtml(identity.subtitle)}</span>` : ''}
            </span>
          </div>
          <div class="project-plaza-pill-row">
            <span class="project-plaza-pill members">● ${Number(project.member_count || 0)} 位成员</span>
            <span class="project-plaza-pill mode">${joined ? '已加入' : joinModeLabel(project.join_mode)}</span>
            ${project.latest_apk_url ? '<span class="project-plaza-pill">可安装 APK</span>' : ''}
          </div>
          <div class="project-plaza-meta">创建者：${escapeHtml(project.owner_account || '未知')} · ${escapeHtml(project.last_task_status || '准备就绪')}</div>
          <div class="project-plaza-actions">
            <span class="project-plaza-btn primary" data-plaza-action="${action}" data-id="${escapeHtml(project.id)}" data-stop="true" aria-disabled="${disabled ? 'true' : 'false'}">${state.busyId === project.id ? '处理中...' : roleActionLabel(project)}</span>
            ${project.latest_apk_url ? `<span class="project-plaza-btn" data-plaza-action="download" data-id="${escapeHtml(project.id)}" data-stop="true">下载 APK</span>` : ''}
          </div>
        </div>
      </button>
    `;
  }

  function selectedProject() {
    return state.projects.find((p) => p.id === state.selectedId) || null;
  }

  function renderDetail() {
    const project = selectedProject();
    if (!project) {
      return '<div class="project-plaza-empty">选择一个公开项目查看详情</div>';
    }
    const joined = state.joinedIds.has(project.id);
    const canApply = !joined && project.join_mode === 'approval';
    const canJoin = !joined && project.join_mode !== 'approval' && project.join_mode !== 'invite';
    const identity = projectIdentity(project);
    return `
      <h3>${escapeHtml(identity.title)}</h3>
      ${identity.subtitle ? `<div class="project-plaza-detail-subtitle">${escapeHtml(identity.subtitle)}</div>` : ''}
      <div class="project-plaza-detail-section">
        <div class="project-plaza-detail-row"><span>加入方式</span><strong>${joinModeLabel(project.join_mode)}</strong></div>
        <div class="project-plaza-detail-row"><span>成员</span><strong>${Number(project.member_count || 0)} 人</strong></div>
        <div class="project-plaza-detail-row"><span>模板</span><strong>${escapeHtml(project.template || 'custom')}</strong></div>
        <div class="project-plaza-detail-row"><span>状态</span><strong>${escapeHtml(project.last_task_status || '准备就绪')}</strong></div>
        <div class="project-plaza-detail-row"><span>更新时间</span><strong>${formatTime(project.updated_at)}</strong></div>
        <div class="project-plaza-detail-row"><span>Owner</span><strong>${escapeHtml(project.owner_account || '未知')}</strong></div>
      </div>
      <div class="project-plaza-detail-section project-plaza-actions">
        ${joined ? `<button class="project-plaza-btn primary" type="button" data-plaza-action="open" data-id="${escapeHtml(project.id)}">进入项目</button>` : ''}
        ${canJoin ? `<button class="project-plaza-btn primary" type="button" data-plaza-action="join" data-id="${escapeHtml(project.id)}">${roleActionLabel(project)}</button>` : ''}
        ${project.latest_apk_url ? `<button class="project-plaza-btn" type="button" data-plaza-action="download" data-id="${escapeHtml(project.id)}">下载 APK</button>` : ''}
      </div>
      ${canApply ? `
        <div class="project-plaza-detail-section project-plaza-apply">
          <textarea id="projectPlazaJoinMessage" placeholder="申请说明（可选）"></textarea>
          <button class="project-plaza-btn primary" type="button" data-plaza-action="apply" data-id="${escapeHtml(project.id)}">${state.busyId === project.id ? '提交中...' : '提交申请'}</button>
        </div>
      ` : ''}
      <div class="project-plaza-detail-section">
        <div class="project-plaza-status" id="projectPlazaStatus"></div>
      </div>
    `;
  }

  async function joinProject(id) {
    if (!token()) {
      setStatus('请先登录后加入项目。', true);
      return;
    }
    state.busyId = id;
    setStatus('', false);
    render();
    try {
      const res = await callApi('/api/projects/' + encodeURIComponent(id) + '/join', {
        method: 'POST',
        body: '{}'
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok || data.ok === false) {
        if (data.code === 'approval_required') {
          state.selectedId = id;
          throw new Error('该项目需要审批，请提交申请。');
        }
        throw new Error(data.error || data.message || '加入失败');
      }
      state.joinedIds.add(id);
      await reloadMainProjects();
      setStatus(data.message || '已加入项目。', false);
    } catch (e) {
      setStatus(e.message || '加入失败', true);
    } finally {
      state.busyId = '';
      render();
    }
  }

  async function applyToJoin(id) {
    if (!token()) {
      setStatus('请先登录后提交申请。', true);
      return;
    }
    const input = document.getElementById('projectPlazaJoinMessage');
    const message = input ? input.value.trim() : '';
    state.busyId = id;
    setStatus('', false);
    render();
    try {
      const res = await callApi('/api/projects/' + encodeURIComponent(id) + '/request-join', {
        method: 'POST',
        body: JSON.stringify({ message })
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok || data.ok === false) throw new Error(data.error || data.message || '申请失败');
      setStatus(data.message || '申请已提交，等待审核。', false);
    } catch (e) {
      setStatus(e.message || '申请失败', true);
    } finally {
      state.busyId = '';
      render();
    }
  }

  async function reloadMainProjects() {
    const app = bridge();
    if (typeof app.reloadProjects === 'function') {
      await app.reloadProjects();
      await loadJoinedIds();
    }
  }

  async function openJoinedProject(id) {
    let project = localProjectById(id);
    if (!project) {
      await reloadMainProjects();
      project = localProjectById(id);
    }
    if (!project) project = selectedProject();
    const app = bridge();
    if (project && typeof app.openProject === 'function') {
      closePlaza();
      app.openProject(project);
    } else {
      setStatus('项目已加入，但当前页面暂时无法打开，请刷新后重试。', true);
    }
  }

  function downloadProjectApk(id) {
    const project = state.projects.find((p) => p.id === id);
    if (!project || !project.latest_apk_url) return;
    window.open(project.latest_apk_url, '_blank', 'noopener');
  }

  function handleAction(event) {
    const actionEl = event.target.closest('[data-plaza-action]');
    if (!actionEl) return;
    const action = actionEl.dataset.plazaAction;
    const id = actionEl.dataset.id || '';
    if (actionEl.dataset.stop === 'true') {
      event.preventDefault();
      event.stopPropagation();
    }
    if (actionEl.getAttribute('aria-disabled') === 'true') return;
    if (action === 'close') closePlaza();
    else if (action === 'refresh') loadProjects((document.getElementById('projectPlazaSearch') || {}).value || '');
    else if (action === 'select') {
      state.selectedId = id;
      render();
    } else if (action === 'select-apply') {
      state.selectedId = id;
      render();
      const input = document.getElementById('projectPlazaJoinMessage');
      if (input) input.focus();
    } else if (action === 'join') joinProject(id);
    else if (action === 'apply') applyToJoin(id);
    else if (action === 'open') openJoinedProject(id);
    else if (action === 'download') downloadProjectApk(id);
  }

  function init() {
    const row = document.getElementById('projectPlazaRow');
    if (row) row.addEventListener('click', openPlaza);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init, { once: true });
  } else {
    init();
  }
})();
