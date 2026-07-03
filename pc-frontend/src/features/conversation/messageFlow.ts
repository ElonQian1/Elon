import { clean } from '../../lib/utils'
import type { Message } from './types'

const TASK_PROCESS_KINDS = new Set(['ai_task', 'ai_progress', 'ai_result'])
const CONVERSATION_USER_TASK_ROLES = new Set(['user', 'human'])
const CONVERSATION_ASSISTANT_TASK_ROLES = new Set(['assistant', 'ai', 'bot'])
const TERMINAL_TASK_STATUSES = new Set(['done', 'failed', 'error', 'canceled', 'cancelled', 'interrupted'])
const ASSISTANT_PROGRESS_EVENT_TYPES = new Set(['assistant_message', 'assistant_chunk'])
const ASSISTANT_PROGRESS_FLAG = 'assistant_progress_event'

export type SingleMessageGroup = {
  type: 'single'
  message: Message
  grouped: boolean
  key: string
}

export type TaskMessageGroup = {
  type: 'task'
  taskId: string
  messages: Message[]
  key: string
}

export type MessageGroup = SingleMessageGroup | TaskMessageGroup

export function messageKind(message: Message): string {
  return clean(message.kind ?? message.role ?? (message as Record<string, unknown>).message_kind ?? '').toLowerCase()
}

export function messageTaskId(message: Message): string {
  return clean(message.task_id ?? message.taskId ?? '')
}

export function messageConversationId(message: Message): string {
  return clean(message.conversation_id ?? (message as Record<string, unknown>).conversationId ?? '')
}

export function isTaskProcessMessage(message: Message): boolean {
  return isTaskBackfillMessage(message) && !parseAssistantProgressEvent(message)
}

export function isTerminalTaskStatus(status: unknown): boolean {
  return TERMINAL_TASK_STATUSES.has(String(status ?? '').toLowerCase())
}

export function buildTaskProcessMessageMap(messageSources: Message[][]): Map<string, Message[]> {
  const byTaskId = new Map<string, Message[]>()
  const seen = new Set<string>()

  for (const messages of messageSources) {
    for (const message of messages) {
      if (!isTaskBackfillMessage(message)) continue
      const taskId = messageTaskId(message)
      const key = messageIdentity(message)
      if (seen.has(key)) continue
      seen.add(key)
      const list = byTaskId.get(taskId)
      if (list) list.push(message)
      else byTaskId.set(taskId, [message])
    }
  }

  return byTaskId
}

export function buildDisplayMessages(input: {
  sessionView: string | 'new' | null
  channelMessages: Message[]
  conversationMessages: Message[]
  conversationLoading: boolean
  taskMessagesById: Map<string, Message[]>
}): Message[] {
  const {
    sessionView,
    channelMessages,
    conversationMessages,
    conversationLoading,
    taskMessagesById,
  } = input

  if (!sessionView) return materializeDisplayMessages(channelMessages)
  if (sessionView === 'new') return []

  if (conversationMessages.length > 0 || conversationLoading) {
    return materializeDisplayMessages(mergeConversationMessagesWithTaskProcess(conversationMessages, taskMessagesById))
  }

  const byConversation = channelMessages.filter((message) => messageConversationId(message) === sessionView)
  if (byConversation.length > 0) {
    return materializeDisplayMessages(mergeConversationMessagesWithTaskProcess(byConversation, taskMessagesById))
  }

  return materializeDisplayMessages(channelMessages.filter((message) => messageTaskId(message) === sessionView))
}

export function hasRunningTask(messages: Message[]): boolean {
  const taskIds = new Set<string>()
  const doneIds = new Set<string>()
  let latestOpenTaskId = ''

  for (const message of messages) {
    const kind = messageKind(message)
    const taskId = messageTaskId(message)
    if (taskId && (kind === 'ai_task' || isConversationUserTaskMessage(message))) {
      taskIds.add(taskId)
      latestOpenTaskId = taskId
    }
    if (taskId && isTaskTerminalMessage(message)) {
      taskIds.add(taskId)
      doneIds.add(taskId)
      if (latestOpenTaskId === taskId) latestOpenTaskId = ''
      continue
    }
    if (!taskId && latestOpenTaskId && isConversationAssistantTaskMessage(message) && !isAssistantProgressDisplay(message)) {
      doneIds.add(latestOpenTaskId)
      latestOpenTaskId = ''
    }
  }

  for (const taskId of taskIds) {
    if (!doneIds.has(taskId)) return true
  }
  return false
}

export function buildMessageGroups(messages: Message[], taskFlowEnabled: boolean): MessageGroup[] {
  const groups: MessageGroup[] = []
  let activeTaskGroup: TaskMessageGroup | null = null

  messages.forEach((message, index) => {
    const taskId = taskThreadId(message)
    if (taskFlowEnabled && taskId) {
      if (activeTaskGroup && activeTaskGroup.taskId === taskId) {
        activeTaskGroup.messages.push(message)
      } else {
        const group: TaskMessageGroup = {
          type: 'task',
          taskId,
          messages: [message],
          key: `task-${taskId}-${index}`,
        }
        activeTaskGroup = group
        groups.push(group)
      }
      return
    }

    activeTaskGroup = null
    groups.push({
      type: 'single',
      message,
      grouped: isGroupedWithPrevious(messages, index),
      key: clean(message.id) || String(index),
    })
  })

  return groups
}

export function containsTaskProcess(messages: Message[]): boolean {
  return messages.some(isTaskProcessMessage)
}

function isTaskTerminalMessage(message: Message): boolean {
  return messageKind(message) === 'ai_result'
    || isConversationAssistantTaskMessage(message)
    || isTerminalTaskStatus(message.task_status ?? message.taskStatus)
}

function mergeConversationMessagesWithTaskProcess(
  conversationMessages: Message[],
  taskMessagesById: Map<string, Message[]>,
): Message[] {
  const merged: Message[] = []
  const seen = new Set<string>()
  const insertedTaskIds = new Set<string>()
  let activeConversationTaskId = ''

  for (const message of conversationMessages) {
    const taskId = messageTaskId(message)
    const taskMessages = taskId ? taskMessagesById.get(taskId) : undefined

    if (taskId && taskMessages?.length && isConversationUserTaskMessage(message)) {
      activeConversationTaskId = taskId
      pushUniqueMessage(merged, seen, message)
      if (!insertedTaskIds.has(taskId)) {
        for (const taskMessage of taskMessages) pushUniqueMessage(merged, seen, taskMessage)
        insertedTaskIds.add(taskId)
      }
      continue
    }

    if (taskId && taskMessages?.length && isConversationAssistantTaskMessage(message)) {
      if (!insertedTaskIds.has(taskId)) {
        for (const taskMessage of taskMessages) pushUniqueMessage(merged, seen, taskMessage)
        insertedTaskIds.add(taskId)
      }
      if (!taskMessages.some((taskMessage) => messageKind(taskMessage) === 'ai_result')) {
        pushUniqueMessage(merged, seen, assistantReplyAsTaskResult(message, taskId))
      }
      if (activeConversationTaskId === taskId) activeConversationTaskId = ''
      continue
    }

    if (!taskId && activeConversationTaskId && isConversationAssistantTaskMessage(message)) {
      const activeTaskMessages = taskMessagesById.get(activeConversationTaskId) ?? []
      if (activeTaskMessages.length > 0 && !activeTaskMessages.some((taskMessage) => messageKind(taskMessage) === 'ai_result')) {
        pushUniqueMessage(merged, seen, assistantReplyAsTaskResult(message, activeConversationTaskId))
        activeConversationTaskId = ''
        continue
      }
      activeConversationTaskId = ''
      continue
    }

    pushUniqueMessage(merged, seen, message)
  }

  return merged
}

function isConversationUserTaskMessage(message: Message): boolean {
  return CONVERSATION_USER_TASK_ROLES.has(messageKind(message))
}

function isConversationAssistantTaskMessage(message: Message): boolean {
  return CONVERSATION_ASSISTANT_TASK_ROLES.has(messageKind(message))
}

function isAssistantProgressDisplay(message: Message): boolean {
  return (message as Record<string, unknown>)[ASSISTANT_PROGRESS_FLAG] === true
}

function taskThreadId(message: Message): string {
  const taskId = messageTaskId(message)
  if (taskId && (isTaskBackfillMessage(message) || isConversationUserTaskMessage(message) || isConversationAssistantTaskMessage(message))) {
    return taskId
  }
  if (isAssistantProgressDisplay(message)) {
    return clean((message as Record<string, unknown>).source_task_id ?? (message as Record<string, unknown>).sourceTaskId ?? '')
  }
  return ''
}

function assistantReplyAsTaskResult(message: Message, taskId: string): Message {
  return {
    ...message,
    kind: 'ai_result',
    role: 'assistant',
    task_id: taskId,
    taskId,
    task_status: message.task_status ?? message.taskStatus ?? 'done',
    taskStatus: message.task_status ?? message.taskStatus ?? 'done',
  }
}

function pushUniqueMessage(target: Message[], seen: Set<string>, message: Message) {
  const key = messageIdentity(message)
  if (seen.has(key)) return
  seen.add(key)
  target.push(message)
}

function isGroupedWithPrevious(messages: Message[], index: number): boolean {
  if (index <= 0 || index >= messages.length) return false
  const current = messages[index]
  const previous = messages[index - 1]
  if (!current || !previous) return false
  if (isTaskProcessMessage(current) || isTaskProcessMessage(previous)) return false

  const currentKind = messageKind(current)
  const previousKind = messageKind(previous)
  if (currentKind !== previousKind) return false
  if (currentKind === 'user' || currentKind === 'human' || currentKind === 'discussion') {
    const currentUserId = clean(current.user_id ?? (current as Record<string, unknown>).userId ?? '')
    const previousUserId = clean(previous.user_id ?? (previous as Record<string, unknown>).userId ?? '')
    return currentUserId !== '' && currentUserId === previousUserId
  }

  return true
}

function messageIdentity(message: Message): string {
  return clean(message.id) || [
    messageKind(message),
    messageTaskId(message),
    clean(message.created_at),
    clean(message.content ?? message.text ?? ''),
  ].join('|')
}

function isTaskBackfillMessage(message: Message): boolean {
  return TASK_PROCESS_KINDS.has(messageKind(message)) && !!messageTaskId(message)
}

function materializeDisplayMessages(messages: Message[]): Message[] {
  const out: Message[] = []
  for (const message of messages) {
    const assistantMessage = assistantProgressAsConversationMessage(message)
    if (assistantMessage) {
      out.push(assistantMessage)
      continue
    }
    if (parseAssistantProgressEvent(message)) continue
    out.push(message)
  }
  return out
}

function assistantProgressAsConversationMessage(message: Message): Message | null {
  const event = parseAssistantProgressEvent(message)
  if (!event) return null
  const text = clean(event.text)
  if (!text) return null
  return {
    ...message,
    id: `assistant-progress-${clean(message.id) || messageIdentity(message)}`,
    kind: 'assistant',
    role: 'assistant',
    content: text,
    text,
    task_id: undefined,
    taskId: undefined,
    task_status: undefined,
    taskStatus: undefined,
    task_error: undefined,
    taskError: undefined,
    model_used: clean(event.model_used) || message.model_used,
    node_id: clean(event.node_id) || message.node_id,
    stream_id: clean(event.stream_id) || message.stream_id,
    assistant_progress_event: true,
    source_task_id: messageTaskId(message),
  }
}

function parseAssistantProgressEvent(message: Message): Record<string, unknown> | null {
  if (messageKind(message) !== 'ai_progress') return null
  const content = clean(message.content ?? message.text ?? '')
  if (!content.startsWith('{')) return null
  try {
    const event = JSON.parse(content) as Record<string, unknown>
    return ASSISTANT_PROGRESS_EVENT_TYPES.has(clean(event.type)) ? event : null
  } catch {
    return null
  }
}
