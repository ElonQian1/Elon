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
import type { ProcessCard } from './taskProcessCardModel'
import type { ChatMessage, TaskContext, TaskTone } from './types'
import { coverageLabels } from './taskTimelineModel'
import type { TaskTimelineDiagnostic, TaskTimelineModel, TaskTimelineStage, TimelineItem, TimelineItemKind } from './taskTimelineModel'
import { launchWinClientProtocol, WIN_CLIENT_DOWNLOAD_URL } from '../node/launchWinClient'
import styles from './TaskTimeline.module.css'

interface TaskTimelineProps {
  model: TaskTimelineModel
  taskContext: TaskContext
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

type PrimaryTimelineBlock =
  | { type: 'item'; item: TimelineItem }
  | { type: 'commands'; id: string; items: TimelineItem[] }

export default function TaskTimeline({ model, taskContext, onCancel, onApprove }: TaskTimelineProps) {
  if (model.items.length === 0) return null
  const grouped = groupTimelineItems(model.items)
  const primaryBlocks = groupPrimaryTimelineBlocks(grouped.primary)
  const showStageAtTop = (model.stage.key !== 'finished' && model.stage.key !== 'heartbeat') || model.stage.stuck
  const statusCount = (showStageAtTop ? 0 : 1) + model.diagnostics.length + 1

  return (
    <div className={styles.timeline}>
      {showStageAtTop && <StageCard stage={model.stage} />}
      {primaryBlocks.map((block) => (
        block.type === 'commands' ? (
          <CommandRunGroup key={block.id} items={block.items} />
        ) : (
          <TimelineRow
            key={block.item.id}
            item={block.item}
            taskContext={taskContext}
            onCancel={onCancel}
            onApprove={onApprove}
          />
        )
      ))}
      <TimelineFold title="连接信息" count={grouped.connection.length} defaultOpen={!model.coverage.finalReply && grouped.primary.length === 0}>
        {grouped.connection.map((item) => (
          <TimelineRow
            key={item.id}
            item={item}
            taskContext={taskContext}
            onCancel={onCancel}
            onApprove={onApprove}
          />
        ))}
      </TimelineFold>
      <TimelineFold title="状态诊断" count={statusCount} defaultOpen={model.stage.stuck}>
        {!showStageAtTop && <StageCard stage={model.stage} />}
        {model.diagnostics.map((diagnostic, index) => (
          <DiagnosticCard key={`${diagnostic.title}-${index}`} diagnostic={diagnostic} />
        ))}
        <CoverageStrip model={model} />
      </TimelineFold>
      <TimelineFold title="用量与摘要" count={grouped.summary.length}>
        {grouped.summary.map((item) => (
          <TimelineRow
            key={item.id}
            item={item}
            taskContext={taskContext}
            onCancel={onCancel}
            onApprove={onApprove}
          />
        ))}
      </TimelineFold>
    </div>
  )
}

function groupPrimaryTimelineBlocks(items: TimelineItem[]): PrimaryTimelineBlock[] {
  const blocks: PrimaryTimelineBlock[] = []
  let commandItems: TimelineItem[] = []

  const flushCommands = () => {
    if (!commandItems.length) return
    blocks.push({
      type: 'commands',
      id: `commands-${commandItems[0].id}-${commandItems.length}`,
      items: commandItems,
    })
    commandItems = []
  }

  for (const item of items) {
    if (isCommandTimelineItem(item)) {
      commandItems.push(item)
      continue
    }
    flushCommands()
    blocks.push({ type: 'item', item })
  }
  flushCommands()
  return blocks
}

function isCommandTimelineItem(item: TimelineItem): boolean {
  return Boolean(item.process?.commandText)
}

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

function groupTimelineItems(items: TimelineItem[]) {
  const grouped: { primary: TimelineItem[]; connection: TimelineItem[]; summary: TimelineItem[] } = {
    primary: [],
    connection: [],
    summary: [],
  }
  for (const item of items) {
    if (isConnectionItem(item)) grouped.connection.push(item)
    else if (isSummaryItem(item)) grouped.summary.push(item)
    else grouped.primary.push(item)
  }
  return grouped
}

function isConnectionItem(item: TimelineItem) {
  if (item.kind === 'heartbeat') return false
  if (item.kind === 'node') return true
  if (item.event?.type === 'pc_dispatch_started') return true
  const phase = String(item.event?.phase ?? '')
  const text = [item.title, item.detail, item.meta, phase].filter(Boolean).join(' ')
  if (/^(Codex|Claude|Copilot|Gemini|AI CLI)$/i.test(text.trim())) return true
  return /(执行权|会话隔离|PC 节点|pc 节点|绑定的 PC|正在直连|正在连接|快速检查|巡检|派发|确认接收|等待 .*CLI|正在处理中|重连)/i.test(text)
}

function isSummaryItem(item: TimelineItem) {
  if (item.event?.type === 'usage' || item.event?.type === 'runtime_summary') return true
  const text = [item.title, item.detail].filter(Boolean).join(' ')
  return item.kind === 'status' && item.tone !== 'failed' && /(用量统计|运行完成|任务完成)/.test(text)
}

function CoverageStrip({ model }: { model: TaskTimelineModel }) {
  const labels = coverageLabels(model.coverage)
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
  const showNodeRecovery = stage.stuck && (stage.key === 'heartbeat' || stage.key === 'timeout' || stage.key === 'dispatch')
  return (
    <div className={[styles.stageCard, styles[`tone_${stage.tone}`]].join(' ')} data-stuck={stage.stuck ? 'true' : undefined}>
      <div>
        <strong>{stage.label}</strong>
        {stage.meta && <em>{stage.meta}</em>}
      </div>
      <span>{stage.detail}</span>
      {showNodeRecovery && (
        <div className={styles.recoveryActions}>
          <button type="button" onClick={launchWinClientProtocol}>启动 Win 端</button>
          <a href={WIN_CLIENT_DOWNLOAD_URL} download>下载 Win 端</a>
          <a href="/pc/node">节点设置</a>
        </div>
      )}
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

function TimelineRow({ item, taskContext, onCancel, onApprove }: {
  item: TimelineItem
  taskContext: TaskContext
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}) {
  const embedded = shouldRenderEmbeddedMessage(item)

  return (
    <div className={[styles.item, styles[`tone_${item.tone}`], styles[`kind_${item.kind}`], item.compact ? styles.compact : ''].filter(Boolean).join(' ')}>
      <div className={styles.rail}>
        <span className={styles.icon}>{iconFor(item.kind, item.tone)}</span>
      </div>
      <div className={styles.content}>
        <div className={styles.head}>
          <span className={styles.title}>{item.title}</span>
          {item.meta && <span className={styles.meta}>{item.meta}</span>}
        </div>
        {item.process && <ProcessCardView process={item.process} />}
        {item.detail && !embedded && !item.process && <div className={styles.detail}>{item.detail}</div>}
        {embedded && item.message && (
          <div className={styles.embedded}>
            <DevTaskMessage
              message={item.message as ChatMessage}
              context={taskContext}
              onCancel={onCancel}
              onApprove={onApprove}
            />
          </div>
        )}
      </div>
    </div>
  )
}

function shouldRenderEmbeddedMessage(item: TimelineItem): boolean {
  const type = item.event?.type
  if (item.process) return false
  return type === 'tool_approval_required'
    || type === 'tool_approval_decision'
}

function CommandRunGroup({ items }: { items: TimelineItem[] }) {
  const failed = items.some((item) => item.tone === 'failed')
  return (
    <details className={styles.commandRunGroup} data-tone={failed ? 'failed' : undefined}>
      <summary>
        <span className={styles.commandRunGroupIcon} aria-hidden="true"><Terminal size={13} /></span>
        <span>已运行 {items.length} 条命令</span>
      </summary>
      <div className={styles.commandRunBody}>
        {items.map((item) => (
          <CommandRunItem key={item.id} item={item} />
        ))}
      </div>
    </details>
  )
}

function CommandRunItem({ item }: { item: TimelineItem }) {
  const process = item.process
  if (!process) return null
  const commandText = process.commandText ?? process.subtitle
  return (
    <details className={styles.commandRunItem}>
      <summary title={commandText}>
        <span>已运行</span>
        <span className={styles.commandRunOpenLabel}>命令</span>
        <code>{commandSummary(commandText)}</code>
      </summary>
      <ShellProcessCardView process={process} />
    </details>
  )
}

function ProcessCardView({ process }: { process: ProcessCard }) {
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
          <details className={styles.processDetails}>
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
