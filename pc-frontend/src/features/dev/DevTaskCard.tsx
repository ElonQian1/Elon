import styles from './DevTaskCard.module.css'
import { clean } from '../../lib/utils'
import {
  taskIsTerminal, statusForTask, parseToolEvent, approvalFinalState,
  approvalStateFor, runtimeStatusLabel, shortId,
} from './devTaskUtils'
import type { ChatMessage, TaskContext, ToolEvent, TaskTone } from './types'

interface DevTaskMessageProps {
  message: ChatMessage
  context: TaskContext
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

export function DevTaskMessage({ message, context, onCancel, onApprove }: DevTaskMessageProps) {
  const kind = clean(message.kind ?? message.role ?? message.message_kind ?? '').toLowerCase()
  if (!['ai_task', 'ai_progress', 'ai_result'].includes(kind)) return null
  if (kind === 'ai_task') return <TaskStartCard message={message} context={context} onCancel={onCancel} />
  if (kind === 'ai_progress') return <ProgressCard message={message} context={context} onCancel={onCancel} onApprove={onApprove} />
  return <ResultCard message={message} context={context} />
}

/* ── Task Start Card ── */
function TaskStartCard({ message, context, onCancel }: Omit<DevTaskMessageProps, 'onApprove'>) {
  const taskId = clean(message.task_id ?? message.taskId ?? '')
  const task = taskId ? context.tasks.get(taskId) ?? null : null
  const status = statusForTask(task)
  const request = clean(message.content ?? message.text ?? '').replace(/^发起\s*AI\s*开发任务[:：]\s*/i, '')
  const canCancel = !!taskId && !taskIsTerminal(task)
  return (
    <DevTaskCard
      tone={status.tone} eyebrow="AI 开发任务" title={status.label}
      body={request || '已提交开发任务。'} taskId={taskId}
      meta={task?.progressCount ? `${task.progressCount} 条进度` : '等待执行'}
      canCancel={canCancel} onCancel={onCancel}
    />
  )
}

/* ── Progress Card ── */
function ProgressCard({ message, context, onCancel, onApprove }: DevTaskMessageProps) {
  const taskId = clean(message.task_id ?? message.taskId ?? '')
  const task = taskId ? context.tasks.get(taskId) ?? null : null
  const canCancel = !!taskId && !taskIsTerminal(task)
  const content = clean(message.content ?? message.text ?? '')
  const event = parseToolEvent(content)
  if (!event) {
    return (
      <DevTaskCard
        tone="running" eyebrow="执行进度" title="Agent 正在处理"
        body={content} taskId={taskId} meta="来自运行时" canCancel={canCancel} onCancel={onCancel}
      />
    )
  }
  return <ToolEventCard event={event} taskId={taskId} context={context} canCancel={canCancel} onCancel={onCancel} onApprove={onApprove} />
}

/* ── Tool Event Card ── */
function ToolEventCard({ event, taskId, context, canCancel, onCancel, onApprove }: {
  event: ToolEvent; taskId: string; context: TaskContext; canCancel: boolean
  onCancel?: DevTaskMessageProps['onCancel']; onApprove?: DevTaskMessageProps['onApprove']
}) {
  if (event.type === 'runtime_status') {
    const label = runtimeStatusLabel(clean(event.phase ?? '').toLowerCase())
    return (
      <DevTaskCard
        tone={label.tone} eyebrow="运行阶段" title={label.title}
        body={clean(event.message ?? '') || label.body} taskId={taskId}
        meta={`${clean(event.runtime ?? 'runtime')} · ${Number(event.turn) > 0 ? `第 ${event.turn} 轮` : '运行阶段'}`}
        canCancel={canCancel} onCancel={onCancel}
      />
    )
  }
  if (event.type === 'runtime_summary') {
    const total = Number(event.total_tools ?? 0); const failed = Number(event.failed_tools ?? 0)
    const status = clean(event.status ?? '').toLowerCase()
    const canceled = ['canceled', 'cancelled', 'stopped'].includes(status)
    const ok = failed === 0 && !canceled && !['error', 'failed'].includes(status)
    return (
      <DevTaskCard
        tone={canceled ? 'canceled' : (ok ? 'done' : 'failed')}
        eyebrow="执行摘要" title={canceled ? 'Runtime 已停止' : (ok ? 'Runtime 已完成' : 'Runtime 有失败工具')}
        body={[clean(event.message ?? '') || (canceled ? '运行已停止。' : (ok ? '运行已完成。' : '运行结束。')), `工具调用 ${total} 个，失败 ${failed} 个。`].join('\n')}
        taskId={taskId} meta={`${clean(event.runtime ?? 'runtime')} · ${Number(event.turn) > 0 ? `第 ${event.turn} 轮` : '运行结束'}`}
        canCancel={canCancel} onCancel={onCancel}
      />
    )
  }
  if (event.type === 'tool_approval_required') {
    const approvalId = clean(event.approval_id ?? '')
    const savedState = approvalId ? approvalStateFor(context, taskId, approvalId) : null
    const closedState = savedState && savedState.status !== 'pending' ? savedState : null
    const tool = clean(event.tool ?? 'tool')
    return (
      <DevTaskCard
        tone={closedState?.tone ?? 'approval'}
        eyebrow="工具审批"
        title={closedState ? `${tool} ${closedState.label}` : `确认 ${tool}`}
        body={formatApprovalBody(event)} bodyIsHtml={false}
        taskId={taskId} meta={closedState?.meta ?? '批准前不会执行'}
        canCancel={canCancel} onCancel={onCancel}
        approvalId={!closedState && approvalId ? approvalId : undefined}
        onApprove={onApprove}
      />
    )
  }
  if (event.type === 'tool_approval_decision') {
    const finalState = approvalFinalState(event)
    return (
      <DevTaskCard
        tone={finalState.tone} eyebrow="工具审批" title={`${clean(event.tool ?? 'tool')} ${finalState.label}`}
        body={`决定: ${clean(event.decision ?? event.status ?? '已处理')}`}
        taskId={taskId} meta={finalState.meta} canCancel={canCancel} onCancel={onCancel}
      />
    )
  }
  const isResult = event.type === 'tool_result'
  const failed = isResult && clean(event.status ?? '').toLowerCase() === 'error'
  const tool = clean(event.tool ?? 'tool')
  return (
    <DevTaskCard
      tone={failed ? 'failed' : (isResult ? 'done' : 'running')}
      eyebrow={isResult ? '工具结果' : '工具调用'}
      title={isResult ? `${tool} 执行结果` : `正在调用 ${tool}`}
      body={isResult ? (clean(event.result ?? '') || '完成') : JSON.stringify(event.args ?? {}, null, 2)}
      taskId={taskId} meta={isResult ? (failed ? '工具返回错误' : '工具已完成') : '等待工具返回'}
      canCancel={canCancel} onCancel={onCancel}
    />
  )
}

/* ── Result Card ── */
function ResultCard({ message, context: _context }: Omit<DevTaskMessageProps, 'onCancel' | 'onApprove'>) {
  const taskId = clean(message.task_id ?? message.taskId ?? '')
  const content = clean(message.content ?? message.text ?? '')
  const canceled = /停止|取消|canceled|cancelled/i.test(content)
  const failed = !canceled && /失败|错误|error|failed/i.test(content)
  return (
    <DevTaskCard
      tone={canceled ? 'canceled' : (failed ? 'failed' : 'done')}
      eyebrow="执行结果"
      title={canceled ? '任务已停止' : (failed ? '任务失败' : '任务完成')}
      body={content} taskId={taskId}
      meta={canceled ? '已中断运行' : (failed ? '需要继续处理' : '可以检查变更')}
    />
  )
}

/* ── Base Card Component ── */
interface DevTaskCardProps {
  tone: TaskTone; eyebrow: string; title: string; body: string; bodyIsHtml?: boolean
  taskId: string; meta: string; canCancel?: boolean; onCancel?: (id: string) => void
  approvalId?: string; onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

function DevTaskCard({ tone, eyebrow, title, body, bodyIsHtml, taskId, meta, canCancel, onCancel, approvalId, onApprove }: DevTaskCardProps) {
  return (
    <div className={styles.wrap}>
      <section className={[styles.card, styles[tone]].join(' ')}>
        <div className={styles.head}>
          <span>{eyebrow}</span>
          <strong>{title}</strong>
        </div>
        {bodyIsHtml
          ? <div className={styles.htmlBody} dangerouslySetInnerHTML={{ __html: body }} />
          : <pre className={styles.body}>{body}</pre>}
        <div className={styles.foot}>
          <div>
            {taskId && <span title={taskId}>任务 {shortId(taskId)}</span>}
            <span>{meta}</span>
          </div>
          <div className={styles.actions}>
            {approvalId && onApprove && (
              <>
                <button className={styles.approveBtn} onClick={() => onApprove(taskId, approvalId, 'approve')}>批准</button>
                <button className={styles.denyBtn} onClick={() => onApprove(taskId, approvalId, 'deny')}>拒绝</button>
              </>
            )}
            {canCancel && onCancel && (
              <button className={styles.cancelBtn} onClick={() => { if (window.confirm('停止这个 AI 开发任务？')) onCancel(taskId) }}>停止</button>
            )}
          </div>
        </div>
      </section>
    </div>
  )
}

function formatApprovalBody(event: ToolEvent): string {
  const args = event.args ? JSON.stringify(event.args, null, 2) : ''
  const diff = event.diff?.preview ? `\n\nDiff 预览:\n${event.diff.preview}` : ''
  return `待审批参数:\n${args}${diff}`
}
