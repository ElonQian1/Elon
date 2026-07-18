import { api } from '../../../api/client'
import { clean } from '../../../lib/utils'

const SUCCEEDED_STATUSES = new Set(['done', 'completed', 'success', 'succeeded'])
const FAILED_STATUSES = new Set(['failed', 'error', 'canceled', 'cancelled', 'interrupted'])

interface TaskSnapshotResponse {
  task?: {
    id?: string
    status?: string
    error?: string | null
  } | null
}

export interface AiTaskSettlement {
  taskId: string
  succeeded: boolean
  error?: string
}

export function classifyAiTaskSettlement(
  taskId: string,
  snapshot: TaskSnapshotResponse,
): AiTaskSettlement | null {
  const status = clean(snapshot.task?.status).toLowerCase()
  const error = clean(snapshot.task?.error)
  if (SUCCEEDED_STATUSES.has(status)) return { taskId, succeeded: true }
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
    `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/ai-tasks/${encodeURIComponent(taskId)}/snapshot?limit=1`,
  )
  return classifyAiTaskSettlement(taskId, snapshot)
}
