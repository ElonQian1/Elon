(function () {
  function create(deps) {
    const clean = deps.clean;
    const escapeHtml = deps.escapeHtml;
    const markdown = deps.markdown || {};
    const refreshActiveChannel = deps.refreshActiveChannel;
    const cancelTask = deps.cancelTask;
    const approveTool = deps.approveTool;
    const draftContinuation = deps.draftContinuation;
    const continuationDrafts = new Map();

    function buildContext(messages) {
      continuationDrafts.clear();
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
            request: '',
            resultText: '',
            failed: false,
            canceled: false
          });
        }
        const task = tasks.get(taskId);
        if (kind === 'ai_task') task.request = taskRequest(messageText(message)) || task.request;
        if (kind === 'ai_progress') task.progressCount += 1;
        if (kind === 'ai_result') {
          const content = messageText(message);
          task.result = message;
          task.resultText = content;
          task.canceled = /停止|取消|canceled|cancelled/i.test(content);
          task.failed = !task.canceled && /失败|错误|error|failed/i.test(content);
        }
      });
      return { tasks };
    }

    function renderMessage(message, context) {
      const kind = messageKind(message);
      if (!['ai_task', 'ai_progress', 'ai_result'].includes(kind)) return '';
      if (kind === 'ai_task') return renderTaskStart(message, context);
      if (kind === 'ai_progress') return renderProgress(message, context);
      return renderResult(message, context);
    }

    function bindActions(root) {
      if (!root) return;
      root.querySelectorAll('[data-dev-task-action="refresh"]').forEach((button) => {
        button.addEventListener('click', () => {
          if (typeof refreshActiveChannel === 'function') refreshActiveChannel();
        });
      });
      root.querySelectorAll('[data-dev-task-action="cancel"]').forEach((button) => {
        button.addEventListener('click', async () => {
          const taskId = clean(button.dataset.taskId);
          if (!taskId || typeof cancelTask !== 'function') return;
          const ok = window.confirm('停止这个 AI 开发任务？当前运行中的命令会被中断。');
          if (!ok) return;
          button.disabled = true;
          button.textContent = '停止中...';
          try {
            await cancelTask(taskId);
          } catch (error) {
            window.alert(error.message || error);
            button.disabled = false;
            button.textContent = '停止';
          }
        });
      });
      root.querySelectorAll('[data-dev-task-action="continue"]').forEach((button) => {
        button.addEventListener('click', () => {
          const taskId = clean(button.dataset.taskId);
          const draft = taskId ? continuationDrafts.get(taskId) : '';
          if (!draft || typeof draftContinuation !== 'function') return;
          draftContinuation(draft);
        });
      });
      root.querySelectorAll('[data-dev-task-action="tool-approval"]').forEach((button) => {
        button.addEventListener('click', async () => {
          const taskId = clean(button.dataset.taskId);
          const approvalId = clean(button.dataset.approvalId);
          const decision = clean(button.dataset.decision);
          if (!taskId || !approvalId || !decision || typeof approveTool !== 'function') return;
          const wrap = button.closest('.dev-task-card-actions');
          if (wrap) {
            wrap.querySelectorAll('button').forEach((item) => { item.disabled = true; });
          } else {
            button.disabled = true;
          }
          try {
            await approveTool(taskId, approvalId, decision);
          } catch (error) {
            window.alert(error.message || error);
            if (wrap) {
              wrap.querySelectorAll('button').forEach((item) => { item.disabled = false; });
            } else {
              button.disabled = false;
            }
          }
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
        actions: true,
        canCancel: !!taskId && !(task && task.result)
      });
    }

    function renderProgress(message, context) {
      const taskId = taskIdOf(message);
      const task = taskId ? context.tasks.get(taskId) : null;
      const toolEvent = parseToolEvent(messageText(message));
      if (toolEvent) return renderToolEvent(message, context, toolEvent);
      return cardHtml({
        tone: 'running',
        eyebrow: '执行进度',
        title: 'Agent 正在处理',
        body: messageText(message),
        taskId,
        meta: '来自运行时',
        actions: true,
        canCancel: !!taskId && !(task && task.result)
      });
    }

    function renderToolEvent(message, context, event) {
      const taskId = taskIdOf(message);
      const task = taskId ? context.tasks.get(taskId) : null;
      if (event.type === 'tool_approval_required') {
        return renderToolApproval(message, context, event);
      }
      if (event.type === 'tool_approval_decision') {
        const decision = clean(event.decision).toLowerCase();
        const denied = decision === 'deny' || clean(event.status).toLowerCase() === 'denied';
        const tool = clean(event.tool) || 'tool';
        return cardHtml({
          tone: denied ? 'canceled' : 'done',
          eyebrow: '工具审批',
          title: denied ? `${tool} 已拒绝` : `${tool} 已批准`,
          body: renderToolBody(event),
          bodyIsHtml: true,
          taskId,
          meta: denied ? '工具不会执行' : '继续执行工具',
          actions: true,
          canCancel: !!taskId && !(task && task.result)
        });
      }
      const isResult = event.type === 'tool_result';
      const failed = isResult && clean(event.status).toLowerCase() === 'error';
      const tool = clean(event.tool) || 'tool';
      return cardHtml({
        tone: failed ? 'failed' : (isResult ? 'done' : 'running'),
        eyebrow: isResult ? '工具结果' : '工具调用',
        title: isResult ? `${tool} 执行结果` : `正在调用 ${tool}`,
        body: renderToolBody(event),
        bodyIsHtml: true,
        taskId,
        meta: isResult ? (failed ? '工具返回错误' : '工具已完成') : '等待工具返回',
        actions: true,
        canCancel: !!taskId && !(task && task.result)
      });
    }

    function renderToolApproval(message, context, event) {
      const taskId = taskIdOf(message);
      const task = taskId ? context.tasks.get(taskId) : null;
      const tool = clean(event.tool) || 'tool';
      const approvalId = clean(event.approval_id);
      return cardHtml({
        tone: 'approval',
        eyebrow: '工具审批',
        title: `确认 ${tool}`,
        body: renderApprovalBody(event),
        bodyIsHtml: true,
        taskId,
        meta: '批准前不会执行',
        actions: true,
        canCancel: !!taskId && !(task && task.result),
        approval: approvalId ? { approvalId } : null
      });
    }

    function renderResult(message, context) {
      const taskId = taskIdOf(message);
      const task = taskId ? context.tasks.get(taskId) : null;
      const content = messageText(message);
      const canceled = /停止|取消|canceled|cancelled/i.test(content);
      const failed = !canceled && /失败|错误|error|failed/i.test(content);
      return cardHtml({
        tone: canceled ? 'canceled' : (failed ? 'failed' : 'done'),
        eyebrow: '执行结果',
        title: canceled ? '任务已停止' : (failed ? '任务失败' : '任务完成'),
        body: renderBody(content, true),
        bodyIsHtml: true,
        taskId,
        meta: canceled ? '已中断运行' : (failed ? '需要继续处理' : '可以检查变更'),
        actions: true,
        continueDraft: continuationDraft(task && task.request, content, canceled, failed)
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
      if (taskId && card.continueDraft) continuationDrafts.set(taskId, card.continueDraft);
      const actions = card.actions
        ? taskActionsHtml(taskId, {
          canCancel: !!card.canCancel,
          canContinue: !!card.continueDraft,
          approval: card.approval
        })
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

    function taskActionsHtml(taskId, options) {
      const approval = options.approval && taskId
        ? approvalButtonsHtml(taskId, options.approval.approvalId)
        : '';
      const stop = options.canCancel && taskId
        ? `<button class="danger" type="button" data-dev-task-action="cancel" data-task-id="${escapeHtml(taskId)}">停止</button>`
        : '';
      const cont = options.canContinue && taskId
        ? `<button class="primary" type="button" data-dev-task-action="continue" data-task-id="${escapeHtml(taskId)}">继续</button>`
        : '';
      return `<div class="dev-task-card-actions">${approval}${cont}${stop}<button type="button" data-dev-task-action="refresh">刷新</button></div>`;
    }

    function approvalButtonsHtml(taskId, approvalId) {
      if (!approvalId) return '';
      return `<button class="primary" type="button" data-dev-task-action="tool-approval" data-decision="approve" data-task-id="${escapeHtml(taskId)}" data-approval-id="${escapeHtml(approvalId)}">批准</button>
        <button class="danger" type="button" data-dev-task-action="tool-approval" data-decision="deny" data-task-id="${escapeHtml(taskId)}" data-approval-id="${escapeHtml(approvalId)}">拒绝</button>`;
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

    function renderToolBody(event) {
      if (event.type === 'tool_approval_decision') {
        return `<div class="dev-tool-body">
          <span>决定</span>
          <pre class="dev-tool-json">${escapeHtml(clean(event.decision) || clean(event.status) || '已处理')}</pre>
        </div>`;
      }
      if (event.type === 'tool_call') {
        return `<div class="dev-tool-body">
          <span>参数</span>
          <pre class="dev-tool-json">${escapeHtml(formatToolValue(event.args || {}))}</pre>
        </div>`;
      }
      return `<div class="dev-tool-body">
        <span>${clean(event.status).toLowerCase() === 'error' ? '错误输出' : '输出'}</span>
        <pre class="dev-tool-json">${escapeHtml(clean(event.result) || '完成')}</pre>
      </div>`;
    }

    function renderApprovalBody(event) {
      const diff = event.diff || {};
      const preview = clean(diff.preview);
      const files = Array.isArray(event.args && event.args.files)
        ? event.args.files.map(clean).filter(Boolean)
        : (Array.isArray(diff.files) ? diff.files.map(clean).filter(Boolean) : []);
      const fileLine = files.length
        ? `<div class="dev-tool-files">${files.map((file) => `<span>${escapeHtml(file)}</span>`).join('')}</div>`
        : '';
      const diffHtml = preview
        ? `<div class="dev-tool-diff"><span>Diff 预览${diff.truncated ? '（已截断）' : ''}</span><pre>${escapeHtml(preview)}</pre></div>`
        : '';
      return `<div class="dev-tool-body">
        <span>待审批参数</span>
        ${fileLine}
        <pre class="dev-tool-json">${escapeHtml(formatToolValue(event.args || {}))}</pre>
        ${diffHtml}
      </div>`;
    }

    function parseToolEvent(content) {
      const text = clean(content);
      if (!text || text[0] !== '{') return null;
      try {
        const event = JSON.parse(text);
        const type = clean(event && event.type);
        if (!['tool_call', 'tool_result', 'tool_approval_required', 'tool_approval_decision'].includes(type)) return null;
        if (!clean(event.tool)) return null;
        return event;
      } catch (_) {
        return null;
      }
    }

    function formatToolValue(value) {
      if (typeof value === 'string') return value;
      try {
        return JSON.stringify(value || {}, null, 2);
      } catch (_) {
        return String(value || '');
      }
    }

    function statusForTask(task) {
      if (task && task.result) {
        if (task.canceled) return { tone: 'canceled', label: '已停止' };
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

    function continuationDraft(request, result, canceled, failed) {
      const original = compactForDraft(request || '', 1200);
      const lastResult = compactForDraft(result || '', 1600);
      const statusLine = canceled
        ? '上次任务被我停止了。'
        : (failed ? '上次任务失败了。' : '上次任务已完成，我要继续迭代。');
      return [
        '继续处理这个 AI 开发任务。',
        statusLine,
        original ? `原始需求：\n${original}` : '',
        lastResult ? `上次结果：\n${lastResult}` : '',
        '请先检查当前项目工作区状态，保护已有改动，再继续完成剩余开发或下一步迭代。'
      ].filter(Boolean).join('\n\n');
    }

    function compactForDraft(value, limit) {
      const text = clean(value)
        .replace(/```[\s\S]*?```/g, '[代码块已省略，请从仓库实际文件检查]')
        .replace(/\n{3,}/g, '\n\n')
        .trim();
      if (text.length <= limit) return text;
      return `${text.slice(0, limit)}\n...（已截断，请结合频道上下文继续）`;
    }

    function shortId(taskId) {
      const value = clean(taskId);
      return value.length > 12 ? `${value.slice(0, 8)}...` : value;
    }

    return { buildContext, renderMessage, bindActions, hasOpenTasks };
  }

  window.ElonPcDevTasks = { create };
})();
