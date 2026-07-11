import { AlertTriangle, FileCode2, KeyRound, ListChecks, Terminal } from 'lucide-react'
import { clean } from '../../lib/utils'
import MarkdownContent from '../markdown/MarkdownContent'
import { DevTaskMessage } from './DevTaskCard'
import { buildTimelineDisplay, isAssistantTimelineItem } from './taskTimelineDisplayModel'
import { isStatusEchoProgressText } from './taskTimelineRuntime'
import type { TimelineItem, TimelineItemKind, TaskTimelineStage } from './taskTimelineModel'
import type { buildTaskTimeline } from './taskTimelineModel'
import type { ChatMessage, TaskContext, TaskTone } from './types'
import styles from './DevTaskGroup.module.css'

export interface ProgressSurfaceItem {
  surfaceType?: 'text' | 'commands' | 'artifact'
  id: string
  title?: string
  detail?: string
  meta?: string
  kind?: TimelineItemKind
  items?: TimelineItem[]
  tone?: TaskTone
}

interface TaskProgressHighlightsProps {
  items: ProgressSurfaceItem[]
  hiddenCount: number
  expandAll?: boolean
  taskContext?: TaskContext
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

export function TaskProgressHighlights({
  items,
  hiddenCount,
  expandAll = false,
  taskContext,
  onCancel,
  onApprove,
}: TaskProgressHighlightsProps) {
  return (
    <div className={styles.progressHighlights}>
      {items.map((item) => {
        if (item.surfaceType === 'commands') {
          return <ProgressCommandGroup key={item.id} item={item} expandAll={expandAll} />
        }
        if (item.surfaceType === 'artifact') {
          return (
            <ProgressArtifactSummary
              key={item.id}
              item={item}
              expandAll={expandAll}
              taskContext={taskContext}
              onCancel={onCancel}
              onApprove={onApprove}
            />
          )
        }
        const content = item.detail ?? ''
        const hasMarkdown = /[#*`\[\]>|]/.test(content)
        return (
          <div key={item.id} className={styles.progressHighlight} data-tone={item.tone || undefined}>
            {item.title && <div className={styles.progressHighlightTitle}>{item.title}</div>}
            {hasMarkdown ? <MarkdownContent content={content} copy={false} /> : <p>{content}</p>}
          </div>
        )
      })}
      {hiddenCount > 0 && <div className={styles.progressHighlightMore}>还有 {hiddenCount} 条早前步骤在过程里</div>}
    </div>
  )
}

export function publicAssistantPreviewItems(items: TimelineItem[], maxCount: number): TimelineItem[] {
  const deduped: TimelineItem[] = []
  for (const item of items) {
    const text = clean(item.detail ?? '')
    if (!text) continue
    const previous = deduped[deduped.length - 1]
    if (previous && clean(previous.detail ?? '') === text) continue
    deduped.push(item)
  }
  return deduped.slice(Math.max(0, deduped.length - maxCount))
}

export function progressFlowSurfaceItems(
  timeline: ReturnType<typeof buildTaskTimeline>,
  completed: boolean,
): ProgressSurfaceItem[] {
  const display = buildTimelineDisplay(timeline, { completed, hideAssistantReplies: false })
  const items: ProgressSurfaceItem[] = []
  for (const block of display.primaryBlocks) {
    if (block.type === 'commands') {
      const commandItems = block.items.filter((item) => clean(commandTextForSurface(item)))
      if (!commandItems.length) continue
      items.push({
        surfaceType: 'commands',
        id: block.id,
        items: commandItems,
        tone: commandSurfaceTone(commandItems),
      })
      continue
    }
    const item = block.item
    if (isAssistantTimelineItem(item)) {
      const detail = clean(item.detail ?? '')
      if (!detail || isStatusEchoProgressText(detail)) continue
      items.push({ surfaceType: 'text', id: item.id, detail, tone: item.tone })
      continue
    }
    const artifactItem = progressArtifactSurfaceItem(item)
    if (artifactItem) {
      items.push(artifactItem)
      continue
    }
    if (item.tone === 'failed') {
      const detail = clean(item.detail || item.title)
      if (!detail) continue
      items.push({
        surfaceType: 'text',
        id: item.id,
        title: item.title,
        detail,
        tone: item.tone,
      })
    }
  }
  return dedupeProgressSurfaceItems(items)
}

export function progressSurfaceItems(stage: TaskTimelineStage, assistantItems: TimelineItem[]): ProgressSurfaceItem[] {
  const items: ProgressSurfaceItem[] = []
  const stageItem = stageSurfaceItem(stage)
  if (stageItem) items.push(stageItem)
  for (const item of assistantItems) {
    const detail = clean(item.detail ?? '')
    if (!detail) continue
    items.push({
      surfaceType: 'text',
      id: item.id,
      detail,
      tone: item.tone,
    })
  }
  return dedupeProgressSurfaceItems(items)
}

function ProgressCommandGroup({ item, expandAll = false }: { item: ProgressSurfaceItem; expandAll?: boolean }) {
  const commandItems = (item.items ?? []).filter((commandItem) => clean(commandTextForSurface(commandItem)))
  if (!commandItems.length) return null
  const tone = commandSurfaceTone(commandItems)
  const verb = commandSurfaceVerb(tone)
  if (commandItems.length === 1) {
    return (
      <details className={styles.progressCommandSingleDetails} data-tone={tone} open={expandAll || tone === 'failed'}>
        <summary className={styles.progressCommandSingle} data-tone={tone}>
          <Terminal size={13} aria-hidden="true" />
          <span>{verb}</span>
          <code title={commandTextForSurface(commandItems[0])}>{commandSummaryForSurface(commandTextForSurface(commandItems[0]))}</code>
        </summary>
        <ProgressCommandDetail item={commandItems[0]} />
      </details>
    )
  }
  const visibleCommands = commandItems.slice(0, 6)
  const hiddenCount = commandItems.length - visibleCommands.length
  return (
    <details className={styles.progressCommandGroup} data-tone={tone} open={expandAll || tone === 'failed'}>
      <summary>
        <Terminal size={13} aria-hidden="true" />
        <span>{verb} {commandItems.length} 条命令</span>
      </summary>
      <div className={styles.progressCommandList}>
        {visibleCommands.map((commandItem, index) => (
          <details key={`${commandItem.id}-${index}`} className={styles.progressCommandLine} open={expandAll || commandItem.tone === 'failed'}>
            <summary>
              <span>{commandLineState(commandItem)}</span>
              <code title={commandTextForSurface(commandItem)}>{commandSummaryForSurface(commandTextForSurface(commandItem))}</code>
            </summary>
            <ProgressCommandDetail item={commandItem} />
          </details>
        ))}
        {hiddenCount > 0 && <div className={styles.progressCommandMore}>另有 {hiddenCount} 条命令</div>}
      </div>
    </details>
  )
}

function ProgressCommandDetail({ item }: { item: TimelineItem }) {
  const process = item.process
  const command = commandTextForSurface(item)
  const output = clean(process?.body ?? '')
  const chips = process?.chips ?? []
  return (
    <div className={styles.progressCommandDetail}>
      {command && <pre data-monospace="true">{`$ ${command}`}</pre>}
      {output && <pre data-monospace={process?.monospace ? 'true' : undefined}>{output}</pre>}
      {chips.length > 0 && (
        <div className={styles.progressCommandMeta}>
          {chips.map((chip, index) => (
            <em key={`${chip.label}-${index}`} data-tone={chip.tone || undefined}>{chip.label}</em>
          ))}
        </div>
      )}
    </div>
  )
}

function ProgressArtifactSummary({ item, expandAll = false, taskContext, onCancel, onApprove }: {
  item: ProgressSurfaceItem
  expandAll?: boolean
  taskContext?: TaskContext
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}) {
  const timelineItem = item.items?.[0]
  if (item.kind === 'approval' && timelineItem?.message && taskContext) {
    return (
      <div className={styles.progressApproval}>
        <DevTaskMessage
          message={timelineItem.message as ChatMessage}
          context={taskContext}
          onCancel={onCancel}
          onApprove={onApprove}
        />
      </div>
    )
  }
  const Icon = progressArtifactIcon(item.kind, item.tone)
  const body = clean(timelineItem?.process?.body ?? '')
  const process = timelineItem?.process
  if (body) {
    return (
      <details
        className={styles.progressArtifactDetails}
        data-kind={item.kind || undefined}
        data-tone={item.tone || undefined}
        open={expandAll || item.tone === 'failed'}
      >
        <summary className={styles.progressArtifactSingle} data-kind={item.kind || undefined} data-tone={item.tone || undefined}>
          <Icon size={13} aria-hidden="true" />
          <span>{item.title}</span>
          {item.detail && <strong title={item.detail}>{item.detail}</strong>}
          {item.meta && <em title={item.meta}>{item.meta}</em>}
        </summary>
        <pre data-monospace={process?.monospace ? 'true' : undefined}>{body}</pre>
      </details>
    )
  }
  return (
    <div className={styles.progressArtifactSingle} data-kind={item.kind || undefined} data-tone={item.tone || undefined}>
      <Icon size={13} aria-hidden="true" />
      <span>{item.title}</span>
      {item.detail && <strong title={item.detail}>{item.detail}</strong>}
      {item.meta && <em title={item.meta}>{item.meta}</em>}
    </div>
  )
}

function stageSurfaceItem(stage: TaskTimelineStage): ProgressSurfaceItem | null {
  if (!shouldSurfaceStage(stage)) return null
  const detail = surfaceStageDetail(stage)
  const title = surfaceStageTitle(stage)
  if (!detail && !title) return null
  return {
    id: `stage-${stage.key}`,
    surfaceType: 'text',
    title,
    detail: detail || surfaceStageFallback(stage.key),
    tone: stage.tone,
  }
}

function shouldSurfaceStage(stage: TaskTimelineStage): boolean {
  if (stage.stuck) return true
  return [
    'artifact',
    'command',
    'dispatch',
    'empty',
    'heartbeat',
    'recovery',
    'recovering',
    'recovery-timeout',
    'resume-required',
    'server-update',
    'timeout',
    'tool-timeout',
    'win-update',
  ].includes(stage.key)
}

function surfaceStageTitle(stage: TaskTimelineStage): string {
  const label = clean(stage.label)
    .replace(/^最后公开步骤[：:]\s*/, '')
    .replace(/^当前[：:]\s*/, '')
  if (stage.key === 'dispatch') return '连接节点'
  if (stage.key === 'heartbeat') return '等待输出'
  if (stage.key === 'recovery' || stage.key === 'recovering') return '恢复连接'
  if (stage.key === 'server-update') return '服务器更新'
  if (stage.key === 'win-update') return 'Win 端更新'
  if (stage.key === 'recovery-timeout') return '恢复超时'
  if (stage.key === 'resume-required') return '需要继续'
  if (stage.key === 'timeout') return 'CLI 无输出超时'
  if (stage.key === 'tool-timeout') return '工具结果超时'
  if (stage.key === 'command') return '执行命令'
  if (stage.key === 'empty') return '准备中'
  if (stage.key === 'artifact') return label || '同步产物'
  return label
}

function surfaceStageDetail(stage: TaskTimelineStage): string {
  if (stage.key === 'empty') return '我正在准备处理这轮请求。连接到可用节点后会继续。'
  if (stage.key === 'dispatch') return '我正在连接本机节点。确认执行环境后会继续处理。'
  if (stage.key === 'heartbeat') return '我已经接到任务，正在等待本机 AI 输出。收到公开步骤后会继续更新这里。'
  if (stage.key === 'recovery' || stage.key === 'recovering') return '我正在恢复本轮任务连接。先确认本地会话状态，再接上后续步骤。'
  if (stage.key === 'server-update') return '服务器正在更新升级。更新完成后会自动恢复通信。'
  if (stage.key === 'win-update') return 'Win 端正在更新升级。完成后会自动重连并继续本轮任务。'
  if (stage.key === 'recovery-timeout') return '通信自动恢复已超时。可以重试继续；如果仍失败，先检查 Win 节点连接状态。'
  if (stage.key === 'resume-required') return '本轮任务需要继续。确认本机节点在线后，可以点击继续恢复后续步骤。'
  if (stage.key === 'timeout') return '本轮没有收到公开命令、工具结果、回复片段或完成事件。可以重试继续；过程里保留原始状态。'
  if (stage.key === 'tool-timeout') return '工具已经开始执行，但长时间没有返回结果。可以重试继续；必要时检查命令是否卡住。'
  if (stage.key === 'command') return '我正在等待命令执行结果。命令完成后会继续更新公开过程。'
  if (stage.key === 'artifact') return clean(stage.detail) || '正在同步构建产物和安装入口。'
  return clean(stage.detail)
}

function surfaceStageFallback(stageKey: string): string {
  if (stageKey === 'empty') return '正在准备处理这轮请求。'
  if (stageKey === 'dispatch') return '正在连接本机节点。'
  if (stageKey === 'heartbeat') return '已经接到任务，正在等待本机 AI 输出。'
  if (stageKey === 'recovery' || stageKey === 'recovering') return '正在恢复本轮任务连接。'
  if (stageKey === 'command') return '正在等待命令执行结果。'
  return '正在处理这轮任务。'
}

function dedupeProgressSurfaceItems(items: ProgressSurfaceItem[]): ProgressSurfaceItem[] {
  const deduped: ProgressSurfaceItem[] = []
  for (const item of items) {
    if (item.surfaceType === 'commands') {
      const commandItems = (item.items ?? []).filter((commandItem) => clean(commandTextForSurface(commandItem)))
      if (!commandItems.length) continue
      const signature = commandItems.map(commandTextForSurface).join('\n')
      if (deduped.some((existing) => existing.surfaceType === 'commands' && (existing.items ?? []).map(commandTextForSurface).join('\n') === signature)) continue
      deduped.push({ ...item, items: commandItems })
      continue
    }
    if (item.surfaceType === 'artifact') {
      const signature = [
        item.surfaceType,
        item.kind ?? '',
        item.title ?? '',
        item.detail ?? '',
        item.meta ?? '',
      ].map(clean).join('\n')
      if (!signature.trim()) continue
      if (deduped.some((existing) => {
        if (existing.surfaceType !== 'artifact') return false
        return [
          existing.surfaceType,
          existing.kind ?? '',
          existing.title ?? '',
          existing.detail ?? '',
          existing.meta ?? '',
        ].map(clean).join('\n') === signature
      })) continue
      deduped.push(item)
      continue
    }
    const text = clean(item.detail ?? '')
    if (!text) continue
    if (deduped.some((existing) => existing.surfaceType === 'text' && clean(existing.detail ?? '') === text)) continue
    deduped.push({ ...item, surfaceType: 'text', detail: text })
  }
  return deduped
}

function progressArtifactSurfaceItem(item: TimelineItem): ProgressSurfaceItem | null {
  if (!['file', 'test', 'approval', 'artifact'].includes(item.kind)) return null
  const title = progressArtifactVerb(item)
  const detail = progressArtifactDetail(item)
  const meta = progressArtifactMeta(item, detail)
  if (!title && !detail && !meta) return null
  return {
    surfaceType: 'artifact',
    id: item.id,
    title,
    detail,
    meta,
    kind: item.kind,
    items: [item],
    tone: item.tone,
  }
}

function progressArtifactVerb(item: TimelineItem): string {
  if (item.kind === 'file') {
    if (item.tone === 'running' || item.tone === 'queued') return '正在修改文件'
    if (item.tone === 'failed') return '文件修改失败'
    return '修改文件'
  }
  if (item.kind === 'test') {
    if (item.tone === 'running' || item.tone === 'queued') return '正在验证'
    if (item.tone === 'failed') return '验证失败'
    if (item.tone === 'canceled') return '验证已停止'
    return '验证通过'
  }
  if (item.kind === 'approval') {
    if (item.tone === 'approval' || item.tone === 'running' || item.tone === 'queued') return '等待审批'
    if (item.tone === 'canceled') return '审批已取消'
    return '审批已处理'
  }
  if (item.kind === 'artifact') return clean(item.title) || '同步产物'
  return clean(item.title)
}

function progressArtifactDetail(item: TimelineItem): string {
  if (item.kind === 'file') {
    return processChipLabel(item, /文件/)
      || fileCountFromDetail(item.detail)
      || compactSurfaceText(item.process?.subtitle ?? item.detail ?? item.title)
  }
  if (item.kind === 'test') {
    return testStatusLabel(item)
  }
  if (item.kind === 'approval') {
    return clean(item.meta ?? item.process?.subtitle ?? item.detail ?? '')
  }
  if (item.kind === 'artifact') {
    return compactSurfaceText(item.detail ?? item.process?.subtitle ?? '')
  }
  return ''
}

function progressArtifactMeta(item: TimelineItem, detail: string): string {
  if (item.kind === 'file') {
    const result = compactSurfaceText(item.detail ?? '')
    if (result && result !== detail) return result
    return compactSurfaceText(item.process?.subtitle ?? '')
  }
  if (item.kind === 'test') {
    return compactSurfaceText(item.process?.commandText ?? item.process?.subtitle ?? item.detail ?? '')
  }
  if (item.kind === 'approval') {
    const message = compactSurfaceText(item.detail ?? '')
    if (message && message !== detail) return message
  }
  return ''
}

function processChipLabel(item: TimelineItem, pattern: RegExp): string {
  return clean(item.process?.chips.find((chip) => pattern.test(chip.label))?.label ?? '')
}

function fileCountFromDetail(value: string | undefined): string {
  const match = clean(value ?? '').match(/\b([0-9]+)\s+files?\s+changed\b/i)
  if (!match) return ''
  return `${match[1]} 个文件`
}

function testStatusLabel(item: TimelineItem): string {
  const exitChip = processChipLabel(item, /^exit=/i)
  if (exitChip && !/^exit=0$/i.test(exitChip)) return exitChip
  if (item.tone === 'failed') return '未通过'
  if (item.tone === 'running' || item.tone === 'queued') return '运行中'
  if (item.tone === 'canceled') return '已停止'
  return processChipLabel(item, /测试|构建/) || '已通过'
}

function compactSurfaceText(value: string | undefined): string {
  const text = clean(value ?? '').replace(/\s+/g, ' ').trim()
  if (text.length <= 96) return text
  return `${text.slice(0, 96)}...`
}

function progressArtifactIcon(kind: TimelineItemKind | undefined, tone: TaskTone | undefined) {
  if (tone === 'failed') return AlertTriangle
  if (kind === 'file') return FileCode2
  if (kind === 'test') return ListChecks
  if (kind === 'approval') return KeyRound
  if (kind === 'artifact') return FileCode2
  return Terminal
}

function commandTextForSurface(item: TimelineItem): string {
  return clean(item.process?.commandText ?? item.process?.subtitle ?? '')
}

function commandSurfaceTone(items: TimelineItem[]): TaskTone {
  if (items.some((item) => item.tone === 'failed')) return 'failed'
  if (items.some((item) => item.tone === 'running' || item.tone === 'queued')) return 'running'
  if (items.some((item) => item.tone === 'canceled')) return 'canceled'
  return 'done'
}

function commandSurfaceVerb(tone: TaskTone): string {
  if (tone === 'running' || tone === 'queued') return '正在运行'
  if (tone === 'failed') return '运行失败'
  if (tone === 'canceled') return '已停止'
  return '已运行'
}

function commandLineState(item: TimelineItem): string {
  if (item.tone === 'running' || item.tone === 'queued') return '正在运行'
  if (item.tone === 'failed') return '运行失败'
  if (item.tone === 'canceled') return '已停止'
  return '已运行'
}

function commandSummaryForSurface(command: string): string {
  const text = command.replace(/\s+/g, ' ').trim()
  if (text.length <= 118) return text
  return `${text.slice(0, 118)}...`
}
