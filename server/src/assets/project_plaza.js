(function () {
  const ROOT_ID = 'projectPlazaInlineRoot';
  const PAGE_LIMIT = 50;
  const MAX_PROJECTS = 200;
  const filters = [
    { key: 'all', label: '全部' },
    { key: 'installable', label: '可安装', hasApk: true },
    { key: 'no_approval', label: '无审批', noApprovalOnly: true },
    { key: 'joined', label: '已加入', joinedOnly: true },
    { key: 'popular', label: '最热门', sort: 'members' }
  ];

  const state = {
    loaded: false,
    loading: false,
    projects: [],
    joinedIds: new Set(),
    busyId: '',
    filterKey: 'all',
    status: '',
    error: false
  };

  function bridge() {
    return window.ElonWebApp || {};
  }

  function root() {
    return document.getElementById(ROOT_ID);
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

  function activeFilter() {
    return filters.find((item) => item.key === state.filterKey) || filters[0];
  }

  function normalizeJoinMode(mode) {
    const value = String(mode || 'open').trim().toLowerCase();
    return value || 'open';
  }

  function approvalLabel(mode) {
    return normalizeJoinMode(mode) === 'approval' ? '需审批' : '无需审批';
  }

  function joinActionLabel(mode, joined) {
    if (joined) return '进入项目';
    if (normalizeJoinMode(mode) === 'open') return '加入';
    if (normalizeJoinMode(mode) === 'readonly') return '进入体验';
    return '申请加入';
  }

  function projectIdentity(project) {
    const name = String(project.name || '').trim() || '未命名项目';
    const description = String(project.description || '').trim();
    if (description && /^[A-Za-z0-9._-]{3,24}$/.test(name) && /[A-Za-z]/.test(name) && Array.from(description).length <= 24) {
      return { title: description, subtitle: '项目代号：' + name };
    }
    return { title: name, subtitle: description };
  }

  function projectInitial(title) {
    return Array.from(String(title || '').trim())[0] || '项';
  }

  function iconUrlOf(project) {
    return [
      project.iconDataUrl,
      project.icon_data_url,
      project.iconUrl,
      project.icon_url,
      project.icon,
      project.avatar,
      project.logo
    ].find((value) => typeof value === 'string' && value.trim()) || '';
  }

  function renderThumb(project, title) {
    const icon = iconUrlOf(project);
    return `
      <span class="project-plaza-thumb" aria-hidden="true">
        ${escapeHtml(projectInitial(title))}
        ${icon ? `<img src="${escapeHtml(icon)}" alt="" loading="lazy" onerror="this.remove()" />` : ''}
      </span>
    `;
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

  async function loadProjects() {
    state.loading = true;
    state.loaded = true;
    state.status = '';
    state.error = false;
    render();
    try {
      const filter = activeFilter();
      const projects = await fetchAllProjects(filter);
      await loadJoinedIds();
      state.projects = applyClientFilter(projects, filter);
    } catch (e) {
      state.projects = [];
      state.status = e.message || '加载失败';
      state.error = true;
    } finally {
      state.loading = false;
      render();
    }
  }

  async function fetchAllProjects(filter) {
    const projects = [];
    const seenIds = new Set();
    let offset = 0;
    while (projects.length < MAX_PROJECTS) {
      const page = await fetchProjectPage(filter, offset);
      if (!page.length) break;
      page.forEach((project) => {
        if (!project || !project.id || seenIds.has(project.id) || projects.length >= MAX_PROJECTS) return;
        seenIds.add(project.id);
        projects.push(project);
      });
      if (page.length < PAGE_LIMIT) break;
      offset += PAGE_LIMIT;
    }
    return projects;
  }

  async function fetchProjectPage(filter, offset) {
    const params = new URLSearchParams({
      limit: String(PAGE_LIMIT),
      offset: String(offset)
    });
    if (filter.hasApk != null) params.set('has_apk', String(filter.hasApk));
    if (filter.sort) params.set('sort', filter.sort);
    const res = await fetch('/api/store/projects?' + params.toString(), { cache: 'no-store' });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || '加载失败');
    return Array.isArray(data.projects) ? data.projects : [];
  }

  function applyClientFilter(projects, filter) {
    return projects.filter((project) => {
      const joined = state.joinedIds.has(project.id);
      return (!filter.joinedOnly || joined) &&
        (!filter.noApprovalOnly || normalizeJoinMode(project.join_mode) !== 'approval');
    });
  }

  function openInline() {
    attachEvents();
    if (!state.loaded) loadProjects();
    else render();
  }

  function render() {
    const el = root();
    if (!el) return;
    el.innerHTML = `
      <div class="project-plaza-search-panel">
        <h2 class="project-plaza-search-title">搜索</h2>
        <div class="project-plaza-filter-row">
          ${filters.map(renderFilter).join('')}
        </div>
      </div>
      <div class="project-plaza-results">
        ${renderResults()}
      </div>
    `;
  }

  function renderFilter(filter) {
    return `
      <button class="project-plaza-filter ${filter.key === state.filterKey ? 'active' : ''}" type="button" data-plaza-action="filter" data-filter="${escapeHtml(filter.key)}">
        ${escapeHtml(filter.label)}
      </button>
    `;
  }

  function renderResults() {
    if (state.loading) return '<div class="project-plaza-empty">加载中...</div>';
    if (state.error) return `<div class="project-plaza-error">${escapeHtml(state.status)}</div>`;
    if (!state.projects.length) return '<div class="project-plaza-empty">暂无匹配项目</div>';
    return state.projects.map(renderCard).join('');
  }

  function renderCard(project) {
    const identity = projectIdentity(project);
    const joined = state.joinedIds.has(project.id);
    const busy = state.busyId === project.id;
    const apkUrl = String(project.latest_apk_url || project.last_apk_url || '').trim();
    const mode = normalizeJoinMode(project.join_mode);
    const action = joined ? 'open' : mode === 'approval' ? 'apply' : 'join';
    const actionLabel = busy ? '处理中...' : joinActionLabel(mode, joined);
    return `
      <div class="project-plaza-card" data-id="${escapeHtml(project.id)}">
        <div class="project-plaza-card-head">
          <div class="project-plaza-name">${escapeHtml(identity.title)}</div>
          <div class="project-plaza-status" style="--dot:${mode === 'approval' ? '#F04B4F' : '#58BE6A'}">${approvalLabel(mode)}</div>
          <div class="project-plaza-status" style="--dot:${apkUrl ? '#58BE6A' : '#777777'}">${apkUrl ? '可安装' : '暂无APK'}</div>
        </div>
        <div class="project-plaza-card-body">
          <div class="project-plaza-card-main">
            <div class="project-plaza-info-row">
              ${renderThumb(project, identity.title)}
              <div class="project-plaza-details">
                <span>创建者：${escapeHtml(project.owner_account || '未知')}</span>
                <span>成员：${escapeHtml(Number(project.member_count || 0))}</span>
              </div>
            </div>
            <span class="project-plaza-divider" aria-hidden="true"></span>
            <span class="project-plaza-desc">简介：${escapeHtml(identity.subtitle || project.description || '暂无简介')}</span>
          </div>
          <span class="project-plaza-time">时间</span>
          <div class="project-plaza-actions">
            <button class="project-plaza-btn" type="button" data-plaza-action="${action}" data-id="${escapeHtml(project.id)}">${escapeHtml(actionLabel)}</button>
            <button class="project-plaza-btn" type="button" data-plaza-action="download" data-id="${escapeHtml(project.id)}" aria-disabled="${apkUrl ? 'false' : 'true'}">下载APK</button>
          </div>
        </div>
      </div>
    `;
  }

  async function joinProject(id) {
    if (!token()) {
      window.alert('请先登录后加入项目');
      return;
    }
    state.busyId = id;
    render();
    try {
      const res = await callApi('/api/projects/' + encodeURIComponent(id) + '/join', {
        method: 'POST',
        body: '{}'
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok || data.ok === false) throw new Error(data.error || data.message || '加入失败');
      state.joinedIds.add(id);
      await reloadMainProjects();
      await openJoinedProject(id);
    } catch (e) {
      window.alert(e.message || '加入失败');
    } finally {
      state.busyId = '';
      render();
    }
  }

  async function applyToJoin(id) {
    if (!token()) {
      window.alert('请先登录后提交申请');
      return;
    }
    state.busyId = id;
    render();
    try {
      const res = await callApi('/api/projects/' + encodeURIComponent(id) + '/request-join', {
        method: 'POST',
        body: JSON.stringify({ message: '' })
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok || data.ok === false) throw new Error(data.error || data.message || '申请失败');
      window.alert(data.message || '申请已提交，等待审核');
    } catch (e) {
      window.alert(e.message || '申请失败');
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
    if (!project) project = state.projects.find((p) => p.id === id);
    const app = bridge();
    if (project && typeof app.openProject === 'function') app.openProject(project);
    else window.alert('项目已加入，但当前页面暂时无法打开，请刷新后重试');
  }

  function downloadProjectApk(id) {
    const project = state.projects.find((p) => p.id === id);
    const url = project && String(project.latest_apk_url || project.last_apk_url || '').trim();
    if (!url) return;
    window.open(url, '_blank', 'noopener');
  }

  function attachEvents() {
    const el = root();
    if (!el || el.dataset.projectPlazaReady === 'true') return;
    el.dataset.projectPlazaReady = 'true';
    el.addEventListener('click', handleAction);
  }

  function handleAction(event) {
    const actionEl = event.target.closest('[data-plaza-action]');
    if (!actionEl) return;
    event.preventDefault();
    if (actionEl.getAttribute('aria-disabled') === 'true') return;
    const action = actionEl.dataset.plazaAction;
    const id = actionEl.dataset.id || '';
    if (action === 'filter') {
      state.filterKey = actionEl.dataset.filter || 'all';
      loadProjects();
    } else if (action === 'join') {
      joinProject(id);
    } else if (action === 'apply') {
      applyToJoin(id);
    } else if (action === 'open') {
      openJoinedProject(id);
    } else if (action === 'download') {
      downloadProjectApk(id);
    }
  }

  function init() {
    const row = document.getElementById('projectPlazaRow');
    if (row) row.addEventListener('click', () => {
      const app = bridge();
      if (typeof app.openProjectPlaza === 'function') app.openProjectPlaza();
      else openInline();
    });
  }

  window.ElonProjectPlaza = {
    open: openInline,
    openInline,
    close: function () {}
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init, { once: true });
  } else {
    init();
  }
})();
