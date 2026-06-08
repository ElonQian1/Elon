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
    return value ? String(value).replace('T', ' ').slice(0, 16) : '时间';
  }

  function titleOf(project) {
    return project.name || project.title || '未命名项目';
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

  function renderPlazaTiles() {
    const tiles = [];
    for (let row = 0; row < 6; row += 1) {
      for (let col = 0; col < 10; col += 1) {
        const left = (col * 12) - 8 + (row % 2 ? 6 : 0);
        const top = (row * 30) - 16;
        tiles.push(`<span class="project-plaza-tile" style="left:${left}%;top:${top}%"></span>`);
      }
    }
    return tiles.join('');
  }

  function renderPlazaBanner() {
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
        <span class="project-home-thumb" aria-hidden="true"></span>
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
