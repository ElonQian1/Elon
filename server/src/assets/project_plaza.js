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
    pendingIds: new Set(),
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
    const normalized = normalizeJoinMode(mode);
    if (normalized === 'approval') return '需审批';
    if (normalized === 'invite') return '仅限邀请';
    if (normalized === 'readonly') return '只读体验';
    return '无需审批';
  }

  function joinActionLabel(mode, joined) {
    if (joined) return '进入空间';
    const normalized = normalizeJoinMode(mode);
    if (normalized === 'approval') return '申请加入';
    if (normalized === 'open') return '加入项目';
    if (normalized === 'readonly') return '进入体验';
    return '查看项目';
  }

  function primaryAction(project) {
    const joined = state.joinedIds.has(project.id) || cleanText(project.viewer_role) || cleanText(project.viewerRole);
    const mode = normalizeJoinMode(project.join_mode || project.joinMode);
    let action = 'open';
    if (!joined && mode === 'approval') action = 'apply';
    else if (!joined && mode === 'open') action = 'join';
    const pending = state.pendingIds.has(project.id);
    const busy = state.busyId === project.id;
    return {
      action,
      label: busy ? '处理中…' : (pending ? '申请已提交' : joinActionLabel(mode, joined)),
      disabled: busy || pending
    };
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
        <input class="project-plaza-search-input" type="search" placeholder="搜索项目、作者" value="${escapeHtml(state.query)}" data-plaza-action="search" aria-label="搜索项目或作者" />
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
    if (state.loading) return renderLoadingState();
    if (state.error) return renderFeedbackState(
      '项目暂时没有加载出来',
      state.status || '请检查网络连接后重试。',
      '重新加载',
      'retry',
      true
    );
    if (!state.projects.length) return renderFeedbackState(
      '没有找到匹配项目',
      '可以清除搜索与筛选条件，再看看项目广场的全部内容。',
      state.query.trim() || state.filterKey !== 'all' ? '清除筛选' : '刷新项目',
      'clear',
      false
    );
    const featuredProjects = state.projects.slice(0, 5);
    return `
      <div class="project-plaza-featured-label">
        <strong>精选项目</strong>
        <span data-plaza-featured-position>01 / ${escapeHtml(String(featuredProjects.length).padStart(2, '0'))}</span>
      </div>
      <div class="project-plaza-featured-scroller" aria-label="精选项目">
        <div class="project-plaza-featured-track">
          ${featuredProjects.map((project, index) => renderFeaturedCard(project, index)).join('')}
        </div>
      </div>
      ${renderResultsHeading()}
      <div class="project-plaza-list">
        ${state.projects.map(renderCard).join('')}
      </div>
    `;
  }

  function renderLoadingState() {
    return `
      <div class="project-plaza-skeleton" aria-label="正在加载项目">
        ${[0, 1, 2].map(() => `
          <div class="project-plaza-skeleton-row">
            <i aria-hidden="true"></i>
            <span aria-hidden="true"><b></b><b></b><b></b></span>
          </div>
        `).join('')}
      </div>
    `;
    window.requestAnimationFrame(configureFeaturedCarousel);
  }

  function configureFeaturedCarousel() {
    const el = root();
    const scroller = el && el.querySelector('.project-plaza-featured-scroller');
    if (!scroller) return;
    const cards = Array.from(scroller.querySelectorAll('.project-plaza-featured-card'));
    const indicator = el.querySelector('[data-plaza-featured-position]');
    let frame = 0;
    const update = () => {
      frame = 0;
      if (!cards.length) return;
      const firstOffset = cards[0].offsetLeft;
      let activeIndex = 0;
      let activeDistance = Number.POSITIVE_INFINITY;
      cards.forEach((card, index) => {
        const distance = Math.abs((card.offsetLeft - firstOffset) - scroller.scrollLeft);
        if (distance < activeDistance) {
          activeIndex = index;
          activeDistance = distance;
        }
      });
      cards.forEach((card, index) => card.classList.toggle('is-active', index === activeIndex));
      if (indicator) indicator.textContent = String(activeIndex + 1).padStart(2, '0') + ' / ' + String(cards.length).padStart(2, '0');
    };
    scroller.addEventListener('scroll', () => {
      if (!frame) frame = window.requestAnimationFrame(update);
    }, { passive: true });
    update();
  }

  function renderFeedbackState(title, message, actionLabel, action, danger) {
    return `
      <section class="project-plaza-feedback ${danger ? 'is-danger' : ''}">
        <i aria-hidden="true"></i>
        <h2>${escapeHtml(title)}</h2>
        <p>${escapeHtml(message)}</p>
        <button type="button" data-plaza-action="${escapeHtml(action)}">${escapeHtml(actionLabel)}</button>
      </section>
    `;
  }

  function renderResultsHeading() {
    if (state.loading || state.error) return '';
    const installableCount = state.projects.filter((project) => cleanText(project.latest_apk_url) || cleanText(project.last_apk_url)).length;
    return `
      <div class="project-plaza-section-heading">
        <h2>全部</h2>
        <span>${escapeHtml(state.projects.length)} 个项目 · ${escapeHtml(installableCount)} 个可安装</span>
      </div>
    `;
  }

  function reactionSelected(project, key) {
    try {
      return window.localStorage.getItem(`project-plaza-reaction:${project.id}:${key}`) === '1';
    } catch (_) {
      return false;
    }
  }

  function toggleReaction(id, key) {
    const project = state.projects.find((item) => item.id === id);
    if (!project) return;
    const storageKey = `project-plaza-reaction:${project.id}:${key}`;
    try {
      window.localStorage.setItem(storageKey, reactionSelected(project, key) ? '0' : '1');
    } catch (_) {}
    render();
  }

  function featuredStatus(project) {
    if (state.joinedIds.has(project.id) || cleanText(project.viewer_role) || cleanText(project.viewerRole)) {
      return { label: '已加入', tone: 'success' };
    }
    const mode = normalizeJoinMode(project.join_mode || project.joinMode);
    if (mode === 'approval') return { label: '需审批', tone: 'danger' };
    if (mode === 'invite') return { label: '仅限邀请', tone: 'neutral' };
    if (mode === 'readonly') return { label: '只读体验', tone: 'neutral' };
    if (cleanText(project.latest_apk_url) || cleanText(project.last_apk_url)) {
      return { label: '可安装', tone: 'success' };
    }
    return { label: '无需审批', tone: 'success' };
  }

  function projectBuildStatus(project) {
    const raw = cleanText(project.last_task_status) || cleanText(project.lastTaskStatus);
    const normalized = raw.toLowerCase().replace(/-/g, '_');
    if (['success', 'succeeded', 'completed', 'complete', 'passed', 'ready', 'done'].includes(normalized)) {
      return { label: '构建成功', tone: 'success' };
    }
    if (['failed', 'failure', 'error', 'cancelled', 'canceled', 'blocked'].includes(normalized)) {
      return { label: '构建异常', tone: 'danger' };
    }
    if (['running', 'building', 'pending', 'queued', 'in_progress', 'processing', 'working'].includes(normalized)) {
      return { label: '构建中', tone: 'neutral' };
    }
    return { label: '暂无构建', tone: 'neutral' };
  }

  function renderFeaturedCard(project, index) {
    const identity = projectIdentity(project);
    const description = identity.subtitle || cleanText(project.description) || '这个项目还没有填写简介。';
    const status = featuredStatus(project);
    const action = primaryAction(project);
    const build = projectBuildStatus(project);
    const cover = Array.from(identity.title.trim())[0] || '项';
    const icon = iconUrlOf(project);
    const owner = cleanText(project.owner_account) || '未知';
    const members = Math.max(0, Number(project.member_count || 0));
    return `
      <article class="project-plaza-featured-card ${index === 0 ? 'is-active' : ''}" data-id="${escapeHtml(project.id)}">
        <header class="project-plaza-featured-head">
          <div class="project-plaza-featured-rank"><strong>精选</strong><span>${escapeHtml(String(index + 1).padStart(2, '0'))}</span></div>
          <div class="project-plaza-featured-status is-${escapeHtml(status.tone)}"><i aria-hidden="true"></i><span>${escapeHtml(status.label)}</span></div>
        </header>
        <div class="project-plaza-featured-body">
          <div class="project-plaza-featured-identity">
            <span class="project-plaza-featured-cover" aria-hidden="true">
              ${escapeHtml(cover)}
              ${icon ? `<img src="${escapeHtml(icon)}" alt="" loading="lazy" onerror="this.remove()" />` : ''}
            </span>
            <div class="project-plaza-featured-copy">
              <h3>${escapeHtml(identity.title)}</h3>
              <p>${escapeHtml(description)}</p>
            </div>
          </div>
          <div class="project-plaza-featured-facts">
            <span class="project-plaza-featured-fact"><small>创建者</small><b>${escapeHtml(owner)}</b></span>
            <span class="project-plaza-featured-fact"><small>成员</small><b>${escapeHtml(members)} 人</b></span>
            <span class="project-plaza-featured-fact"><small>最近构建</small><b class="is-${escapeHtml(build.tone)}">${escapeHtml(build.label)}</b></span>
          </div>
        </div>
        <div class="project-plaza-featured-actions">
          <button class="project-plaza-featured-primary" type="button" data-plaza-action="${escapeHtml(action.action)}" data-id="${escapeHtml(project.id)}" aria-label="${escapeHtml(action.label)}${escapeHtml(identity.title)}" aria-disabled="${action.disabled ? 'true' : 'false'}" ${action.disabled ? 'disabled' : ''}>${escapeHtml(action.label)}</button>
          <div class="project-plaza-reactions" aria-label="项目偏好">
            <button class="project-plaza-reaction is-star ${reactionSelected(project, 'favorite') ? 'is-selected' : ''}" type="button" data-plaza-action="favorite" data-id="${escapeHtml(project.id)}" aria-label="${reactionSelected(project, 'favorite') ? '取消收藏' : '收藏'}${escapeHtml(identity.title)}"></button>
            <button class="project-plaza-reaction is-heart ${reactionSelected(project, 'liked') ? 'is-selected' : ''}" type="button" data-plaza-action="liked" data-id="${escapeHtml(project.id)}" aria-label="${reactionSelected(project, 'liked') ? '取消点赞' : '点赞'}${escapeHtml(identity.title)}"></button>
          </div>
        </div>
      </article>
    `;
  }

  function renderCard(project) {
    const identity = projectIdentity(project);
    const description = identity.subtitle || cleanText(project.description) || '这个项目还没有填写简介。';
    const icon = iconUrlOf(project);
    const build = projectBuildStatus(project);
    const owner = cleanText(project.owner_account) || cleanText(project.ownerAccount) || '未知';
    const members = projectMemberCount(project);
    return `
      <article class="project-plaza-card" data-id="${escapeHtml(project.id)}">
        <span class="project-plaza-list-thumb" aria-hidden="true">
          ${escapeHtml(projectInitial(identity.title))}
          ${icon ? `<img src="${escapeHtml(icon)}" alt="" loading="lazy" onerror="this.remove()" />` : ''}
        </span>
        <div class="project-plaza-title-block">
          <h3 class="project-plaza-name">${escapeHtml(identity.title)}</h3>
          <p class="project-plaza-desc">${escapeHtml(description)}</p>
          <div class="project-plaza-list-meta"><span>${escapeHtml(owner)} · ${escapeHtml(members)} 人</span><b class="is-${escapeHtml(build.tone)}"><i aria-hidden="true"></i>${escapeHtml(build.label)}</b></div>
        </div>
        <button class="project-plaza-open" type="button" data-plaza-action="open" data-id="${escapeHtml(project.id)}" aria-label="打开${escapeHtml(identity.title)}"><img src="/assets/project_view_chevron.png" alt="" /></button>
      </article>
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
      window.alert('已加入项目，点击按钮进入空间');
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
      state.pendingIds.add(id);
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
      } else if (action === 'favorite' || action === 'liked') {
        toggleReaction(id, action);
      } else if (action === 'retry') {
        loadProjects();
      } else if (action === 'clear') {
        state.query = '';
        state.filterKey = 'all';
        loadProjects();
      }
      return;
    }
    const card = event.target.closest('.project-plaza-card[data-id], .project-plaza-featured-card[data-id]');
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
    const inlineRoot = root();
    if (inlineRoot && !inlineRoot.classList.contains('hidden')) openInline();
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
