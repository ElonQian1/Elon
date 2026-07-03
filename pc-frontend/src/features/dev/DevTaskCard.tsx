/**
 * DevTaskCard — Claude Desktop 风格的 AI 任务进度展示
 *
 * 设计原则：
 * - tool_call / tool_result → 可折叠内联 chip（默认折叠）
 * - 运行状态 → 细小状态行，不占主要视觉空间
 * - 最终结果 → 主文本，突出展示
 * - 工具审批 → 保留可操作 banner
 */
import { useState } from 'react'
import styles from './DevTaskCard.module.css'
import { clean } from '../../lib/utils'
import {
  taskIsTerminal, statusForTask, parseToolEvent, approvalFinalState,
  approvalStateFor, runtimeStatusLabel, shortId, toolEventSummary, toolEventTitle,
  usageEventSummary,
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
  if (kind === 'ai_task') return <TaskHeader message={message} context={context} onCancel={onCancel} />
  if (kind === 'ai_progress') return <ProgressLine message={message} context={context} onCancel={onCancel} onApprove={onApprove} />
  return <ResultBlock message={message} />
}

/* ══ TaskHeader — 顶部任务标题（一行，简洁） ══ */
function TaskHeader({ message, context, onCancel }: Omit<DevTaskMessageProps, 'onApprove'>) {
  const taskId = clean(message.task_id ?? message.taskId ?? '')
  const task = taskId ? context.tasks.get(taskId) ?? null : null
  const status = statusForTask(task)
  const request = clean(message.content ?? message.text ?? '').replace(/^发起\s*AI\s*开发任务[：:]\s*/i, '')
  const canCancel = !!taskId && !taskIsTerminal(task)
  return (
    <div className={[styles.taskHeader, styles[`h_${status.tone}`]].join(' ')}>
      <span className={styles.taskIcon}>{statusIcon(status.tone)}</span>
      <span className={styles.taskLabel}>{status.label}</span>
      {request && <span className={styles.taskRequest}>{request}</span>}
      <div className={styles.taskMeta}>
        {taskId && <span className={styles.taskId}>{shortId(taskId)}</span>}
        {task?.progressCount ? <span>{task.progressCount} 步</span> : null}
      </div>
      {canCancel && onCancel && (
        <button className={styles.cancelSmall} onClick={() => { if (window.confirm('停止任务？')) onCancel(taskId) }}>停止</button>
      )}
    </div>
  )
}

/* ══ ProgressLine — 进度行（工具调用/状态） ══ */
function ProgressLine({ message, context, onCancel, onApprove }: DevTaskMessageProps) {
  const taskId = clean(message.task_id ?? message.taskId ?? '')
  const task = taskId ? context.tasks.get(taskId) ?? null : null
  const canCancel = !!taskId && !taskIsTerminal(task)
  const content = clean(message.content ?? message.text ?? '')
  const event = parseToolEvent(content)

  if (!event) {
    return <StatusLine text={content} />
  }
  if (event.type === 'runtime_status') {
    const label = runtimeStatusLabel(clean(event.phase ?? '').toLowerCase())
    const text = clean(event.message ?? '') || label.body
    return <StatusLine text={text} tone={label.tone} runtime={clean(event.runtime ?? '')} turn={Number(event.turn)} />
  }
  if (event.type === 'runtime_summary') {
    const total = Number(event.total_tools ?? 0); const failed = Number(event.failed_tools ?? 0)
    const status = clean(event.status ?? '').toLowerCase()
    const canceled = ['canceled', 'cancelled', 'stopped'].includes(status)
    const ok = failed === 0 && !canceled && !['error', 'failed'].includes(status)
    const tone: TaskTone = canceled ? 'canceled' : (ok ? 'done' : 'failed')
    return <StatusLine text={clean(event.message ?? '') || `${total} 步完成，${failed} 步失败`} tone={tone} runtime={clean(event.runtime ?? '')} turn={Number(event.turn)} />
  }
  if (event.type === 'pc_dispatch_started') {
    const cli = clean(event.cli ?? 'AI')
    const agent = shortNode(clean(event.agent_id ?? ''))
    return <StatusLine text={`已派发到 PC 节点，等待 ${cli} CLI 输出`} tone="running" runtime={agent} />
  }
  if (event.type === 'assistant_message' || event.type === 'assistant_chunk') {
    return null
  }
  if (event.type === 'usage') {
    return <StatusLine text={usageEventSummary(event)} tone="done" runtime={clean(event.model ?? '')} />
  }
  if (event.type === 'tool_approval_required') {
    const approvalId = clean(event.approval_id ?? '')
    const savedState = approvalId ? approvalStateFor(context, taskId, approvalId) : null
    const closedState = savedState && savedState.status !== 'pending' ? savedState : null
    const tool = clean(event.tool ?? 'tool')
    return <ApprovalBanner tool={tool} event={event} taskId={taskId} closedState={closedState} canCancel={canCancel} onCancel={onCancel} approvalId={!closedState && approvalId ? approvalId : undefined} onApprove={onApprove} />
  }
  if (event.type === 'tool_approval_decision') {
    const finalState = approvalFinalState(event)
    return <StatusLine text={`${clean(event.tool ?? 'tool')} ${finalState.label}`} tone={finalState.tone} />
  }
  return <ToolChip event={event} />
}

/* ══ ToolChip — Claude 风格可折叠工具调用 ══ */
function ToolChip({ event }: { event: ToolEvent }) {
  const [open, setOpen] = useState(false)
  const isResult = event.type === 'tool_result'
  const failed = isResult && clean(event.status ?? '').toLowerCase() === 'error'
  const tool = clean(event.tool ?? 'tool')
  const title = toolEventTitle(event)
  const summary = toolEventSummary(event, 96)
  return (
    <div className={[styles.toolChip, failed ? styles.toolFailed : isResult ? styles.toolDone : styles.toolRunning].join(' ')}>
      <button className={styles.toolChipBtn} onClick={() => setOpen(!open)} type="button">
        <span className={styles.toolChipArrow}>{open ? '▾' : '▸'}</span>
        <span className={styles.toolChipIcon}>{isResult ? (failed ? '✗' : '✓') : '⟳'}</span>
        <span className={styles.toolChipName}>{title}</span>
        {!open && summary && <span className={styles.toolChipSummary}>{summary}</span>}
      </button>
      {open && (
        <div className={styles.toolChipBody}>
          <div className={styles.toolChipMeta}>{tool}</div>
          {!isResult && event.args && (
            <pre className={styles.toolChipPre}>{formatToolArgs(event.args)}</pre>
          )}
          {isResult && (
            <pre className={[styles.toolChipPre, failed ? styles.toolChipPreErr : ''].join(' ')}>
              {clean(event.result ?? '') || '（无输出）'}
            </pre>
          )}
        </div>
      )}
    </div>
  )
}

/* ══ StatusLine — 极简状态行 ══ */
function StatusLine({ text, tone, runtime, turn }: {
  text: string; tone?: TaskTone; runtime?: string; turn?: number
}) {
  const dot = tone === 'done' ? styles.dotDone : tone === 'failed' ? styles.dotFail : tone === 'canceled' ? styles.dotCancel : styles.dotRun
  return (
    <div className={styles.statusLine}>
      <span className={[styles.dot, dot].join(' ')} />
      <span className={styles.statusText}>{text}</span>
      {(runtime || (turn && turn > 0)) && (
        <span className={styles.statusMeta}>
          {runtime || ''}{turn && turn > 0 ? ` · 第${turn}轮` : ''}
        </span>
      )}
    </div>
  )
}

/* ══ ApprovalBanner — 审批横幅 ══ */
function ApprovalBanner({ tool, event, taskId, closedState, canCancel, onCancel, approvalId, onApprove }: {
  tool: string; event: ToolEvent; taskId: string
  closedState: { tone: TaskTone; label: string; meta: string } | null
  canCancel: boolean; onCancel?: (id: string) => void
  approvalId?: string; onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}) {
  const [open, setOpen] = useState(false)
  const tone = closedState?.tone ?? 'approval'
  return (
    <div className={[styles.approvalBanner, styles[`tone_${tone}`]].join(' ')}>
      <div className={styles.approvalHead}>
        <span>🔐</span>
        <strong>{closedState ? `${tool} ${closedState.label}` : `确认执行 ${tool}？`}</strong>
        <button className={styles.approvalToggle} onClick={() => setOpen(!open)} type="button">{open ? '隐藏详情' : '查看详情'}</button>
        {!closedState && approvalId && onApprove && (
          <>
            <button className={styles.approveBtn} onClick={() => onApprove(taskId, approvalId, 'approve')}>批准</button>
            <button className={styles.denyBtn} onClick={() => onApprove(taskId, approvalId, 'deny')}>拒绝</button>
          </>
        )}
        {canCancel && onCancel && !closedState && (
          <button className={styles.cancelSmall} onClick={() => { if (window.confirm('停止任务？')) onCancel(taskId) }}>停止</button>
        )}
      </div>
      {open && <pre className={styles.toolChipPre}>{formatApprovalBody(event)}</pre>}
    </div>
  )
}

/* ══ ResultBlock — 最终结果（突出展示） ══ */
function ResultBlock({ message }: { message: ChatMessage }) {
  const content = clean(message.content ?? message.text ?? '')
  const canceled = /停止|取消|canceled|cancelled/i.test(content)
  const failed = !canceled && /失败|错误|error|failed/i.test(content)
  const tone: TaskTone = canceled ? 'canceled' : (failed ? 'failed' : 'done')
  return (
    <div className={[styles.resultBlock, styles[`r_${tone}`]].join(' ')}>
      <span className={styles.resultIcon}>{tone === 'done' ? '✓' : tone === 'canceled' ? '◉' : '✗'}</span>
      <div className={styles.resultContent}>
        <span className={styles.resultLabel}>{tone === 'done' ? '任务完成' : tone === 'canceled' ? '任务已停止' : '任务失败'}</span>
        {content && <p className={styles.resultText}>{content}</p>}
      </div>
    </div>
  )
}

function statusIcon(tone: TaskTone): string {
  if (tone === 'done') return '✓'
  if (tone === 'failed') return '✗'
  if (tone === 'canceled') return '◉'
  if (tone === 'approval') return '🔐'
  return '⟳'
}

function formatToolArgs(args: Record<string, unknown>): string {
  const command = clean(args.command)
  if (command) return command
  return JSON.stringify(args, null, 2)
}

function shortNode(value: string): string {
  const v = clean(value)
  return v.length > 18 ? `${v.slice(0, 11)}...${v.slice(-6)}` : v
}

function formatApprovalBody(event: ToolEvent): string {
  const args = event.args ? JSON.stringify(event.args, null, 2) : ''
  const diff = event.diff?.preview ? `\n\nDiff 预览:\n${event.diff.preview}` : ''
  return `待审批参数:\n${args}${diff}`
}
