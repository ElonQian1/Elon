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

export default function TaskTimeline({ model, taskContext, onCancel, onApprove }: TaskTimelineProps) {
  if (model.items.length === 0) return null

  return (
    <div className={styles.timeline}>
      <CoverageStrip model={model} />
      <StageCard stage={model.stage} />
      {model.diagnostics.map((diagnostic, index) => (
        <DiagnosticCard key={`${diagnostic.title}-${index}`} diagnostic={diagnostic} />
      ))}
      {model.items.map((item) => (
        <TimelineRow
          key={item.id}
          item={item}
          taskContext={taskContext}
          onCancel={onCancel}
          onApprove={onApprove}
        />
      ))}
    </div>
  )
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

function ProcessCardView({ process }: { process: ProcessCard }) {
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
        <div className={styles.processBody}>
          <strong>{process.bodyLabel}</strong>
          <pre data-monospace={process.monospace ? 'true' : undefined}>{process.body}</pre>
        </div>
      )}
    </div>
  )
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
