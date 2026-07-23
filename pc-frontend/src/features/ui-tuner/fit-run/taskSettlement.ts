import { api } from '../../../api/client'
import { clean } from '../../../lib/utils'
import {
  parseAiWritebackReceipt,
  type AiWritebackReceipt,
} from '../source-preview/aiWritebackReceipt'

const SUCCEEDED_STATUSES = new Set(['done', 'completed', 'success', 'succeeded'])
const FAILED_STATUSES = new Set(['failed', 'error', 'canceled', 'cancelled', 'interrupted'])

interface TaskSnapshotResponse {
  task?: {
    id?: string
    status?: string
    error?: string | null
  } | null
  messages?: Array<{
    task_id?: string | null
    taskId?: string | null
    kind?: string
    content?: string
  }>
}

export interface AiTaskSettlement {
  taskId: string
  succeeded: boolean
  receipt?: AiWritebackReceipt
  error?: string
}

export function classifyAiTaskSettlement(
  taskId: string,
  snapshot: TaskSnapshotResponse,
): AiTaskSettlement | null {
  const status = clean(snapshot.task?.status).toLowerCase()
  const error = clean(snapshot.task?.error)
  if (SUCCEEDED_STATUSES.has(status)) {
    return {
      taskId,
      succeeded: true,
      receipt: taskWritebackReceipt(taskId, snapshot.messages ?? []),
    }
  }
  if (FAILED_STATUSES.has(status) || error) {
    return {
      taskId,
      succeeded: false,
      error: error || `AI 任务以 ${status || '未知错误'} 结束`,
    }
  }
  return null
}

export async function loadAiTaskSettlement(
  projectId: string,
  channelId: string,
  taskId: string,
): Promise<AiTaskSettlement | null> {
  const snapshot = await api.get<TaskSnapshotResponse>(
    `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/ai-tasks/${encodeURIComponent(taskId)}/snapshot?limit=20`,
  )
  return classifyAiTaskSettlement(taskId, snapshot)
}

function taskWritebackReceipt(
  taskId: string,
  messages: NonNullable<TaskSnapshotResponse['messages']>,
): AiWritebackReceipt | undefined {
  for (const message of [...messages].reverse()) {
    const messageTaskId = clean(message.task_id ?? message.taskId)
    if (messageTaskId && messageTaskId !== taskId) continue
    const receipt = parseAiWritebackReceipt(clean(message.content))
    if (receipt) return receipt
  }
  return undefined
}
