import {
  AlertTriangle,
  Ban,
  CheckCircle2,
  Circle,
  Clock3,
  FileCode2,
  HardDrive,
  KeyRound,
  ListChecks,
  Terminal,
} from 'lucide-react'
import type { ReactNode } from 'react'
import { DevTaskMessage } from './DevTaskCard'
import MarkdownContent from '../markdown/MarkdownContent'
import type { ProcessCard } from './taskProcessCardModel'
import type { ChatMessage, TaskContext, TaskTone } from './types'
import { coverageLabels } from './taskTimelineModel'
import type { TaskTimelineDiagnostic, TaskTimelineModel, TaskTimelineStage, TimelineItem, TimelineItemKind } from './taskTimelineModel'
import {
  buildTimelineDisplay,
  diagnosticFoldTitle,
  isAssistantTimelineItem,
  taskTimelineHasVisibleDetails,
} from './taskTimelineDisplayModel'
import styles from './TaskTimeline.module.css'

interface TaskTimelineProps {
  model: TaskTimelineModel
  taskContext: TaskContext
  completed?: boolean
  hideAssistantReplies?: boolean
  hideCommands?: boolean
  expandAll?: boolean
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

interface RuntimeReply {
  message: string
  tone: TaskTone
  runtime?: string
  phase?: string
  reqId?: string
  fingerprint?: string
}

export default function TaskTimeline({ model, taskContext, completed = false, hideAssistantReplies = false, hideCommands = false, expandAll = false, onCancel, onApprove }: TaskTimelineProps) {
  const display = buildTimelineDisplay(model, { completed, hideAssistantReplies, hideCommands })
  if (!display.hasVisibleTimeline) return null

  return (
    <div className={styles.timeline}>
      {display.showStageAtTop && <StageCard stage={model.stage} />}
      {display.primaryBlocks.map((block) => (
        block.type === 'commands' ? (
          <CommandRunGroup key={block.id} items={block.items} expandAll={expandAll} />
        ) : (
          <TimelineRow
            key={block.item.id}
            item={block.item}
            taskContext={taskContext}
            expandAll={expandAll}
            onCancel={onCancel}
            onApprove={onApprove}
          />
        )
      ))}
      <TimelineFold title="连接诊断" count={display.grouped.connection.length} defaultOpen={false}>
        {display.grouped.connection.map((item) => (
          <TimelineRow
            key={item.id}
            item={item}
            taskContext={taskContext}
            expandAll={expandAll}
            onCancel={onCancel}
            onApprove={onApprove}
          />
        ))}
      </TimelineFold>
      {display.showDiagnosticDetails && (
        <TimelineFold title={diagnosticFoldTitle(model)} count={display.diagnosticCount} defaultOpen={expandAll}>
          {model.diagnostics.map((diagnostic, index) => (
            <DiagnosticCard key={`${diagnostic.title}-${index}`} diagnostic={diagnostic} />
          ))}
          {display.showCoverageInDiagnostics && <CoverageStrip model={model} />}
        </TimelineFold>
      )}
      <TimelineFold title="运行摘要" count={display.grouped.summary.length} defaultOpen={expandAll}>
        {display.grouped.summary.map((item) => (
          <TimelineRow
            key={item.id}
            item={item}
            taskContext={taskContext}
            expandAll={expandAll}
            onCancel={onCancel}
            onApprove={onApprove}
          />
        ))}
      </TimelineFold>
    </div>
  )
}

export { taskTimelineHasVisibleDetails }

function TimelineFold({ title, count, defaultOpen = false, children }: {
  title: string
  count: number
  defaultOpen?: boolean
  children: ReactNode
}) {
  if (count <= 0) return null
  return (
    <details className={styles.fold} open={defaultOpen}>
      <summary>
        <span>{title}</span>
        <em>{count} 项</em>
      </summary>
      <div className={styles.foldBody}>{children}</div>
    </details>
  )
}

function CoverageStrip({ model }: { model: TaskTimelineModel }) {
  const labels = coverageLabels(model.coverage).filter((item) => item.active)
  if (!labels.length) return null
  return (
    <div className={styles.coverage} aria-label="过程覆盖情况">
      {labels.map((item) => (
        <span
          key={item.key}
          className={[styles.coveragePill, item.active ? styles.coverageActive : ''].join(' ')}
        >
          {item.label}
        </span>
      ))}
    </div>
  )
}

function StageCard({ stage }: { stage: TaskTimelineStage }) {
  return (
    <div className={[styles.stageCard, styles[`tone_${stage.tone}`]].join(' ')} data-stuck={stage.stuck ? 'true' : undefined}>
      <div>
        <strong>{stage.label}</strong>
        {stage.meta && <em>{stage.meta}</em>}
      </div>
      <span>{stage.detail}</span>
    </div>
  )
}

function DiagnosticCard({ diagnostic }: { diagnostic: TaskTimelineDiagnostic }) {
  return (
    <div className={[styles.diagnostic, styles[`tone_${diagnostic.tone}`]].join(' ')}>
      <strong>{diagnostic.title}</strong>
      <span>{diagnostic.detail}</span>
    </div>
  )
}

function TimelineRow({ item, taskContext, expandAll = false, onCancel, onApprove }: {
  item: TimelineItem
  taskContext: TaskContext
  expandAll?: boolean
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}) {
  const embedded = shouldRenderEmbeddedMessage(item)
  const assistantReply = isAssistantTimelineItem(item)

  return (
    <div className={[
      styles.item,
      styles[`tone_${item.tone}`],
      styles[`kind_${item.kind}`],
      assistantReply ? styles.publicReplyItem : '',
      item.compact ? styles.compact : '',
    ].filter(Boolean).join(' ')}>
      <div className={styles.rail}>
        <span className={styles.icon}>{iconFor(item.kind, item.tone)}</span>
      </div>
      <div className={styles.content}>
        {assistantReply ? (
          <AssistantTimelineReply item={item} expandAll={expandAll} />
        ) : (
          <>
            <div className={styles.head}>
              <span className={styles.title}>{item.title}</span>
              {item.meta && <span className={styles.meta} title={item.metaTitle || item.meta}>{item.meta}</span>}
            </div>
            {item.process && <ProcessCardView process={item.process} expandAll={expandAll} />}
            {item.detail && !embedded && !item.process && <div className={styles.detail}>{item.detail}</div>}
          </>
        )}
        {embedded && item.message && (
          <div className={styles.embedded}>
            <DevTaskMessage
              message={item.message as ChatMessage}
              context={taskContext}
              onCancel={item.event?.type === 'tool_approval_required' ? undefined : onCancel}
              onApprove={onApprove}
            />
          </div>
        )}
      </div>
    </div>
  )
}

function AssistantTimelineReply({ item, expandAll = false }: { item: TimelineItem; expandAll?: boolean }) {
  const text = item.detail ?? ''
  const runtimeReply = runtimeReplyFromText(text)
  const hasMarkdown = /[#*`\[\]>|]/.test(text)
  const meta = assistantReplyMeta(item.meta)
  if (runtimeReply) return <RuntimeTimelineReply info={runtimeReply} expandAll={expandAll} />
  return (
    <div className={styles.assistantReply}>
      {meta && <div className={styles.assistantReplyMeta}>{meta}</div>}
      {hasMarkdown ? <MarkdownContent content={text} copy /> : <p>{text}</p>}
    </div>
  )
}

function assistantReplyMeta(value: string | undefined): string {
  const meta = String(value ?? '').trim()
  if (/^(codex|claude|copilot|gemini|gpt|gpt-[\w.-]+|ai)$/i.test(meta)) return ''
  return meta
}

function RuntimeTimelineReply({ info, expandAll = false }: { info: RuntimeReply; expandAll?: boolean }) {
  const title = info.tone === 'failed' ? '平台 AI 暂时不可用' : '平台 AI 正在处理'
  const meta = [info.runtime, info.phase].filter(Boolean).join(' · ')
  return (
    <div className={styles.runtimeReply} data-tone={info.tone}>
      <strong>{title}</strong>
      <p>{info.message}</p>
      {(meta || info.reqId || info.fingerprint) && (
        <details className={styles.runtimeReplyDetails} open={expandAll}>
          <summary>技术信息</summary>
          {meta && <span>{meta}</span>}
          {info.reqId && <span>req_id: {info.reqId}</span>}
          {info.fingerprint && <span>fingerprint: {info.fingerprint}</span>}
        </details>
      )}
    </div>
  )
}

function runtimeReplyFromText(text: string): RuntimeReply | null {
  const trimmed = text.trim()
  if (!trimmed.startsWith('{')) return null
  try {
    const value = JSON.parse(trimmed) as Record<string, unknown>
    const type = String(value.type ?? '')
    const schema = String(value.schema ?? '')
    if (type !== 'runtime_status' && type !== 'runtime_summary' && !schema.includes('runtime_')) return null
    const status = String(value.status ?? '').toLowerCase()
    const phase = String(value.phase ?? '').toLowerCase()
    const failed = status === 'error' || phase === 'failed'
    const message = String(value.message ?? '').trim()
    return {
      message: message || (failed ? '服务商返回错误，本轮没有生成有效回复。' : '正在调用平台模型。'),
      tone: failed ? 'failed' : 'running',
      runtime: String(value.runtime ?? '').trim() || undefined,
      phase: phase || undefined,
      reqId: String(value.req_id ?? '').trim() || undefined,
      fingerprint: String(value.fingerprint ?? '').trim() || undefined,
    }
  } catch {
    return null
  }
}

function shouldRenderEmbeddedMessage(item: TimelineItem): boolean {
  const type = item.event?.type
  if (item.process) return false
  return type === 'tool_approval_required'
    || type === 'tool_approval_decision'
}

function CommandRunGroup({ items, expandAll = false }: { items: TimelineItem[]; expandAll?: boolean }) {
  const failed = items.some((item) => item.tone === 'failed')
  return (
    <details className={styles.commandRunGroup} data-tone={failed ? 'failed' : undefined} open={expandAll}>
      <summary>
        <span className={styles.commandRunGroupIcon} aria-hidden="true"><Terminal size={13} /></span>
        <span>已运行 {items.length} 条命令</span>
      </summary>
      <div className={styles.commandRunBody}>
        {items.map((item, index) => (
          <CommandRunItem key={`${item.id}-${index}`} item={item} />
        ))}
      </div>
    </details>
  )
}

function CommandRunItem({ item }: { item: TimelineItem }) {
  const process = item.process
  if (!process) return null
  const commandText = process.commandText ?? process.subtitle
  const openByDefault = item.tone === 'failed'
  return (
    <details className={styles.commandRunItem} open={openByDefault}>
      <summary title={commandText}>
        <span className={styles.commandRunOpenLabel}>命令</span>
        <code>{commandSummary(commandText)}</code>
        <span className={styles.commandRunStatus} data-tone={item.tone}>{shellStatus(process).label}</span>
      </summary>
      <ShellProcessCardView process={process} />
    </details>
  )
}

function ProcessCardView({ process, expandAll = false }: { process: ProcessCard; expandAll?: boolean }) {
  if (process.commandText) return <ShellProcessCardView process={process} />
  return (
    <div className={[styles.processCard, styles[`process_${process.kind}`], styles[`tone_${process.tone}`]].join(' ')}>
      <div className={styles.processCardHead}>
        <span>{process.subtitle}</span>
        <div className={styles.processChips}>
          {process.chips.map((chip, index) => (
            <em key={`${chip.label}-${index}`} data-tone={chip.tone || undefined}>{chip.label}</em>
          ))}
          {process.truncated && <em data-tone="muted">已截断</em>}
        </div>
      </div>
      {process.body && (
        process.bodyCollapsed ? (
          <details className={styles.processDetails} open={expandAll || process.tone === 'failed'}>
            <summary>{process.bodyLabel}</summary>
            <pre data-monospace={process.monospace ? 'true' : undefined}>{process.body}</pre>
          </details>
        ) : (
          <div className={styles.processBody}>
            <strong>{process.bodyLabel}</strong>
            <pre data-monospace={process.monospace ? 'true' : undefined}>{process.body}</pre>
          </div>
        )
      )}
    </div>
  )
}

function ShellProcessCardView({ process }: { process: ProcessCard }) {
  const commandText = process.commandText ?? ''
  const status = shellStatus(process)
  return (
    <div className={[styles.processCard, styles.shellProcessCard, styles[`process_${process.kind}`], styles[`tone_${process.tone}`]].join(' ')}>
      <div className={styles.shellPanelHeader}>
        <span>Shell</span>
      </div>
      <pre className={styles.shellCode} data-monospace="true">{`$ ${commandText}`}</pre>
      {process.body && (
        <pre className={styles.shellCode} data-monospace={process.monospace ? 'true' : undefined}>{process.body}</pre>
      )}
      <div className={styles.shellPanelFooter} data-tone={status.tone}>
        <span>{status.label}</span>
        {process.truncated && <em>已截断</em>}
      </div>
    </div>
  )
}

function commandSummary(value: string): string {
  const text = value.replace(/\s+/g, ' ').trim()
  if (text.length <= 96) return text
  return `${text.slice(0, 96)}...`
}

function shellStatus(process: ProcessCard): { label: string; tone: TaskTone } {
  if (process.tone === 'failed') return { label: '失败', tone: 'failed' }
  if (process.tone === 'running') return { label: '运行中', tone: 'running' }
  if (process.tone === 'canceled') return { label: '已停止', tone: 'canceled' }
  const exitChip = process.chips.find((chip) => /^exit=/i.test(chip.label))
  if (exitChip && !/^exit=0$/i.test(exitChip.label)) {
    return { label: exitChip.label, tone: exitChip.tone || process.tone }
  }
  return { label: '成功', tone: 'done' }
}

function iconFor(kind: TimelineItemKind, tone: TaskTone) {
  const props = { size: 14, strokeWidth: 2.2 }
  if (tone === 'done') return <CheckCircle2 {...props} />
  if (tone === 'failed') return <AlertTriangle {...props} />
  if (tone === 'canceled') return <Ban {...props} />
  if (kind === 'node') return <HardDrive {...props} />
  if (kind === 'test') return <ListChecks {...props} />
  if (kind === 'file') return <FileCode2 {...props} />
  if (kind === 'tool') return <Terminal {...props} />
  if (kind === 'approval') return <KeyRound {...props} />
  if (kind === 'artifact') return <FileCode2 {...props} />
  if (kind === 'heartbeat') return <Clock3 {...props} />
  return <Circle {...props} />
}
