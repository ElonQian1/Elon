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

  function formatTime(value) {
    const app = bridge();
    if (typeof app.formatTime === 'function') return app.formatTime(value);
    return value ? String(value).replace('T', ' ').slice(0, 16) : '';
  }

  function isWorkingStatus(status) {
    const app = bridge();
    if (typeof app.isWorkingStatus === 'function') return app.isWorkingStatus(status);
    const normalized = String(status || '').toLowerCase();
    return ['running', 'working', 'pending', 'queued'].includes(normalized);
  }

  function titleOf(project) {
    return project.name || project.title || '未命名项目';
  }

  function subtitleOf(project, joint) {
    const description = String(project.description || project.subtitle || '').trim();
    if (description) return description;
    if (project.workspace_path) return 'PC 本地项目';
    if (project.source_type === 'github') return 'GitHub 项目';
    return joint ? '联合开发项目' : '个人独立项目';
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

  function roleLabel(project, joint) {
    const role = roleOf(project);
    if (role === 'owner') return '个人';
    if (role === 'editor') return '协作';
    if (role === 'observer' || role === 'viewer') return '只读';
    return joint ? '联合' : '个人';
  }

  function statusLabel(project) {
    return project.last_task_status || project.status || '准备就绪';
  }

  function initialFor(project) {
    const chars = Array.from(String(titleOf(project)).trim());
    return (chars[0] || 'P').toUpperCase();
  }

  function renderHeader(all, personal, joint) {
    return `
      <div class="project-home-actions">
        <button class="project-home-action" type="button" data-project-home-action="create">＋ 新建项目</button>
        <button class="project-home-action" type="button" data-project-home-action="plaza">项目广场</button>
      </div>
      <div class="project-home-summary" aria-label="项目统计">
        <div class="project-home-stat">
          <div class="project-home-stat-value">${all.length}</div>
          <div class="project-home-stat-label">全部项目</div>
        </div>
        <div class="project-home-stat">
          <div class="project-home-stat-value">${personal.length}</div>
          <div class="project-home-stat-label">个人独立项目</div>
        </div>
        <div class="project-home-stat">
          <div class="project-home-stat-value">${joint.length}</div>
          <div class="project-home-stat-label">联合开发项目</div>
        </div>
      </div>
    `;
  }

  function renderSection(title, items, emptyText) {
    return `
      <div class="project-home-section-title">${escapeHtml(title)}</div>
      <div class="project-home-list">
        ${items.length ? items.map(renderCard).join('') : `<div class="project-home-empty">${escapeHtml(emptyText)}</div>`}
      </div>
    `;
  }

  function renderCard(project) {
    const joint = isJointProject(project);
    const status = statusLabel(project);
    const working = isWorkingStatus(status);
    const updated = formatTime(project.updated_at || project.updatedAt);
    const meta = [
      `<span class="project-home-status ${working ? 'working' : ''}">${escapeHtml(status)}</span>`,
      updated ? `<span>${escapeHtml(updated)}</span>` : '',
      project.node_id ? '<span>PC 节点</span>' : ''
    ].filter(Boolean).join('');
    return `
      <div class="project-home-card" role="button" tabindex="0" data-project-home-action="open" data-project-id="${escapeHtml(project.id)}">
        <div class="project-home-icon ${joint ? 'joint' : ''}">${escapeHtml(initialFor(project))}</div>
        <div class="project-home-main">
          <div class="project-home-title-row">
            <div class="project-home-name">${escapeHtml(titleOf(project))}</div>
            <span class="project-home-badge ${joint ? '' : 'personal'}">${escapeHtml(roleLabel(project, joint))}</span>
          </div>
          <div class="project-home-subtitle">${escapeHtml(subtitleOf(project, joint))}</div>
          <div class="project-home-meta">${meta}</div>
        </div>
        <div class="project-home-arrow">›</div>
      </div>
    `;
  }

  function render() {
    const root = document.getElementById(ROOT_ID);
    if (!root) return;
    attachEvents(root);
    const all = projects();
    const personal = all.filter((project) => !isJointProject(project));
    const joint = all.filter(isJointProject);
    root.innerHTML = [
      renderHeader(all, personal, joint),
      renderSection('个人独立项目', personal, '暂无个人独立项目'),
      renderSection('联合开发项目', joint, '暂无联合开发项目')
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
