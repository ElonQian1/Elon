(function () {
  function create(deps) {
    const clean = deps.clean;
    const escapeHtml = deps.escapeHtml;
    const markdown = deps.markdown || {};
    const refreshActiveChannel = deps.refreshActiveChannel;

    function buildContext(messages) {
      const tasks = new Map();
      (messages || []).forEach((message) => {
        const kind = messageKind(message);
        const taskId = taskIdOf(message);
        if (!taskId) return;
        if (!tasks.has(taskId)) {
          tasks.set(taskId, {
            taskId,
            progressCount: 0,
            result: null,
            failed: false
          });
        }
        const task = tasks.get(taskId);
        if (kind === 'ai_progress') task.progressCount += 1;
        if (kind === 'ai_result') {
          const content = messageText(message);
          task.result = message;
          task.failed = /失败|错误|error|failed/i.test(content);
        }
      });
      return { tasks };
    }

    function renderMessage(message, context) {
      const kind = messageKind(message);
      if (!['ai_task', 'ai_progress', 'ai_result'].includes(kind)) return '';
      if (kind === 'ai_task') return renderTaskStart(message, context);
      if (kind === 'ai_progress') return renderProgress(message);
      return renderResult(message);
    }

    function bindActions(root) {
      if (!root) return;
      root.querySelectorAll('[data-dev-task-action="refresh"]').forEach((button) => {
        button.addEventListener('click', () => {
          if (typeof refreshActiveChannel === 'function') refreshActiveChannel();
        });
      });
    }

    function hasOpenTasks(messages, context) {
      if (!Array.isArray(messages) || !messages.length) return false;
      const built = context || buildContext(messages);
      return Array.from(built.tasks.values()).some((task) => !task.result);
    }

    function renderTaskStart(message, context) {
      const taskId = taskIdOf(message);
      const task = taskId ? context.tasks.get(taskId) : null;
      const status = statusForTask(task);
      const request = taskRequest(messageText(message));
      return cardHtml({
        tone: status.tone,
        eyebrow: 'AI 开发任务',
        title: status.label,
        body: request || '已提交开发任务。',
        taskId,
        meta: task && task.progressCount ? `${task.progressCount} 条进度` : '等待执行回写',
        actions: true
      });
    }

    function renderProgress(message) {
      return cardHtml({
        tone: 'running',
        eyebrow: '执行进度',
        title: 'Agent 正在处理',
        body: messageText(message),
        taskId: taskIdOf(message),
        meta: '来自运行时'
      });
    }

    function renderResult(message) {
      const content = messageText(message);
      const failed = /失败|错误|error|failed/i.test(content);
      return cardHtml({
        tone: failed ? 'failed' : 'done',
        eyebrow: '执行结果',
        title: failed ? '任务失败' : '任务完成',
        body: renderBody(content, true),
        bodyIsHtml: true,
        taskId: taskIdOf(message),
        meta: failed ? '需要继续处理' : '可以检查变更',
        actions: true
      });
    }

    function cardHtml(card) {
      const taskId = clean(card.taskId);
      const body = card.bodyIsHtml
        ? card.body
        : `<div class="dev-task-card-text">${escapeHtml(card.body || '')}</div>`;
      const taskMeta = taskId
        ? `<span title="${escapeHtml(taskId)}">任务 ${escapeHtml(shortId(taskId))}</span>`
        : '';
      const actions = card.actions
        ? '<button type="button" data-dev-task-action="refresh">刷新</button>'
        : '';
      return `<div class="message-content dev-task-wrap">
        <section class="dev-task-card ${escapeHtml(card.tone || 'running')}">
          <div class="dev-task-card-head">
            <span>${escapeHtml(card.eyebrow || '开发任务')}</span>
            <strong>${escapeHtml(card.title || '')}</strong>
          </div>
          ${body}
          <div class="dev-task-card-foot">
            <div>${taskMeta}<span>${escapeHtml(card.meta || '')}</span></div>
            ${actions}
          </div>
        </section>
      </div>`;
    }

    function renderBody(content, allowMarkdown) {
      if (allowMarkdown && markdown.renderMessage) {
        return markdown.renderMessage(content, {
          className: 'dev-task-result',
          copy: true
        });
      }
      return `<div class="dev-task-card-text">${escapeHtml(content || '')}</div>`;
    }

    function statusForTask(task) {
      if (task && task.result) {
        return task.failed
          ? { tone: 'failed', label: '任务失败' }
          : { tone: 'done', label: '任务完成' };
      }
      if (task && task.progressCount > 0) return { tone: 'running', label: '执行中' };
      return { tone: 'queued', label: '已排队' };
    }

    function messageKind(message) {
      return clean(message.kind || message.role || message.message_kind).toLowerCase();
    }

    function messageText(message) {
      return clean(message.content || message.text || message.message);
    }

    function taskIdOf(message) {
      return clean(message.task_id || message.taskId);
    }

    function taskRequest(content) {
      return clean(content).replace(/^发起\s*AI\s*开发任务[:：]\s*/i, '');
    }

    function shortId(taskId) {
      const value = clean(taskId);
      return value.length > 12 ? `${value.slice(0, 8)}...` : value;
    }

    return { buildContext, renderMessage, bindActions, hasOpenTasks };
  }

  window.ElonPcDevTasks = { create };
})();
