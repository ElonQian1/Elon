(function () {
  const ROOT_ID = 'projectHomeRoot';

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

  function projects() {
    const app = bridge();
    return typeof app.getProjects === 'function' ? app.getProjects() : [];
  }

  const plazaBannerState = {
    loading: false,
    loaded: false,
    projects: []
  };

  const plazaBannerSlots = [
    { x: -3, y: 38, size: 50, rot: -14 },
    { x: 20, y: 24, size: 50, rot: -14 },
    { x: 38, y: -3, size: 50, rot: -14 },
    { x: 64, y: -6, size: 50, rot: -14 },
    { x: 79, y: 19, size: 50, rot: -14 },
    { x: 98, y: 28, size: 50, rot: -14 },
    { x: 8, y: 86, size: 50, rot: -14 },
    { x: 22, y: 62, size: 50, rot: -14 },
    { x: 43, y: 50, size: 50, rot: -14 },
    { x: 68, y: 78, size: 50, rot: -14 },
    { x: 84, y: 50, size: 50, rot: -14 },
    { x: 105, y: 84, size: 50, rot: -14 }
  ];
  const plazaBannerFocusSlot = { x: 58, y: 40, size: 56, rot: -14 };

  function memberCountOf(project) {
    return Number(project.member_count || project.memberCount || project.members || 0) || 0;
  }

  function sortedBannerProjects() {
    return plazaBannerState.projects
      .slice()
      .sort((a, b) => memberCountOf(b) - memberCountOf(a) || String(titleOf(a)).localeCompare(String(titleOf(b))))
      .slice(0, plazaBannerSlots.length + 1);
  }

  function ensurePlazaBannerProjects() {
    if (plazaBannerState.loading || plazaBannerState.loaded) return;
    plazaBannerState.loading = true;
    fetch('/api/store/projects?limit=18&sort=members', { cache: 'no-store' })
      .then((res) => res.json().then((data) => ({ ok: res.ok, data })))
      .then(({ ok, data }) => {
        if (!ok) throw new Error(data && data.error ? data.error : 'load failed');
        plazaBannerState.projects = Array.isArray(data.projects) ? data.projects : [];
        plazaBannerState.loaded = true;
      })
      .catch(() => {
        plazaBannerState.projects = [];
        plazaBannerState.loaded = true;
      })
      .finally(() => {
        plazaBannerState.loading = false;
        render();
      });
  }

  function formatTime(value) {
    const app = bridge();
    if (typeof app.formatTime === 'function') return app.formatTime(value);
    return value ? String(value).replace('T', ' ').slice(0, 16) : '时间';
  }

  function titleOf(project) {
    return project.name || project.title || '未命名项目';
  }

  function projectInitial(project) {
    const title = String(titleOf(project)).trim();
    if (title.startsWith('一龙')) return '龙';
    return Array.from(title)[0] || '项';
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

  function renderProjectThumb(project) {
    const iconUrl = iconUrlOf(project);
    const label = escapeHtml(projectInitial(project));
    return `
      <span class="project-home-thumb" aria-hidden="true">
        ${label}
        ${iconUrl ? `<img src="${escapeHtml(iconUrl)}" alt="" loading="lazy" onerror="this.remove()" />` : ''}
      </span>
    `;
  }

  function roleOf(project) {
    return String(project.role || '').trim().toLowerCase();
  }

  function isJointProject(project) {
    const role = roleOf(project);
    return Boolean(
      project.isJointProject ||
      project.is_joint_project ||
      project.collaborationProjectId ||
      project.collaboration_project_id ||
      (role && role !== 'owner')
    );
  }

  function stageOf(project) {
    return project.last_task_status || project.status || project.stage || '待提交需求';
  }

  function conversationCount(project) {
    if (Array.isArray(project.conversations)) return project.conversations.length || 1;
    return project.conversation_count || project.conversationCount || project.chat_count || 1;
  }

  function projectTime(project) {
    return formatTime(project.updated_at || project.updatedAt || project.updated_at_ms) || '时间';
  }

  function renderBannerIcon(project, slot, extraClass) {
    const iconUrl = project ? iconUrlOf(project) : '';
    const label = project ? escapeHtml(projectInitial(project)) : '';
    const fontSize = Math.round(slot.size * 0.36);
    return `
      <span class="project-plaza-tile ${extraClass || ''}" style="--x:${slot.x}%;--y:${slot.y}%;--size:${slot.size}px;--font:${fontSize}px;--rot:${slot.rot}deg">
        ${label}
        ${iconUrl ? `<img src="${escapeHtml(iconUrl)}" alt="" loading="lazy" onerror="this.remove()" />` : ''}
      </span>
    `;
  }

  function renderPlazaTiles() {
    const bannerProjects = sortedBannerProjects();
    const focus = bannerProjects[0] || null;
    const rest = bannerProjects.slice(1);
    const tiles = plazaBannerSlots.map((slot, index) => renderBannerIcon(rest[index] || null, slot));
    tiles.push(renderBannerIcon(focus, plazaBannerFocusSlot, 'project-plaza-focus-tile'));
    return tiles.join('');
  }

  function renderPlazaBanner() {
    ensurePlazaBannerProjects();
    return `
      <button class="project-plaza-banner" type="button" data-project-home-action="plaza" aria-label="进入项目广场">
        <span class="project-plaza-pattern" aria-hidden="true">${renderPlazaTiles()}</span>
        <span class="project-plaza-title">项目广场</span>
        <span class="project-plaza-search" aria-hidden="true"></span>
      </button>
    `;
  }

  function renderSection(title, items, emptyAction) {
    const cells = items.slice();
    if (!cells.length) {
      cells.push(null, null);
    } else if (cells.length % 2) {
      cells.push(null);
    }
    return `
      <div class="project-home-section-head">
        <div class="project-home-section-title">${escapeHtml(title)}</div>
        <div class="project-home-section-arrow">›</div>
      </div>
      <div class="project-home-grid">
        ${cells.map((project) => project ? renderCard(project) : renderEmptyCard(emptyAction)).join('')}
      </div>
    `;
  }

  function renderCard(project) {
    const joint = isJointProject(project);
    const kind = joint ? '联合开发' : '个人独立';
    const meta = `${kind} · ${conversationCount(project)}个会话 · ${stageOf(project)}`;
    const app = bridge();
    const active = typeof app.isCurrentProject === 'function' && app.isCurrentProject(project);
    return `
      <button class="project-home-card ${active ? 'active' : ''}" type="button" data-project-home-action="open" data-project-id="${escapeHtml(project.id)}">
        ${renderProjectThumb(project)}
        <span class="project-home-info">
          <span class="project-home-title-row">
            <span class="project-home-name">${escapeHtml(titleOf(project))}</span>
            <span class="project-home-time">${escapeHtml(projectTime(project))}</span>
          </span>
          <span class="project-home-meta">${escapeHtml(meta)}</span>
        </span>
      </button>
    `;
  }

  function renderEmptyCard(action) {
    const attr = action ? ` data-project-home-action="${action}" tabindex="0"` : ' tabindex="-1"';
    return `<button class="project-home-empty-card" type="button"${attr} aria-label="空项目位"></button>`;
  }

  function render() {
    const root = document.getElementById(ROOT_ID);
    if (!root) return;
    attachEvents(root);
    const all = projects();
    const personal = all.filter((project) => !isJointProject(project));
    const joint = all.filter(isJointProject);
    root.innerHTML = [
      renderPlazaBanner(),
      renderSection('个人项目', personal, 'create'),
      renderSection('联合项目', joint, null)
    ].join('');
  }

  function attachEvents(root) {
    if (root.dataset.projectHomeReady === 'true') return;
    root.dataset.projectHomeReady = 'true';
    root.addEventListener('click', handleAction);
    root.addEventListener('keydown', (event) => {
      if (event.key !== 'Enter' && event.key !== ' ') return;
      const actionEl = event.target.closest('[data-project-home-action]');
      if (!actionEl) return;
      event.preventDefault();
      runAction(actionEl);
    });
  }

  function handleAction(event) {
    const actionEl = event.target.closest('[data-project-home-action]');
    if (!actionEl) return;
    runAction(actionEl);
  }

  function runAction(actionEl) {
    const action = actionEl.dataset.projectHomeAction;
    const app = bridge();
    if (action === 'create') {
      if (typeof app.openNewProject === 'function') app.openNewProject();
      return;
    }
    if (action === 'plaza') {
      if (typeof app.openProjectPlaza === 'function') app.openProjectPlaza();
      return;
    }
    if (action === 'open') {
      const id = actionEl.dataset.projectId;
      const project = projects().find((item) => item && item.id === id);
      if (project && typeof app.openProject === 'function') app.openProject(project);
    }
  }

  window.ElonProjectHome = { render };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', render, { once: true });
  } else {
    render();
  }
})();
