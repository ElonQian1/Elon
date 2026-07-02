import { clean } from '../../lib/utils'
import type { Message } from './types'

const TASK_PROCESS_KINDS = new Set(['ai_task', 'ai_progress', 'ai_result'])
const CONVERSATION_USER_TASK_ROLES = new Set(['user', 'human'])
const CONVERSATION_ASSISTANT_TASK_ROLES = new Set(['assistant', 'ai', 'bot'])
const TERMINAL_TASK_STATUSES = new Set(['done', 'failed', 'error', 'canceled', 'cancelled', 'interrupted'])

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
  return TASK_PROCESS_KINDS.has(messageKind(message)) && !!messageTaskId(message)
}

export function isTerminalTaskStatus(status: unknown): boolean {
  return TERMINAL_TASK_STATUSES.has(String(status ?? '').toLowerCase())
}

export function buildTaskProcessMessageMap(messageSources: Message[][]): Map<string, Message[]> {
  const byTaskId = new Map<string, Message[]>()
  const seen = new Set<string>()

  for (const messages of messageSources) {
    for (const message of messages) {
      if (!isTaskProcessMessage(message)) continue
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

  if (!sessionView) return channelMessages
  if (sessionView === 'new') return []

  if (conversationMessages.length > 0 || conversationLoading) {
    return mergeConversationMessagesWithTaskProcess(conversationMessages, taskMessagesById)
  }

  const byConversation = channelMessages.filter((message) => messageConversationId(message) === sessionView)
  if (byConversation.length > 0) {
    return mergeConversationMessagesWithTaskProcess(byConversation, taskMessagesById)
  }

  return channelMessages.filter((message) => messageTaskId(message) === sessionView)
}

export function hasRunningTask(messages: Message[]): boolean {
  const taskIds = new Set<string>()
  const doneIds = new Set<string>()

  for (const message of messages) {
    const kind = messageKind(message)
    const taskId = messageTaskId(message)
    if (!taskId) continue
    if (kind === 'ai_task') taskIds.add(taskId)
    if (kind === 'ai_result' || isTerminalTaskStatus(message.task_status ?? message.taskStatus)) {
      doneIds.add(taskId)
    }
  }

  for (const taskId of taskIds) {
    if (!doneIds.has(taskId)) return true
  }
  return false
}

export function buildMessageGroups(messages: Message[], taskFlowEnabled: boolean): MessageGroup[] {
  const groups: MessageGroup[] = []
  const taskGroupById = new Map<string, TaskMessageGroup>()

  messages.forEach((message, index) => {
    const taskId = messageTaskId(message)
    if (taskFlowEnabled && isTaskProcessMessage(message)) {
      const existing = taskGroupById.get(taskId)
      if (existing) {
        existing.messages.push(message)
      } else {
        const group: TaskMessageGroup = {
          type: 'task',
          taskId,
          messages: [message],
          key: `task-${taskId}-${index}`,
        }
        taskGroupById.set(taskId, group)
        groups.push(group)
      }
      return
    }

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

function mergeConversationMessagesWithTaskProcess(
  conversationMessages: Message[],
  taskMessagesById: Map<string, Message[]>,
): Message[] {
  const merged: Message[] = []
  const seen = new Set<string>()
  const insertedTaskIds = new Set<string>()

  for (const message of conversationMessages) {
    const taskId = messageTaskId(message)
    const taskMessages = taskId ? taskMessagesById.get(taskId) : undefined

    if (taskId && taskMessages?.length && isConversationUserTaskMessage(message)) {
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
        pushUniqueMessage(merged, seen, assistantMessageAsTaskResult(message))
      }
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

function assistantMessageAsTaskResult(message: Message): Message {
  return {
    ...message,
    id: `task-result-${clean(message.id) || messageIdentity(message)}`,
    kind: 'ai_result',
    role: undefined,
    task_status: clean(message.task_status ?? message.taskStatus) || 'done',
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
