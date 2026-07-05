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
    const agentId = clean(event.agent_id ?? '')
    const nodeDisplayName = clean(event.node_display_name ?? '')
    const agent = nodeDisplayName || shortNode(agentId)
    return <StatusLine text={`已派发到 PC 节点，等待 ${cli} CLI 输出`} tone="running" runtime={agent} runtimeTitle={agentId} />
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
function StatusLine({ text, tone, runtime, runtimeTitle, turn }: {
  text: string; tone?: TaskTone; runtime?: string; runtimeTitle?: string; turn?: number
}) {
  const dot = tone === 'done' ? styles.dotDone : tone === 'failed' ? styles.dotFail : tone === 'canceled' ? styles.dotCancel : styles.dotRun
  return (
    <div className={styles.statusLine}>
      <span className={[styles.dot, dot].join(' ')} />
      <span className={styles.statusText}>{text}</span>
      {(runtime || (turn && turn > 0)) && (
        <span className={styles.statusMeta} title={runtimeTitle || runtime || undefined}>
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
  const insight = approvalInsight(event, tool)
  const active = !closedState && Boolean(approvalId && onApprove)
  return (
    <div className={[styles.approvalBanner, styles[`tone_${tone}`]].join(' ')}>
      <div className={styles.approvalHead}>
        <span className={styles.approvalIcon}>🔐</span>
        <div className={styles.approvalTitleBlock}>
          <strong>{closedState ? `${insight.title} · ${closedState.label}` : insight.title}</strong>
          <span>{closedState?.meta ?? insight.summary}</span>
        </div>
        <span className={[styles.approvalRisk, styles[`risk_${insight.riskTone}`]].join(' ')}>{closedState ? closedState.label : insight.riskLabel}</span>
      </div>
      <div className={styles.approvalScope} aria-label="审批影响范围">
        {insight.command && (
          <div className={styles.scopeRow}>
            <span>命令</span>
            <code>{insight.command}</code>
          </div>
        )}
        {insight.files.length > 0 && (
          <div className={styles.scopeRow}>
            <span>文件</span>
            <div className={styles.fileList}>
              {insight.files.slice(0, 4).map((file) => <code key={file}>{file}</code>)}
              {insight.files.length > 4 && <em>另 {insight.files.length - 4} 个</em>}
            </div>
          </div>
        )}
        <div className={styles.scopeRow}>
          <span>工作区</span>
          <code>{insight.workspace || '当前项目工作区'}</code>
        </div>
        <div className={styles.scopeRow}>
          <span>后果</span>
          <p>{closedState ? closedApprovalNotice(closedState) : insight.consequence}</p>
        </div>
      </div>
      <div className={styles.approvalActions}>
        <button className={styles.approvalToggle} onClick={() => setOpen(!open)} type="button">
          {open ? '隐藏 diff / 参数' : insight.detailLabel}
        </button>
        {active && approvalId && onApprove && (
          <>
            <button className={styles.approveBtn} onClick={() => onApprove(taskId, approvalId, 'approve')}>
              {insight.approveLabel}
            </button>
            <button className={styles.denyBtn} onClick={() => onApprove(taskId, approvalId, 'deny')}>
              拒绝执行
            </button>
          </>
        )}
        {!active && (
          <button className={styles.disabledApprovalBtn} type="button" disabled>
            审批不可操作
          </button>
        )}
        {canCancel && onCancel && !closedState && (
          <button className={styles.cancelSmall} onClick={() => { if (window.confirm('停止当前 AI 任务？停止后不会批准这个工具动作。')) onCancel(taskId) }}>
            停止任务
          </button>
        )}
      </div>
      {open && (
        <div className={styles.approvalDetails}>
          {insight.diffPreview && (
            <section>
              <strong>Diff 预览</strong>
              <pre className={styles.toolChipPre}>{insight.diffPreview}</pre>
            </section>
          )}
          <section>
            <strong>{insight.argsLabel}</strong>
            <pre className={styles.toolChipPre}>{insight.argsPreview || '（没有参数）'}</pre>
          </section>
        </div>
      )}
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

interface ApprovalInsight {
  title: string
  summary: string
  riskLabel: string
  riskTone: 'normal' | 'danger'
  command: string
  workspace: string
  files: string[]
  consequence: string
  detailLabel: string
  approveLabel: string
  argsLabel: string
  argsPreview: string
  diffPreview: string
}

function approvalInsight(event: ToolEvent, tool: string): ApprovalInsight {
  const command = approvalCommand(event)
  const files = approvalFiles(event)
  const workspace = clean(event.args?.cwd ?? event.args?.workdir ?? event.args?.workspace ?? event.args?.project_root ?? event.cwd)
  const diffPreview = clean(event.diff?.preview ?? '')
  const argsPreview = formatApprovalArgs(event)
  const highRisk = approvalLooksHighRisk(tool, command)

  if (tool === 'shell') {
    return {
      title: '请求执行命令',
      summary: command ? 'AI CLI 需要在本机工作区运行这条命令。' : 'AI CLI 请求执行一条本机命令。',
      riskLabel: highRisk ? '高风险命令' : '命令审批',
      riskTone: highRisk ? 'danger' : 'normal',
      command,
      workspace,
      files,
      consequence: highRisk ? '批准后可能修改文件、安装依赖、推送代码或影响本机环境。' : '批准后命令会继续执行；拒绝后本轮工具动作不会运行。',
      detailLabel: '查看命令 / 参数',
      approveLabel: '批准执行命令',
      argsLabel: '命令参数',
      argsPreview,
      diffPreview,
    }
  }

  if (tool === 'file_change') {
    return {
      title: '请求修改文件',
      summary: files.length ? `将影响 ${files.length} 个文件，批准前可查看 diff。` : 'AI CLI 请求写入或修改项目文件。',
      riskLabel: '写文件审批',
      riskTone: 'normal',
      command,
      workspace,
      files,
      consequence: '批准后文件会被写入当前工作区；拒绝后不会应用这次文件修改。',
      detailLabel: diffPreview ? '查看 diff / 参数' : '查看文件参数',
      approveLabel: '批准写入文件',
      argsLabel: '写入参数',
      argsPreview,
      diffPreview,
    }
  }

  return {
    title: `请求调用 ${tool}`,
    summary: 'AI CLI 请求执行一个需要用户确认的工具动作。',
    riskLabel: highRisk ? '高风险工具' : '工具审批',
    riskTone: highRisk ? 'danger' : 'normal',
    command,
    workspace,
    files,
    consequence: '批准后工具动作会继续执行；拒绝后本轮工具动作不会运行。',
    detailLabel: diffPreview ? '查看 diff / 参数' : '查看工具参数',
    approveLabel: '批准调用工具',
    argsLabel: '工具参数',
    argsPreview,
    diffPreview,
  }
}

function approvalCommand(event: ToolEvent): string {
  return clean(event.args?.command ?? event.args?.cmd ?? event.command)
}

function approvalFiles(event: ToolEvent): string[] {
  const files = new Set<string>()
  const add = (value: unknown) => {
    const text = clean(value)
    if (text) files.add(text)
  }
  add(event.args?.path)
  add(event.args?.file)
  add(event.args?.target)
  if (Array.isArray(event.args?.files)) {
    for (const file of event.args.files) add(file)
  }
  if (Array.isArray(event.args?.changes)) {
    for (const change of event.args.changes) {
      if (change && typeof change === 'object') {
        const record = change as Record<string, unknown>
        add(record.path ?? record.file ?? record.target)
      }
    }
  }
  if (Array.isArray(event.diff?.files)) {
    for (const file of event.diff.files) add(file)
  }
  return Array.from(files)
}

function approvalLooksHighRisk(tool: string, command: string): boolean {
  const value = `${tool} ${command}`.toLowerCase()
  return /\b(git\s+push|rm\s+-rf|del\s+\/|remove-item|npm\s+i|npm\s+install|pnpm\s+add|yarn\s+add|cargo\s+install|chmod|chown|sudo|ssh|scp|curl|wget)\b/.test(value)
}

function formatApprovalArgs(event: ToolEvent): string {
  if (!event.args) return ''
  try {
    return JSON.stringify(event.args, null, 2)
  } catch {
    return clean(event.args)
  }
}

function closedApprovalNotice(state: { label: string; meta: string }): string {
  return `${state.label}：${state.meta || '这个审批已经处理，不能再次操作。'}`
}
