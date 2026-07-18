import { safeNodeAdminUrl } from '../../lib/utils'
import { nodeApi } from '../node/localNodeApi'
import type {
  LocalFullAccessGrant,
  LocalTaskApprovalDecision,
  LocalTaskCreateInput,
} from './types'

const LOCAL_TASKS_PATH = '/api/local-tasks'

export async function listLocalFullAccessGrants(): Promise<LocalFullAccessGrant[]> {
  const response = await nodeApi(safeNodeAdminUrl(), '/api/full-access/grants') as {
    grants?: LocalFullAccessGrant[]
  }
  return Array.isArray(response.grants) ? response.grants : []
}

export function listLocalTasks(limit = 50): Promise<unknown> {
  return nodeApi(safeNodeAdminUrl(), `${LOCAL_TASKS_PATH}?limit=${clamp(limit, 1, 100)}`)
}

export function createLocalTask(input: LocalTaskCreateInput): Promise<unknown> {
  return nodeApi(safeNodeAdminUrl(), LOCAL_TASKS_PATH, {
    method: 'POST',
    body: JSON.stringify(input),
  }, 15_000)
}

export function getLocalTask(taskId: string, since = 0, limit = 200): Promise<unknown> {
  const query = new URLSearchParams({
    since: String(Math.max(0, Math.floor(since))),
    limit: String(clamp(limit, 1, 500)),
  })
  return nodeApi(
    safeNodeAdminUrl(),
    `${LOCAL_TASKS_PATH}/${encodeURIComponent(taskId)}?${query}`,
  )
}

export function cancelLocalTask(taskId: string): Promise<unknown> {
  return nodeApi(
    safeNodeAdminUrl(),
    `${LOCAL_TASKS_PATH}/${encodeURIComponent(taskId)}/cancel`,
    {
      method: 'POST',
      body: JSON.stringify({ source: 'pc_ui', reason: 'user_stop_button' }),
    },
  )
}

export function decideLocalTaskApproval(
  taskId: string,
  approvalId: string,
  decision: LocalTaskApprovalDecision,
): Promise<unknown> {
  return nodeApi(
    safeNodeAdminUrl(),
    `${LOCAL_TASKS_PATH}/${encodeURIComponent(taskId)}/tool-approvals/${encodeURIComponent(approvalId)}/decision`,
    {
      method: 'POST',
      body: JSON.stringify({ decision }),
    },
  )
}

function clamp(value: number, min: number, max: number): number {
  const normalized = Number.isFinite(value) ? Math.floor(value) : min
  return Math.max(min, Math.min(max, normalized))
}
