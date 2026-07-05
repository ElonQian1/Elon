/**
 * DevTaskGroup — 会话里的任务线程
 *
 * 将同一 task_id 的用户请求、过程消息和最终回复连成一个对话段：
 *  - 用户请求显示为右侧气泡
 *  - 最终回复显示为左侧 AI 气泡，避免被过程卡片淹没
 *  - 中间回复片段和命令按发生顺序穿插在过程面板中，任务结束后再折叠
 */
import { memo, useState, useEffect, useRef } from 'react'
import { ChevronDown, ChevronRight, StopCircle } from 'lucide-react'
import TaskTimeline from './TaskTimeline'
import MarkdownContent from '../markdown/MarkdownContent'
import UserAvatar from '../shell/UserAvatar'
import { clean, formatTime } from '../../lib/utils'
import { messageKind, messageText, shortId, statusForTask, taskIdOf, taskIsTerminal, taskResultTone } from './devTaskUtils'
import { buildTaskTimeline, timelineSummary } from './taskTimelineModel'
import type { ChatMessage, TaskContext, TaskState, TaskTone } from './types'
import styles from './DevTaskGroup.module.css'

interface Props {
  messages: ChatMessage[]
  taskContext: TaskContext
  user?: { nickname?: string; account?: string; avatar_data_url?: string | null } | null
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

function DevTaskGroup({ messages, taskContext, user, onCancel, onApprove }: Props) {
  const taskId  = taskIdForGroup(messages)
  const task    = taskId ? (taskContext.tasks.get(taskId) ?? null) : null
  const userMsg = firstMessageMatching(messages, isUserTaskMessage)
  const terminalAssistantMsg = latestMessageMatching(messages, isTerminalAssistantTaskMessage)
  const fallbackAssistantMsg = latestMessageMatching(messages, (message) => isFallbackAssistantTaskReply(message, taskId))
  const explicitResultMsg = latestMessageOfKind(messages, 'ai_result') ?? terminalAssistantMsg ?? fallbackAssistantMsg
  const isDone  = taskIsTerminal(task) || !!explicitResultMsg || messages.some(isTerminalTaskMessage)

  // 任务完成后默认折叠；从历史加载的已完成任务也默认折叠
  const [collapsed, setCollapsed] = useState(isDone)
  const prevDone = useRef(isDone)
  const userCollapseOverride = useRef(false)

  // 任务从"运行中"变为"完成"时自动折叠（延迟一下让用户看到结果）
  useEffect(() => {
    if (!prevDone.current && isDone) {
      const t = setTimeout(() => setCollapsed(true), 800)
      prevDone.current = true
      return () => clearTimeout(t)
    }
  }, [isDone])

  const headerMsg   = messages.find((m) => messageKind(m) === 'ai_task')
  const resultMsg   = explicitResultMsg ?? (isDone ? latestVisibleProgress(messages) : undefined)
  const progressMsgs = messages.filter((m) => messageKind(m) === 'ai_progress' || isAssistantProgressNote(m))
  const assistantNotes = messages.filter(isAssistantProgressNote)
  const timeline = buildTaskTimeline(progressMsgs, resultMsg, {
    assistantNoteCount: assistantNotes.length,
  })
  const progressCount = timeline.visibleStepCount
  const status = statusForTaskGroup(task, isDone, resultMsg)
  const request = taskRequestText(userMsg) || task?.request || taskRequestText(headerMsg)
  const hasProgressDetails = progressCount > 0
  const showProcessingBubble = !isDone && !resultMsg && !timeline.coverage.assistantEvent
  const tone = status.tone
  const processSummary = taskThreadSummary(timeline, assistantNotes.length, taskId, taskId ? shortId(taskId) : '')
  const canCancel = !!taskId && !isDone && !!onCancel
  const requestAuthor = userDisplayName(userMsg, user)
  const requestTime = messageTime(userMsg) || messageTime(headerMsg)
  const hideCompletedProcessPanel = isDone && tone === 'done' && collapsed

  useEffect(() => {
    if (userCollapseOverride.current || isDone) return
    if (timeline.coverage.assistantEvent) {
      setCollapsed(false)
      return
    }
    if (progressCount <= 3) return
    setCollapsed(true)
  }, [isDone, progressCount, timeline.coverage.assistantEvent])

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

  return (
    <div className={[styles.thread, styles[`tone_${tone}`] ?? ''].join(' ')}>
      {request && (
        <div className={styles.userTurn}>
          <div className={styles.userBody}>
            <div className={styles.userMeta}>
              <strong>{requestAuthor.name}</strong>
              {requestTime && <span>{requestTime}</span>}
            </div>
            <div className={styles.userBubble}>{request}</div>
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

      {showProcessingBubble && <TaskProcessingBubble time={messageTime(headerMsg) || requestTime} />}

      {resultMsg && <TaskAssistantBubble message={resultMsg} tone={tone} label={replyLabelForTone(tone)} />}

      {(hasProgressDetails || !isDone) && (
        <div className={[styles.processPanel, hideCompletedProcessPanel ? styles.processPanelDormant : ''].join(' ')}>
          {hasProgressDetails ? (
            <button
              type="button"
              className={styles.processToggle}
              onClick={() => {
                userCollapseOverride.current = true
                setCollapsed((c) => !c)
              }}
              aria-expanded={!collapsed}
            >
              <span className={styles.processDot} />
              <span className={styles.processLabel}>{status.label}</span>
              <span className={styles.processStage} data-tone={timeline.stage.tone}>
                {timeline.stage.label}{timeline.stage.meta ? ` · ${timeline.stage.meta}` : ''}
              </span>
              <span className={styles.processMeta}>
                {collapsed ? `查看 ${processSummary || `${progressCount} 步过程`}` : `收起过程${processSummary ? ` · ${processSummary}` : ''}`}
              </span>
              <span className={styles.toggleArrow} aria-hidden="true">
                {collapsed ? <ChevronRight size={14} /> : <ChevronDown size={14} />}
              </span>
            </button>
          ) : (
            <div className={styles.processStatic}>
              <span className={styles.processDot} />
              <span>{status.label}</span>
              {taskId && <em>{shortId(taskId)}</em>}
            </div>
          )}

          {canCancel && (
            <button
              type="button"
              className={styles.processCancel}
              onClick={() => {
                if (window.confirm('停止这个任务？')) onCancel?.(taskId)
              }}
            >
              <StopCircle size={13} />
              <span>停止</span>
            </button>
          )}

          {!collapsed && (
            <div className={styles.processBody}>
              <TaskTimeline
                model={timeline}
                taskContext={taskContext}
                onCancel={onCancel}
                onApprove={onApprove}
              />
            </div>
          )}
        </div>
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
  && prev.onCancel === next.onCancel
  && prev.onApprove === next.onApprove
)

function TaskAssistantBubble({ message, tone, label }: { message: ChatMessage; tone: TaskTone; label: string }) {
  const content = messageText(message)
  if (!content) return null
  const failed = tone === 'failed'
  const canceled = tone === 'canceled'
  const hasMarkdown = /[#*`\[\]>|]/.test(content)
  const time = messageTime(message)

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
          {hasMarkdown ? <MarkdownContent content={content} copy /> : content}
        </div>
      </div>
    </div>
  )
}

function TaskProcessingBubble({ time }: { time?: string }) {
  return (
    <div className={styles.assistantTurn}>
      <div className={styles.assistantAvatar}>AI</div>
      <div className={styles.assistantBody}>
        {time && (
          <div className={styles.assistantMeta}>
            <strong>一龙</strong>
            <span>{time}</span>
          </div>
        )}
        <div className={styles.processingBubble}>
          <span>AI 正在处理</span>
          <div className={styles.processingDots} aria-hidden="true">
            <span /><span /><span />
          </div>
        </div>
      </div>
    </div>
  )
}

function replyLabelForTone(tone: TaskTone): string {
  if (tone === 'failed') return '任务失败'
  if (tone === 'canceled') return '任务已停止'
  return '最终回复'
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

