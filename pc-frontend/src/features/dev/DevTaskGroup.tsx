/**
 * DevTaskGroup — 会话里的任务线程
 *
 * 将同一 task_id 的用户请求、过程消息和最终回复连成一个对话段：
 *  - 用户请求显示为右侧气泡
 *  - 最终回复显示为左侧 AI 气泡，避免被过程卡片淹没
 *  - 中间回复片段和命令按发生顺序穿插在过程面板中，任务结束后再折叠
 */
import { StopCircle } from 'lucide-react'
import { memo, useState, useEffect, useRef } from 'react'
import TaskTimeline, { taskTimelineHasVisibleDetails } from './TaskTimeline'
import TaskProgressCard from './TaskProgressCard'
import MarkdownContent from '../markdown/MarkdownContent'
import UserAvatar from '../shell/UserAvatar'
import { clean, formatTime } from '../../lib/utils'
import { messageKind, messageText, shortId, statusForTask, taskIdOf, taskIsTerminal, taskRequestLooksMarkdown, taskResultTone } from './devTaskUtils'
import { buildTaskTimeline, timelineSummary } from './taskTimelineModel'
import { isStatusEchoProgressText } from './taskTimelineRuntime'
import type { TaskTimelineStage, TimelineItem } from './taskTimelineModel'
import type { ChatMessage, TaskContext, TaskState, TaskTone } from './types'
import styles from './DevTaskGroup.module.css'

interface Props {
  messages: ChatMessage[]
  taskContext: TaskContext
  user?: { nickname?: string; account?: string; avatar_data_url?: string | null } | null
  expandAll?: boolean
  onCancel?: (taskId: string) => void
  onContinue?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

interface ProgressSurfaceItem {
  id: string
  title?: string
  detail: string
  tone?: TaskTone
}

function DevTaskGroup({ messages, taskContext, user, expandAll = false, onCancel, onContinue, onApprove }: Props) {
  const taskId  = taskIdForGroup(messages)
  const task    = taskId ? (taskContext.tasks.get(taskId) ?? null) : null
  const userMsg = firstMessageMatching(messages, isUserTaskMessage)
  const terminalAssistantMsg = latestMessageMatching(messages, isTerminalAssistantTaskMessage)
  const fallbackAssistantMsg = latestMessageMatching(messages, (message) => isFallbackAssistantTaskReply(message, taskId))
  const explicitResultMsg = latestMessageOfKind(messages, 'ai_result') ?? terminalAssistantMsg ?? fallbackAssistantMsg
  const isDone  = taskIsTerminal(task) || !!explicitResultMsg || messages.some(isTerminalTaskMessage)

  // 过程默认保持轻量；审批态和预览强制展开由后续逻辑处理。
  const [collapsed, setCollapsed] = useState(!expandAll)
  const prevDone = useRef(isDone)
  const prevCollapseKey = useRef('')

  const headerMsg   = messages.find((m) => messageKind(m) === 'ai_task')
  const resultMsg   = explicitResultMsg ?? (isDone ? latestVisibleProgress(messages) : undefined)
  const progressMsgs = messages.filter((m) => messageKind(m) === 'ai_progress' || isAssistantProgressNote(m))
  const assistantNotes = messages.filter(isAssistantProgressNote)
  const timeline = buildTaskTimeline(progressMsgs, resultMsg, {
    assistantNoteCount: assistantNotes.length,
  })
  const status = statusForTaskGroup(task, isDone, resultMsg)
  const progressStatus = progressStatusForStage(status, timeline.stage.tone, timeline.stage.label, isDone)
  const forceProcessOpen = timeline.stage.key === 'approval'
  const request = taskRequestText(userMsg) || task?.request || taskRequestText(headerMsg)
  const richRequest = taskRequestLooksMarkdown(request)
  const tone = status.tone
  const compactCompletedProcess = isDone && !!resultMsg
  const publicAssistantItems = publicAssistantTimelineItems(timeline)
  const terminalReason = terminalReasonFromTimeline(timeline, tone)
  const processSummary = taskThreadSummary(timeline, publicAssistantItems.length, taskId, taskId ? shortId(taskId) : '')
  const hasPublicAssistantItems = publicAssistantItems.length > 0
  const defaultProcessOpen = shouldDefaultOpenProcess(isDone, timeline, publicAssistantItems)
  const displayCollapsed = forceProcessOpen ? false : collapsed
  const hideTimelineAssistantReplies = false
  const hasProgressDetails = taskTimelineHasVisibleDetails(timeline, {
    completed: compactCompletedProcess,
    hideAssistantReplies: hideTimelineAssistantReplies,
  })
  const progressCount = hasProgressDetails ? timeline.visibleStepCount : 0
  const showProgressPanel = hasProgressDetails || !isDone
  const canCancel = !!taskId && !isDone && !!onCancel
  const requestAuthor = userDisplayName(userMsg, user)
  const requestTime = messageTime(userMsg) || messageTime(headerMsg)
  const assistantProcessTime = messageTime(headerMsg) || requestTime
  const processedDuration = processDurationLabel(messages, isDone)
  const hideCompletedProcessPanel = false
  const previewAssistantItems = publicAssistantPreviewItems(publicAssistantItems, 3)
  const surfaceItems = !resultMsg && displayCollapsed
    ? progressSurfaceItems(timeline.stage, previewAssistantItems)
    : []
  const previewHiddenCount = Math.max(0, publicAssistantItems.length - previewAssistantItems.length)
  const suppressProgressNarrative = (
    surfaceItems.length > 0
    || (!displayCollapsed && hasPublicAssistantItems)
  )
    && timeline.stage.key !== 'approval'
  const directPublicProcess = !resultMsg
    && !displayCollapsed
    && hasPublicAssistantItems
    && !timeline.stage.stuck
    && timeline.stage.key !== 'approval'

  useEffect(() => {
    const collapseKey = `${taskId}:${expandAll ? 'expanded' : 'default'}:${forceProcessOpen ? 'locked' : 'free'}:${defaultProcessOpen ? 'process-open' : 'process-closed'}`
    if (prevCollapseKey.current === collapseKey) return
    prevCollapseKey.current = collapseKey
    prevDone.current = isDone
    setCollapsed(expandAll || forceProcessOpen || defaultProcessOpen ? false : true)
  }, [taskId, expandAll, forceProcessOpen, defaultProcessOpen, isDone])

  // 任务从"运行中"变为"完成"时自动折叠（延迟一下让用户看到结果）
  useEffect(() => {
    if (expandAll || forceProcessOpen) {
      setCollapsed(false)
      prevDone.current = isDone
      return
    }
    if (!prevDone.current && isDone) {
      const t = setTimeout(() => setCollapsed(true), 800)
      prevDone.current = true
      return () => clearTimeout(t)
    }
    prevDone.current = isDone
  }, [expandAll, forceProcessOpen, isDone])

  useEffect(() => {
    if (!taskId || localStorage.getItem('elon_debug_task_timeline') !== '1') return
    console.info('[elon-task-timeline]', {
      taskId,
      status: status.label,
      done: isDone,
      coverage: timeline.coverage,
      steps: timeline.items.map((item) => ({
        kind: item.kind,
        tone: item.tone,
        title: item.title,
        type: item.event?.type,
        tool: item.event?.tool,
        meta: item.meta,
      })),
    })
  }, [taskId, status.label, isDone, processSummary])

  const renderProgressPanel = (inline: boolean, afterNotes = false) => (
    <div
      className={[
        styles.processPanel,
        inline ? styles.processPanelInline : '',
        afterNotes ? styles.processPanelAfterNotes : '',
        hideCompletedProcessPanel ? styles.processPanelDormant : '',
      ].filter(Boolean).join(' ')}
    >
      {!directPublicProcess && (
        <TaskProgressCard
          status={status}
          displayStatus={progressStatus}
          timeline={timeline}
          progressCount={progressCount}
          processSummary={processSummary}
          collapsed={displayCollapsed}
          canCancel={canCancel}
          compact={compactCompletedProcess}
          lockedOpen={forceProcessOpen}
          processedDuration={processedDuration}
          suppressNarrative={suppressProgressNarrative}
          canContinue={!!taskId && !!onContinue && taskStageAllowsContinue(timeline.stage.key, tone)}
          onToggle={() => {
            if (forceProcessOpen) return
            setCollapsed((c) => !c)
          }}
          onCancel={() => {
            if (!taskId) return
            if (window.confirm('停止这个任务？')) onCancel?.(taskId)
          }}
          onContinue={() => {
            if (!taskId) return
            onContinue?.(taskId)
          }}
        />
      )}
      {!displayCollapsed && hasProgressDetails && (
        <div className={[styles.processBody, directPublicProcess ? styles.processBodyDirect : ''].filter(Boolean).join(' ')}>
          <TaskTimeline
            model={timeline}
            taskContext={taskContext}
            completed={compactCompletedProcess}
            hideAssistantReplies={hideTimelineAssistantReplies}
            expandAll={expandAll}
            onCancel={onCancel}
            onApprove={onApprove}
          />
        </div>
      )}
    </div>
  )

  return (
    <div className={[styles.thread, styles[`tone_${tone}`] ?? ''].join(' ')}>
      {request && (
        <div className={styles.userTurn}>
          <div className={styles.userBody}>
            <div className={styles.userMeta}>
              <strong>{requestAuthor.name}</strong>
              {requestTime && <span>{requestTime}</span>}
            </div>
            <div className={[styles.userBubble, richRequest ? styles.userBubbleRich : ''].join(' ')}>
              {richRequest ? <MarkdownContent content={request} copy={false} /> : request}
            </div>
          </div>
          <UserAvatar
            user={{
              id: requestAuthor.name,
              account: requestAuthor.name,
              nickname: requestAuthor.name,
              avatar_data_url: requestAuthor.avatarDataUrl || null,
            }}
            size="compact"
            className={styles.userAvatar}
          />
        </div>
      )}

      {showProgressPanel && (
        !resultMsg && !hideCompletedProcessPanel ? (
          <div className={styles.assistantTurn}>
            <div className={styles.assistantAvatar}>AI</div>
            <div className={styles.assistantBody}>
              <div className={styles.assistantMeta}>
                <strong>一龙</strong>
                {hasPublicAssistantItems && <span>正在处理</span>}
                {assistantProcessTime && <span>{assistantProcessTime}</span>}
                {directPublicProcess && canCancel && (
                  <button
                    type="button"
                    className={styles.assistantMetaCancel}
                    onClick={() => {
                      if (!taskId) return
                      if (window.confirm('停止这个任务？')) onCancel?.(taskId)
                    }}
                    aria-label="停止任务"
                    title="停止任务"
                  >
                    <StopCircle size={13} />
                  </button>
                )}
              </div>
              {surfaceItems.length > 0 && (
                <TaskProgressHighlights items={surfaceItems} hiddenCount={previewHiddenCount} />
              )}
              {renderProgressPanel(true, hasPublicAssistantItems || surfaceItems.length > 0)}
            </div>
          </div>
        ) : renderProgressPanel(false)
      )}

      {resultMsg && (
        <TaskAssistantBubble
          message={resultMsg}
          tone={tone}
          label={replyLabelForTone(tone)}
          reason={terminalReason}
          time={assistantProcessTime}
        />
      )}
    </div>
  )
}

export default memo(DevTaskGroup, (prev, next) =>
  prev.messages === next.messages
  && prev.taskContext === next.taskContext
  && prev.user?.nickname === next.user?.nickname
  && prev.user?.account === next.user?.account
  && prev.user?.avatar_data_url === next.user?.avatar_data_url
  && prev.expandAll === next.expandAll
  && prev.onCancel === next.onCancel
  && prev.onContinue === next.onContinue
  && prev.onApprove === next.onApprove
)

function progressStatusForStage(
  status: { tone: TaskTone; label: string },
  stageTone: TaskTone,
  stageLabel: string,
  isDone: boolean,
): { tone: TaskTone; label: string } {
  if (isDone) return status
  if (stageTone === 'approval') return { tone: 'approval', label: stageLabel || '等待确认' }
  if (stageTone === 'failed') return { tone: 'failed', label: stageLabel || '需要处理' }
  return status
}

function TaskAssistantBubble({ message, tone, label, reason, time: fallbackTime }: {
  message: ChatMessage
  tone: TaskTone
  label: string
  reason?: string
  time?: string
}) {
  const content = messageText(message)
  if (!content) return null
  const displayContent = terminalDisplayContent(content, tone, reason)
  const failed = tone === 'failed'
  const canceled = tone === 'canceled'
  const hasMarkdown = /[#*`\[\]>|]/.test(displayContent)
  const time = messageTime(message) || fallbackTime

  return (
    <div className={styles.assistantTurn}>
      <div className={styles.assistantAvatar}>AI</div>
      <div className={styles.assistantBody}>
        <div className={styles.assistantMeta}>
          <strong>一龙</strong>
          <span>{label}</span>
          {time && <span>{time}</span>}
        </div>
        <div className={[styles.assistantBubble, failed ? styles.replyFailed : canceled ? styles.replyCanceled : ''].join(' ')}>
          {hasMarkdown ? <MarkdownContent content={displayContent} copy /> : displayContent}
        </div>
      </div>
    </div>
  )
}

function TaskProgressHighlights({ items, hiddenCount }: { items: ProgressSurfaceItem[]; hiddenCount: number }) {
  return (
    <div className={styles.progressHighlights}>
      {items.map((item) => {
        const content = item.detail
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

function replyLabelForTone(tone: TaskTone): string {
  if (tone === 'failed') return '任务失败'
  if (tone === 'canceled') return '任务已停止'
  return '最终回复'
}

function taskStageAllowsContinue(stageKey: string, tone: TaskTone): boolean {
  if (tone === 'failed') return true
  return [
    'heartbeat',
    'resume-required',
    'recovery-timeout',
    'timeout',
    'tool-timeout',
  ].includes(stageKey)
}

function shouldDefaultOpenProcess(
  isDone: boolean,
  timeline: ReturnType<typeof buildTaskTimeline>,
  publicAssistantItems: TimelineItem[],
): boolean {
  if (isDone) return false
  if (timeline.stage.key === 'approval') return true
  return publicAssistantItems.length > 0
}

function terminalReasonFromTimeline(timeline: ReturnType<typeof buildTaskTimeline>, tone: TaskTone): string {
  if (tone !== 'failed' && tone !== 'canceled') return ''
  for (let index = timeline.items.length - 1; index >= 0; index--) {
    const item = timeline.items[index]
    if (item.event?.type !== 'runtime_status') continue
    if (tone === 'failed' && item.tone !== 'failed') continue
    if (tone === 'canceled' && item.tone !== 'canceled') continue
    return clean(item.detail || item.title)
  }
  return ''
}

function terminalDisplayContent(content: string, tone: TaskTone, reason = ''): string {
  if (tone !== 'canceled' || !reason) return content
  const normalizedContent = content.replace(/\s+/g, '')
  const normalizedReason = reason.replace(/\s+/g, '')
  if (normalizedContent.includes(normalizedReason) || normalizedReason.includes(normalizedContent)) return content
  if (/^任务已(停止|取消)[。.!！]*$/.test(content.trim())) {
    return `原因：${reason.replace(/[。.!！]+$/, '')}。`
  }
  return content
}

function taskIdForGroup(messages: ChatMessage[]): string {
  for (const message of messages) {
    const taskId = taskIdOf(message) || clean(message.source_task_id ?? message.sourceTaskId ?? '')
    if (taskId) return taskId
  }
  return ''
}

function statusForTaskGroup(
  task: TaskState | null,
  isDone: boolean,
  resultMsg: ChatMessage | undefined,
): { tone: TaskTone; label: string } {
  if (!isDone) return statusForTask(task)
  const content = resultMsg ? messageText(resultMsg) : ''
  const status = clean(resultMsg?.task_status ?? resultMsg?.taskStatus ?? task?.status ?? '').toLowerCase()
  const tone = taskResultTone(status, content)
  if (tone === 'canceled') {
    return { tone: 'canceled', label: status === 'interrupted' ? '已中断' : '任务已停止' }
  }
  if (tone === 'failed') {
    return { tone: 'failed', label: '任务失败' }
  }
  return { tone: 'done', label: '任务完成' }
}

function taskThreadSummary(
  timeline: ReturnType<typeof buildTaskTimeline>,
  assistantNoteCount: number,
  taskId: string,
  shortTaskId: string,
): string {
  const base = timelineSummary(timeline, taskId, shortTaskId)
  if (!assistantNoteCount) return base
  const noteSummary = `${assistantNoteCount} 条公开回复`
  if (!base) return [noteSummary, shortTaskId || taskId].filter(Boolean).join(' · ')
  return `${base} · ${noteSummary}`
}

function publicAssistantTimelineItems(timeline: ReturnType<typeof buildTaskTimeline>): TimelineItem[] {
  return assistantTimelineItemsFromTimeline(timeline).filter((item) => !isStatusEchoProgressText(item.detail ?? ''))
}

function publicAssistantPreviewItems(items: TimelineItem[], maxCount: number): TimelineItem[] {
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

function progressSurfaceItems(stage: TaskTimelineStage, assistantItems: TimelineItem[]): ProgressSurfaceItem[] {
  const items: ProgressSurfaceItem[] = []
  const stageItem = stageSurfaceItem(stage)
  if (stageItem) items.push(stageItem)
  for (const item of assistantItems) {
    const detail = clean(item.detail ?? '')
    if (!detail) continue
    items.push({
      id: item.id,
      detail,
      tone: item.tone,
    })
  }
  return dedupeProgressSurfaceItems(items)
}

function stageSurfaceItem(stage: TaskTimelineStage): ProgressSurfaceItem | null {
  if (!shouldSurfaceStage(stage)) return null
  const detail = surfaceStageDetail(stage)
  const title = surfaceStageTitle(stage)
  if (!detail && !title) return null
  return {
    id: `stage-${stage.key}`,
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
    const text = clean(item.detail)
    if (!text) continue
    if (deduped.some((existing) => clean(existing.detail) === text)) continue
    deduped.push({ ...item, detail: text })
  }
  return deduped
}

function assistantTimelineItemsFromTimeline(timeline: ReturnType<typeof buildTaskTimeline>): TimelineItem[] {
  return timeline.items.filter((item) =>
    !!(item.detail ?? '').trim()
    && (
      item.event?.type === 'assistant_message'
      || item.event?.type === 'assistant_chunk'
      || (item.message as Record<string, unknown> | undefined)?.assistant_progress_event === true
    )
  )
}

function taskRequestText(message: ChatMessage | undefined): string {
  if (!message) return ''
  return messageText(message)
    .replace(/^发起\s*AI\s*开发任务[：:]\s*/i, '')
    .trim()
}

function userDisplayName(
  message: ChatMessage | undefined,
  user: Props['user'],
): { name: string; avatarDataUrl: string } {
  const name = clean(
    message?.sender_name
    ?? message?.senderName
    ?? message?.sender_account
    ?? message?.senderAccount
    ?? user?.nickname
    ?? user?.account
    ?? '',
  ) || '我'
  return {
    name,
    avatarDataUrl: clean(
      message?.sender_avatar_data_url
      ?? message?.senderAvatarDataUrl
      ?? message?.avatar_data_url
      ?? message?.avatarDataUrl
      ?? user?.avatar_data_url
      ?? '',
    ),
  }
}

function messageTime(message: ChatMessage | undefined): string {
  if (!message) return ''
  const value = message.created_at ?? message.createdAt
  return value ? formatTime(value) : ''
}

function processDurationLabel(messages: ChatMessage[], terminal: boolean): string {
  const timestamps = messages
    .map((message) => timestampMs(message))
    .filter((value): value is number => Number.isFinite(value))
  if (timestamps.length < 2) return ''
  const start = Math.min(...timestamps)
  const end = terminal ? Math.max(...timestamps) : Date.now()
  const seconds = Math.max(0, Math.round((end - start) / 1000))
  if (seconds < 1) return ''
  if (seconds < 60) return `${seconds}s`
  const minutes = Math.floor(seconds / 60)
  const restSeconds = seconds % 60
  if (minutes < 60) return restSeconds ? `${minutes}m ${restSeconds}s` : `${minutes}m`
  const hours = Math.floor(minutes / 60)
  const restMinutes = minutes % 60
  return restMinutes ? `${hours}h ${restMinutes}m` : `${hours}h`
}

function timestampMs(message: ChatMessage): number {
  const value = message.created_at ?? message.createdAt
  if (!value) return Number.NaN
  const timestamp = Date.parse(String(value))
  return Number.isFinite(timestamp) ? timestamp : Number.NaN
}

function latestMessageOfKind(messages: ChatMessage[], kind: string): ChatMessage | undefined {
  for (let index = messages.length - 1; index >= 0; index--) {
    const message = messages[index]
    if (messageKind(message) === kind) return message
  }
  return undefined
}

function firstMessageMatching(
  messages: ChatMessage[],
  predicate: (message: ChatMessage) => boolean,
): ChatMessage | undefined {
  for (const message of messages) {
    if (predicate(message)) return message
  }
  return undefined
}

function latestMessageMatching(
  messages: ChatMessage[],
  predicate: (message: ChatMessage) => boolean,
): ChatMessage | undefined {
  for (let index = messages.length - 1; index >= 0; index--) {
    const message = messages[index]
    if (predicate(message)) return message
  }
  return undefined
}

function isUserTaskMessage(message: ChatMessage): boolean {
  return ['user', 'human'].includes(messageKind(message))
}

function isAssistantTaskMessage(message: ChatMessage): boolean {
  return ['assistant', 'ai', 'bot'].includes(messageKind(message))
}

function isTerminalAssistantTaskMessage(message: ChatMessage): boolean {
  if (!isAssistantTaskMessage(message)) return false
  if (!taskIdOf(message)) return false
  const status = String(message.task_status ?? message.taskStatus ?? '').toLowerCase()
  return ['done', 'failed', 'error', 'canceled', 'cancelled', 'interrupted'].includes(status)
}

function isFallbackAssistantTaskReply(message: ChatMessage, taskId: string): boolean {
  if (!taskId || !isAssistantTaskMessage(message)) return false
  if (isAssistantProgressNote(message)) return false
  return taskIdOf(message) === taskId
}

function isAssistantProgressNote(message: ChatMessage): boolean {
  return (message as Record<string, unknown>).assistant_progress_event === true
    && !isStatusEchoProgressText(messageText(message))
}

function latestVisibleProgress(messages: ChatMessage[]): ChatMessage | undefined {
  const progress = messages.filter((m) => messageKind(m) === 'ai_progress')
  for (let index = progress.length - 1; index >= 0; index--) {
    const message = progress[index]
    const text = messageText(message)
    if (!text || text.includes('正在处理中…') || text.startsWith('AI 还在')) continue
    return { ...message, kind: 'ai_result' }
  }
  return undefined
}

function isTerminalTaskMessage(message: ChatMessage): boolean {
  if (messageKind(message) === 'ai_result') return true
  if (isTerminalAssistantTaskMessage(message)) return true
  const status = String(message.task_status ?? message.taskStatus ?? '').toLowerCase()
  return ['done', 'failed', 'error', 'canceled', 'cancelled', 'interrupted'].includes(status)
}

