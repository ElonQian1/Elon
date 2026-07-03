/**
 * DevTaskGroup — 会话里的任务线程
 *
 * 将同一 task_id 的用户请求、过程消息和最终回复连成一个对话段：
 *  - 用户请求显示为右侧气泡
 *  - 工具调用 / 命令 / 过程默认可折叠
 *  - 最终回复显示为左侧 AI 气泡，避免被过程卡片淹没
 */
import { memo, useState, useEffect, useRef } from 'react'
import { ChevronDown, ChevronRight, ExternalLink, StopCircle } from 'lucide-react'
import TaskTimeline from './TaskTimeline'
import MarkdownContent from '../markdown/MarkdownContent'
import { clean } from '../../lib/utils'
import { messageKind, messageText, shortId, statusForTask, taskIdOf, taskIsTerminal } from './devTaskUtils'
import { buildTaskTimeline, timelineSummary } from './taskTimelineModel'
import type { ChatMessage, TaskContext, TaskState, TaskTone } from './types'
import styles from './DevTaskGroup.module.css'

interface Props {
  messages: ChatMessage[]
  taskContext: TaskContext
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

function DevTaskGroup({ messages, taskContext, onCancel, onApprove }: Props) {
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
  const progressMsgs = messages.filter((m) => messageKind(m) === 'ai_progress')
  const assistantNotes = messages.filter(isAssistantProgressNote)
  const timeline = buildTaskTimeline(progressMsgs, resultMsg, {
    assistantNoteCount: assistantNotes.length,
  })
  const progressCount = timeline.visibleStepCount + assistantNotes.length
  const status = statusForTaskGroup(task, isDone, resultMsg)
  const request = taskRequestText(userMsg) || task?.request || taskRequestText(headerMsg)
  const hasProgressDetails = progressCount > 0
  const tone = status.tone
  const processSummary = taskThreadSummary(timeline, assistantNotes.length, taskId, taskId ? shortId(taskId) : '')
  const codexThreadUri = codexThreadUriFor(messages)
  const canCancel = !!taskId && !isDone && !!onCancel

  useEffect(() => {
    if (!taskId || localStorage.getItem('elon_debug_task_timeline') !== '1') return
    console.info('[elon-task-timeline]', {
      taskId,
      status: status.label,
      done: isDone,
      coverage: timeline.coverage,
      codexThreadUri,
      steps: timeline.items.map((item) => ({
        kind: item.kind,
        tone: item.tone,
        title: item.title,
        type: item.event?.type,
        tool: item.event?.tool,
        meta: item.meta,
      })),
    })
  }, [taskId, status.label, isDone, processSummary, codexThreadUri])

  return (
    <div className={[styles.thread, styles[`tone_${tone}`] ?? ''].join(' ')}>
      {request && (
        <div className={styles.userTurn}>
          <div className={styles.userBubble}>{request}</div>
        </div>
      )}

      {(hasProgressDetails || !isDone) && (
        <div className={styles.processPanel}>
          {hasProgressDetails ? (
            <button
              type="button"
              className={styles.processToggle}
              onClick={() => setCollapsed((c) => !c)}
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

          {codexThreadUri && (
            <a className={styles.codexThreadLink} href={codexThreadUri} title={codexThreadUri}>
              <ExternalLink size={12} />
              <span>Codex 会话</span>
              <em>{shortThreadUri(codexThreadUri)}</em>
            </a>
          )}

          {!collapsed && (
            <div className={styles.processBody}>
              {assistantNotes.length > 0 && (
                <div className={styles.publicNotes}>
                  {assistantNotes.map((message, index) => (
                    <PublicAssistantNote key={clean(message.id ?? '') || index} message={message} />
                  ))}
                </div>
              )}
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

      {resultMsg && <TaskFinalReply message={resultMsg} tone={tone} />}
    </div>
  )
}

export default memo(DevTaskGroup, (prev, next) =>
  prev.messages === next.messages
  && prev.taskContext === next.taskContext
  && prev.onCancel === next.onCancel
  && prev.onApprove === next.onApprove
)

function PublicAssistantNote({ message }: { message: ChatMessage }) {
  const content = messageText(message)
  if (!content) return null
  const hasMarkdown = /[#*`\[\]>|]/.test(content)
  return (
    <div className={styles.publicNote}>
      <strong>{assistantNoteLabel(message)}</strong>
      {hasMarkdown ? <MarkdownContent content={content} copy /> : <span>{content}</span>}
    </div>
  )
}

function TaskFinalReply({ message, tone }: { message: ChatMessage; tone: TaskTone }) {
  const content = messageText(message)
  if (!content) return null
  const failed = tone === 'failed'
  const canceled = tone === 'canceled'
  const label = failed ? '任务失败' : canceled ? '任务已停止' : '最终回复'
  const hasMarkdown = /[#*`\[\]>|]/.test(content)

  return (
    <div className={styles.assistantTurn}>
      <div className={styles.assistantAvatar}>AI</div>
      <div className={styles.assistantBody}>
        <div className={styles.assistantMeta}>
          <strong>一龙</strong>
          <span>{label}</span>
        </div>
        <div className={[styles.assistantBubble, failed ? styles.replyFailed : canceled ? styles.replyCanceled : ''].join(' ')}>
          {hasMarkdown ? <MarkdownContent content={content} copy /> : content}
        </div>
      </div>
    </div>
  )
}

function taskIdForGroup(messages: ChatMessage[]): string {
  for (const message of messages) {
    const taskId = taskIdOf(message) || clean(message.source_task_id ?? message.sourceTaskId ?? '')
    if (taskId) return taskId
  }
  return ''
}

function assistantNoteLabel(message: ChatMessage): string {
  const label = clean(
    message.model_used
    ?? message.modelUsed
    ?? message.cli_name
    ?? message.cliName
    ?? '',
  )
  return label || 'AI CLI'
}

function statusForTaskGroup(
  task: TaskState | null,
  isDone: boolean,
  resultMsg: ChatMessage | undefined,
): { tone: TaskTone; label: string } {
  if (!isDone) return statusForTask(task)
  const content = resultMsg ? messageText(resultMsg) : ''
  const status = clean(resultMsg?.task_status ?? resultMsg?.taskStatus ?? task?.status ?? '').toLowerCase()
  if (['canceled', 'cancelled', 'interrupted'].includes(status) || /停止|取消|canceled|cancelled/i.test(content)) {
    return { tone: 'canceled', label: status === 'interrupted' ? '已中断' : '任务已停止' }
  }
  if (['failed', 'error'].includes(status) || /失败|错误|error|failed/i.test(content)) {
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

function codexThreadUriFor(messages: ChatMessage[]): string {
  for (let index = messages.length - 1; index >= 0; index--) {
    const value = clean(
      messages[index].codex_thread_uri
      ?? messages[index].codexThreadUri
      ?? messages[index].task_codex_thread_uri
      ?? messages[index].taskCodexThreadUri
      ?? '',
    )
    if (value) return value.startsWith('codex://threads/') ? value : `codex://threads/${value}`
    const threadId = clean(
      messages[index].task_codex_thread_id
      ?? messages[index].taskCodexThreadId
      ?? messages[index].codex_thread_id
      ?? messages[index].codexThreadId
      ?? '',
    )
    if (threadId) return threadId.startsWith('codex://threads/') ? threadId : `codex://threads/${threadId}`
  }
  return ''
}

function shortThreadUri(uri: string): string {
  const id = clean(uri).replace(/^codex:\/\/threads\//, '')
  if (!id) return ''
  return id.length > 16 ? `${id.slice(0, 8)}...${id.slice(-4)}` : id
}
