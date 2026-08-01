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
    query: '',
    status: '',
    error: false
  };

  let searchTimer = 0;

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

  function cleanText(value) {
    const text = String(value == null ? '' : value).trim();
    return text && text.toLowerCase() !== 'null' ? text : '';
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
    if (joined || normalizeJoinMode(mode) !== 'approval') return '进入空间';
    return '申请加入';
  }

  function projectIdentity(project) {
    const displayName = cleanText(project.displayName) ||
      cleanText(project.display_name) ||
      cleanText(project.alias) ||
      cleanText(project.project_alias);
    const name = cleanText(project.name) || cleanText(project.title) || '未命名项目';
    const description = cleanText(project.description);
    if (displayName) {
      return { title: displayName, subtitle: description };
    }
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

  function numberField(project, keys, fallback) {
    for (const key of keys) {
      if (!Object.prototype.hasOwnProperty.call(project, key)) continue;
      const value = Number(project[key]);
      if (Number.isFinite(value) && value >= 0) return value;
    }
    return fallback;
  }

  function textField(project, keys) {
    for (const key of keys) {
      const value = cleanText(project[key]);
      if (value) return value;
    }
    return '';
  }

  function projectMemberCount(project) {
    return numberField(project, ['member_count', 'memberCount'], 0);
  }

  function projectInstallCount(project) {
    return numberField(project, ['install_count', 'installCount', 'download_count', 'downloadCount', 'downloads'], 0);
  }

  function projectCommentCount(project) {
    return numberField(project, ['comment_count', 'commentCount', 'review_count', 'reviewCount', 'comments'], 0);
  }

  function projectApkSize(project) {
    const label = textField(project, ['apk_size_label', 'apkSizeLabel', 'size_label', 'sizeLabel']);
    if (label) return label;
    const bytes = numberField(project, [
      'latest_apk_size_bytes',
      'latestApkSizeBytes',
      'apk_size_bytes',
      'apkSizeBytes',
      'size_bytes',
      'sizeBytes',
      'file_size',
      'fileSize'
    ], 0);
    return bytes > 0 ? formatBytes(bytes) : '--';
  }

  function formatBytes(bytes) {
    const mb = bytes / 1024 / 1024;
    if (mb < 0.1) return '<0.1MB';
    return (Math.round(mb * 10) / 10).toFixed(1).replace(/\.0$/, '') + 'MB';
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
    if (state.query.trim()) params.set('q', state.query.trim());
    if (filter.hasApk != null) params.set('has_apk', String(filter.hasApk));
    if (filter.sort) params.set('sort', filter.sort);
    const res = await callApi('/api/store/projects?' + params.toString(), { cache: 'no-store' });
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
    const searchArtwork = el.dataset.searchArtwork || '/assets/project_view_search_icon.png';
    el.innerHTML = `
      <div class="project-plaza-search-bar">
        <span class="project-plaza-search-icon" aria-hidden="true">
          <img src="${escapeHtml(searchArtwork)}" alt="" />
        </span>
        <input class="project-plaza-search-input" type="search" placeholder="搜索应用" value="${escapeHtml(state.query)}" data-plaza-action="search" />
      </div>
      <div class="project-plaza-filter-row">
        ${filters.map(renderFilter).join('')}
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

  function renderDescription(project, identity) {
    const desc = identity.subtitle || cleanText(project.description) || '暂无简介';
    const collapsible = Array.from(desc).length > 46;
    const shown = collapsible
      ? Array.from(desc).slice(0, 46).join('').trimEnd() + '...'
      : desc;
    return `<div class="project-plaza-desc ${collapsible ? 'is-collapsed' : ''}">应用介绍：${escapeHtml(shown)}</div>`;
  }

  function renderCard(project) {
    const identity = projectIdentity(project);
    return `
      <div class="project-plaza-card" data-id="${escapeHtml(project.id)}">
        <div class="project-plaza-card-top">
          ${renderThumb(project, identity.title)}
          <div class="project-plaza-title-block">
            <div class="project-plaza-name">${escapeHtml(identity.title)}</div>
            <div class="project-plaza-owner">创建者：${escapeHtml(project.owner_account || '未知')}</div>
          </div>
          <div class="project-plaza-actions">
            <button class="project-plaza-btn" type="button" data-plaza-action="share" data-id="${escapeHtml(project.id)}" aria-label="分享项目" title="分享项目">分享项目</button>
          </div>
        </div>
        <div class="project-plaza-stats">
          <div class="project-plaza-stat project-plaza-stat-member">
            <span class="project-plaza-stat-top">
              <span class="project-plaza-member-icon" aria-hidden="true">
                <img src="/assets/ic_plaza_member_stat.png" alt="" loading="lazy" />
              </span>
            </span>
            <span class="project-plaza-stat-label">成员：${escapeHtml(projectMemberCount(project))}</span>
          </div>
          <span class="project-plaza-stat-sep" aria-hidden="true"></span>
          <div class="project-plaza-stat"><span class="project-plaza-stat-top"><strong>${escapeHtml(projectInstallCount(project))}</strong></span><span>次安装</span></div>
          <span class="project-plaza-stat-sep" aria-hidden="true"></span>
          <div class="project-plaza-stat project-plaza-stat-size"><span class="project-plaza-stat-top"><strong>${escapeHtml(projectApkSize(project))}</strong></span><span>大小</span></div>
          <span class="project-plaza-stat-sep" aria-hidden="true"></span>
          <div class="project-plaza-stat"><span class="project-plaza-stat-top"><strong>${escapeHtml(projectCommentCount(project))}</strong></span><span>评论</span></div>
        </div>
        ${renderDescription(project, identity)}
        <span class="project-plaza-divider" aria-hidden="true"></span>
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
    else window.alert('项目暂时无法打开，请刷新后重试');
  }

  function projectShareText(project) {
    const identity = projectIdentity(project);
    const lines = [
      '一龙项目：' + identity.title,
      '创建者：' + (cleanText(project.owner_account) || '未知'),
      '成员：' + Number(project.member_count || 0),
      '加入方式：' + approvalLabel(project.join_mode)
    ];
    const desc = identity.subtitle || cleanText(project.description);
    if (desc) lines.push('简介：' + desc);
    const apkUrl = String(project.latest_apk_url || project.last_apk_url || '').trim();
    if (apkUrl) lines.push('APK：' + apkUrl);
    return lines.join('\n');
  }

  async function shareProject(id) {
    const project = state.projects.find((p) => p.id === id);
    if (!project) return;
    const text = projectShareText(project);
    const title = projectIdentity(project).title;
    try {
      if (navigator.share) {
        await navigator.share({ title, text });
        return;
      }
      if (navigator.clipboard && navigator.clipboard.writeText) {
        await navigator.clipboard.writeText(text);
        window.alert('项目分享内容已复制');
        return;
      }
    } catch (e) {
      if (e && e.name === 'AbortError') return;
    }
    window.prompt('复制项目分享内容', text);
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
    el.addEventListener('input', handleInput);
  }

  function handleInput(event) {
    const input = event.target.closest('[data-plaza-action="search"]');
    if (!input) return;
    state.query = input.value || '';
    window.clearTimeout(searchTimer);
    searchTimer = window.setTimeout(() => loadProjects(), 320);
  }

  function handleAction(event) {
    const actionEl = event.target.closest('[data-plaza-action]');
    if (actionEl) {
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
      } else if (action === 'share') {
        shareProject(id);
      } else if (action === 'download') {
        downloadProjectApk(id);
      }
      return;
    }
    const card = event.target.closest('.project-plaza-card[data-id]');
    if (!card) return;
    event.preventDefault();
    openJoinedProject(card.dataset.id || '');
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
