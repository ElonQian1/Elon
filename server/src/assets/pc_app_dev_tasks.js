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

    function buildContext(messages, extras) {
      continuationDrafts.clear();
      const tasks = new Map();
      const approvals = new Map();
      (messages || []).forEach((message) => {
        const kind = messageKind(message);
        const taskId = taskIdOf(message);
        if (!taskId) return;
        if (!tasks.has(taskId)) {
          tasks.set(taskId, emptyTask(taskId));
        }
        const task = tasks.get(taskId);
        const status = taskStatusOf(message);
        if (status) task.status = status;
        const taskError = taskErrorOf(message);
        if (taskError) task.error = taskError;
        const apkUrl = taskApkUrlOf(message);
        if (apkUrl) task.apkUrl = apkUrl;
        if (kind === 'ai_task') task.request = taskRequest(messageText(message)) || task.request;
        if (kind === 'ai_progress') {
          task.progressCount += 1;
          rememberApprovalState(approvals, taskId, parseToolEvent(messageText(message)));
        }
        if (kind === 'ai_result') {
          const content = messageText(message);
          task.result = message;
          task.resultText = content;
          task.canceled = /停止|取消|canceled|cancelled/i.test(content);
          task.failed = !task.canceled && /失败|错误|error|failed/i.test(content);
        }
      });
      mergeSnapshotState(tasks, extras && extras.snapshots);
      return { tasks, approvals };
    }

    function emptyTask(taskId) {
      return {
        taskId,
        progressCount: 0,
        result: null,
        request: '',
        resultText: '',
        failed: false,
        canceled: false,
        status: '',
        error: '',
        apkUrl: '',
        attach: null,
        resume: null,
        lastEventSeq: 0
      };
    }

    function mergeSnapshotState(tasks, snapshots) {
      if (!snapshots || typeof snapshots.forEach !== 'function') return;
      snapshots.forEach((snapshot, snapshotTaskId) => {
        const taskId = clean(snapshotTaskId || (snapshot && snapshot.task && snapshot.task.id));
        if (!taskId) return;
        if (!tasks.has(taskId)) tasks.set(taskId, emptyTask(taskId));
        const task = tasks.get(taskId);
        const snapshotTask = snapshot && snapshot.task ? snapshot.task : {};
        const status = clean(snapshotTask.status).toLowerCase();
        if (status) task.status = status;
        const error = clean(snapshotTask.error);
        if (error) task.error = error;
        const apkUrl = clean(snapshotTask.apk_url || snapshotTask.apkUrl);
        if (apkUrl) task.apkUrl = apkUrl;
        const attach = snapshot && snapshot.attach ? snapshot.attach : null;
        if (attach) task.attach = attach;
        const resume = snapshot && snapshot.resume ? snapshot.resume : null;
        if (resume) task.resume = resume;
        const seq = Number(snapshot && (snapshot.last_event_seq || snapshot.lastEventSeq || 0));
        if (Number.isFinite(seq) && seq > 0) task.lastEventSeq = seq;
      });
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
      return Array.from(built.tasks.values()).some((task) => !taskIsTerminal(task) && !taskNeedsSnapshotContinue(task));
    }

    function openTaskIds(messages, context) {
      if (!Array.isArray(messages) || !messages.length) return [];
      const built = context || buildContext(messages);
      return Array.from(built.tasks.values())
        .filter((task) => !taskIsTerminal(task) && !taskNeedsSnapshotContinue(task))
        .map((task) => task.taskId)
        .filter(Boolean);
    }

    function renderTaskStart(message, context) {
      const taskId = taskIdOf(message);
      const task = taskId ? context.tasks.get(taskId) : null;
      const status = statusForTask(task);
      const request = taskRequest(messageText(message));
      const snapshotContinue = taskNeedsSnapshotContinue(task);
      return cardHtml({
        tone: status.tone,
        eyebrow: 'AI 开发任务',
        title: status.label,
        body: request || '已提交开发任务。',
        taskId,
        meta: attachMeta(task, task && task.progressCount ? `${task.progressCount} 条进度` : '等待执行回写'),
        actions: true,
        canCancel: !!taskId && !taskIsTerminal(task) && !snapshotContinue,
        continueDraft: (taskIsTerminal(task) || snapshotContinue) && !(task && task.result)
          ? continuationDraft(request, task && (task.error || task.status), taskIsCanceled(task), taskIsFailed(task), task && task.resume)
          : null
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
        canCancel: !!taskId && !taskIsTerminal(task) && !taskNeedsSnapshotContinue(task)
      });
    }

    function renderToolEvent(message, context, event) {
      const taskId = taskIdOf(message);
      const task = taskId ? context.tasks.get(taskId) : null;
      if (event.type === 'tool_approval_required') {
        return renderToolApproval(message, context, event);
      }
      if (event.type === 'tool_approval_decision') {
        const finalState = approvalFinalState(event);
        const tool = clean(event.tool) || 'tool';
        return cardHtml({
          tone: finalState.tone,
          eyebrow: '工具审批',
          title: `${tool} ${finalState.label}`,
          body: renderToolBody(event),
          bodyIsHtml: true,
          taskId,
          meta: finalState.meta,
          actions: true,
          canCancel: !!taskId && !taskIsTerminal(task) && !taskNeedsSnapshotContinue(task)
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
        canCancel: !!taskId && !taskIsTerminal(task) && !taskNeedsSnapshotContinue(task)
      });
    }

    function renderToolApproval(message, context, event) {
      const taskId = taskIdOf(message);
      const task = taskId ? context.tasks.get(taskId) : null;
      const tool = clean(event.tool) || 'tool';
      const approvalId = clean(event.approval_id);
      const recoveredState = approvalStateFor(context, taskId, approvalId);
      const snapshotContinue = taskNeedsSnapshotContinue(task);
      const closedState = recoveredState && recoveredState.status !== 'pending'
        ? recoveredState
        : (snapshotContinue ? taskSnapshotContinueApprovalState() : (taskIsTerminal(task) ? taskTerminalApprovalState(task) : null));
      return cardHtml({
        tone: closedState ? closedState.tone : 'approval',
        eyebrow: '工具审批',
        title: closedState ? `${tool} ${closedState.label}` : `确认 ${tool}`,
        body: renderApprovalBody(event),
        bodyIsHtml: true,
        taskId,
        meta: closedState ? closedState.meta : '批准前不会执行',
        actions: true,
        canCancel: !!taskId && !taskIsTerminal(task) && !taskNeedsSnapshotContinue(task),
        approval: approvalId && !closedState ? { approvalId } : null
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
        continueDraft: continuationDraft(task && task.request, content, canceled, failed, task && task.resume)
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

    function rememberApprovalState(approvals, taskId, event) {
      if (!approvals || !event || !taskId) return;
      const approvalId = clean(event.approval_id);
      if (!approvalId) return;
      const key = approvalKey(taskId, approvalId);
      if (event.type === 'tool_approval_required') {
        if (!approvals.has(key)) {
          approvals.set(key, { status: 'pending', tool: clean(event.tool), tone: 'approval', label: '等待确认', meta: '批准前不会执行' });
        }
        return;
      }
      if (event.type === 'tool_approval_decision') {
        approvals.set(key, approvalFinalState(event));
      }
    }

    function approvalStateFor(context, taskId, approvalId) {
      if (!context || !context.approvals || !taskId || !approvalId) return null;
      return context.approvals.get(approvalKey(taskId, approvalId)) || null;
    }

    function approvalKey(taskId, approvalId) {
      return `${clean(taskId)}:${clean(approvalId)}`;
    }

    function approvalFinalState(event) {
      const decision = clean(event && event.decision).toLowerCase();
      const status = clean(event && event.status).toLowerCase();
      if (decision === 'approve' || status === 'approved') {
        return { status: 'approved', tone: 'done', label: '已批准', meta: '继续执行工具' };
      }
      if (['deny', 'denied', 'reject', 'rejected'].includes(decision) || status === 'denied') {
        return { status: 'denied', tone: 'canceled', label: '已拒绝', meta: '工具不会执行' };
      }
      if (decision === 'timeout' || status === 'expired') {
        return { status: 'expired', tone: 'canceled', label: '已过期', meta: '审批已过期' };
      }
      if (['cancel', 'canceled', 'cancelled'].includes(decision) || ['canceled', 'cancelled'].includes(status)) {
        return { status: 'canceled', tone: 'canceled', label: '已取消', meta: '任务已停止' };
      }
      return { status: 'processed', tone: 'done', label: '已处理', meta: '审批已处理' };
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
      if (taskIsTerminal(task)) {
        if (taskIsCanceled(task)) return { tone: 'canceled', label: taskStatusOfValue(task) === 'interrupted' ? '已中断' : '已停止' };
        return taskIsFailed(task)
          ? { tone: 'failed', label: '任务失败' }
          : { tone: 'done', label: '任务完成' };
      }
      if (taskNeedsSnapshotContinue(task)) return { tone: 'failed', label: '需要基于快照继续' };
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

    function taskStatusOf(message) {
      return clean(message.task_status || message.taskStatus).toLowerCase();
    }

    function taskErrorOf(message) {
      return clean(message.task_error || message.taskError);
    }

    function taskApkUrlOf(message) {
      return clean(message.task_apk_url || message.taskApkUrl);
    }

    function taskStatusOfValue(task) {
      return clean(task && task.status).toLowerCase();
    }

    function taskIsTerminal(task) {
      if (!task) return false;
      if (task.result) return true;
      return ['done', 'failed', 'canceled', 'cancelled', 'interrupted'].includes(taskStatusOfValue(task));
    }

    function taskIsCanceled(task) {
      const status = taskStatusOfValue(task);
      return !!(task && task.canceled) || ['canceled', 'cancelled', 'interrupted'].includes(status);
    }

    function taskIsFailed(task) {
      const status = taskStatusOfValue(task);
      return !!(task && task.failed) || ['failed', 'interrupted'].includes(status);
    }

    function taskTerminalApprovalState(task) {
      if (taskIsCanceled(task)) {
        return { status: 'canceled', tone: 'canceled', label: '已失效', meta: '任务已结束' };
      }
      if (taskIsFailed(task)) {
        return { status: 'failed', tone: 'failed', label: '已失效', meta: '任务失败，审批不会继续执行' };
      }
      return { status: 'done', tone: 'done', label: '已失效', meta: '任务已结束' };
    }

    function taskSnapshotContinueApprovalState() {
      return { status: 'detached', tone: 'failed', label: '已失效', meta: '现场已脱离，请基于快照继续' };
    }

    function attachMeta(task, fallback) {
      const attach = task && task.attach ? task.attach : null;
      const status = clean(attach && attach.status).toLowerCase();
      const local = clean(attach && attach.source).toLowerCase() === 'local_journal';
      const hint = resumeHint(task);
      if (status === 'live') return `${local ? '本机现场可连接' : '现场可连接'}${hint} · ${fallback}`;
      if (status === 'detached') return `${local ? '本机现场已脱离' : '现场已脱离'}${hint} · ${fallback}`;
      if (status === 'terminal') return `${local ? '本机终态快照' : '终态快照'}${hint} · ${fallback}`;
      return fallback;
    }

    function taskRequest(content) {
      return clean(content).replace(/^发起\s*AI\s*开发任务[:：]\s*/i, '');
    }

    function continuationDraft(request, result, canceled, failed, resume) {
      const original = compactForDraft(request || '', 1200);
      const lastResult = compactForDraft(result || '', 1600);
      const resumeLine = resumeDraftLine(resume);
      const statusLine = canceled
        ? '上次任务被我停止了。'
        : (failed ? '上次任务失败了。' : '上次任务已完成，我要继续迭代。');
      return [
        '继续处理这个 AI 开发任务。',
        statusLine,
        resumeLine ? `恢复方式：${resumeLine}` : '',
        original ? `原始需求：\n${original}` : '',
        lastResult ? `上次结果：\n${lastResult}` : '',
        '请先检查当前项目工作区状态，保护已有改动，再继续完成剩余开发或下一步迭代。'
      ].filter(Boolean).join('\n\n');
    }

    function taskNeedsSnapshotContinue(task) {
      const resume = task && task.resume ? task.resume : null;
      const action = clean(resume && resume.next_action).toLowerCase();
      return action === 'continue_from_snapshot';
    }

    function resumeHint(task) {
      const resume = task && task.resume ? task.resume : null;
      const action = clean(resume && resume.next_action).toLowerCase();
      const canStream = resume && resume.can_stream_live_output !== false;
      const canReplay = resume && resume.can_replay_journal_events === true;
      if (canReplay) return '（本机事件可回放）';
      if (action === 'wait_or_cancel' && !canStream) return '（暂不回放输出）';
      if (action === 'continue_from_snapshot') return '（基于快照继续）';
      if (action === 'refresh_snapshot') return '（仅云端快照）';
      return '';
    }

    function resumeDraftLine(resume) {
      if (!resume) return '';
      const label = clean(resume.strategy && resume.strategy.label);
      const reason = clean(resume.reason || (resume.strategy && resume.strategy.reason));
      return [label, reason].filter(Boolean).join('：');
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

    return { buildContext, renderMessage, bindActions, hasOpenTasks, openTaskIds };
  }

  window.ElonPcDevTasks = { create };
})();
