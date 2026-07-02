/**
 * DevTaskGroup — 会话里的任务线程
 *
 * 将同一 task_id 的用户请求、过程消息和最终回复连成一个对话段：
 *  - 用户请求显示为右侧气泡
 *  - 工具调用 / 命令 / 过程默认可折叠
 *  - 最终回复显示为左侧 AI 气泡，避免被过程卡片淹没
 */
import { memo, useState, useEffect, useRef } from 'react'
import { ChevronDown, ChevronRight } from 'lucide-react'
import TaskTimeline from './TaskTimeline'
import MarkdownContent from '../markdown/MarkdownContent'
import { messageKind, messageText, shortId, statusForTask, taskIdOf, taskIsTerminal } from './devTaskUtils'
import { buildTaskTimeline, timelineSummary } from './taskTimelineModel'
import type { ChatMessage, TaskContext, TaskTone } from './types'
import styles from './DevTaskGroup.module.css'

interface Props {
  messages: ChatMessage[]
  taskContext: TaskContext
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

function DevTaskGroup({ messages, taskContext, onCancel, onApprove }: Props) {
  const taskId  = taskIdOf(messages[0]) || ''
  const task    = taskId ? (taskContext.tasks.get(taskId) ?? null) : null
  const userMsg = firstMessageMatching(messages, isUserTaskMessage)
  const assistantMsg = latestMessageMatching(messages, isAssistantTaskMessage)
  const explicitResultMsg = latestMessageOfKind(messages, 'ai_result') ?? assistantMsg
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
  const timeline = buildTaskTimeline(progressMsgs, resultMsg)
  const progressCount = timeline.visibleStepCount
  const status = statusForTask(task)
  const request = taskRequestText(userMsg) || task?.request || taskRequestText(headerMsg)
  const hasProgressDetails = progressCount > 0
  const tone = status.tone
  const processSummary = timelineSummary(timeline, taskId, taskId ? shortId(taskId) : '')

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
  if (isAssistantTaskMessage(message) && taskIdOf(message)) return true
  const status = String(message.task_status ?? message.taskStatus ?? '').toLowerCase()
  return ['done', 'failed', 'error', 'canceled', 'cancelled', 'interrupted'].includes(status)
}
