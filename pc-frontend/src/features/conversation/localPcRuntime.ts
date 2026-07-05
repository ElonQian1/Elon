import { nodeApi } from '../node/localNodeApi'
import type { NodeLifecycleReport } from '../node/types'

const DIRECT_PC_CLI_STORAGE_KEY = 'elon_pc_project_direct_pc_cli'

export interface LocalNodeStatus {
  agent_id?: string
  owner_user_id?: string
  device_name?: string
  version?: string
  connected?: boolean
  logged_in?: boolean
  last_event?: string
  lifecycle?: NodeLifecycleReport | null
  codex_cli?: { available?: boolean; logged_in?: boolean; status?: string }
}

interface FullAccessGrant {
  project_id?: string
  workspace_path?: string
}

interface FullAccessGrantListResponse {
  grants?: FullAccessGrant[]
}

interface EnsureLocalFullAccessGrantInput {
  adminUrl: string
  projectId: string
  projectName?: string
  workspacePath: string
  runtimePermission?: string
  useLocalRouteA: boolean
}

type LocalGrantRequest = (path: string, options?: RequestInit) => Promise<unknown>

interface LocalFullAccessGrantDependencies {
  request?: LocalGrantRequest
  confirm?: (message: string) => boolean
}

export type LocalFullAccessGrantResult = 'not_required' | 'already_granted' | 'granted'

export function initialDirectPcCliFromStorage(storage?: Storage | null): boolean {
  try {
    return storage?.getItem(DIRECT_PC_CLI_STORAGE_KEY) === '1'
  } catch {
    return false
  }
}

export function persistDirectPcCliSelection(storage: Storage | null | undefined, enabled: boolean): void {
  try {
    if (enabled) {
      storage?.setItem(DIRECT_PC_CLI_STORAGE_KEY, '1')
    } else {
      storage?.removeItem(DIRECT_PC_CLI_STORAGE_KEY)
    }
  } catch {
    // Ignore blocked storage; the selected value still works for the current session.
  }
}

export function normalizeLocalWorkspacePath(value: string): string {
  return value.trim().replace(/\//g, '\\').replace(/\\+$/, '').toLocaleLowerCase('en-US')
}

export function requiresLocalFullAccessGrant(
  runtimePermission: string | undefined,
  useLocalRouteA: boolean,
): boolean {
  if (!useLocalRouteA) return false
  const permission = String(runtimePermission ?? '').trim().toLowerCase()
  return permission === 'full_access' || permission === 'danger_full_access'
}

export async function ensureLocalFullAccessGrant(
  input: EnsureLocalFullAccessGrantInput,
  dependencies: LocalFullAccessGrantDependencies = {},
): Promise<LocalFullAccessGrantResult> {
  if (!requiresLocalFullAccessGrant(input.runtimePermission, input.useLocalRouteA)) {
    return 'not_required'
  }

  const projectId = input.projectId.trim()
  const workspacePath = input.workspacePath.trim()
  if (!projectId || !workspacePath) {
    throw new Error('当前项目缺少本机目录，无法确认完全访问。请先在项目工作区绑定这台电脑。')
  }

  const request: LocalGrantRequest = dependencies.request
    ?? ((path, options) => nodeApi(input.adminUrl, path, options))
  const grantList = await request('/api/full-access/grants') as FullAccessGrantListResponse
  const normalizedWorkspace = normalizeLocalWorkspacePath(workspacePath)
  const alreadyGranted = (grantList.grants ?? []).some((grant) => (
    grant.project_id === projectId
    && normalizeLocalWorkspacePath(String(grant.workspace_path ?? '')) === normalizedWorkspace
  ))
  if (alreadyGranted) return 'already_granted'

  const dangerMode = input.runtimePermission?.trim().toLowerCase() === 'danger_full_access'
  const permissionLabel = dangerMode ? '完整本机命令行' : '完全访问'
  const riskSummary = dangerMode
    ? '此模式允许本机 AI 执行任意命令，并读取或修改项目目录外的文件和系统设置。'
    : '此模式允许本机 Codex 在执行项目任务时使用完整 CLI 权限。'
  const projectName = input.projectName?.trim() || projectId
  const confirmation = [
    `项目“${projectName}”请求开启${permissionLabel}。`,
    '',
    `本机授权目录：${workspacePath}`,
    '',
    riskSummary,
    '是否确认将这个项目授权到上述目录，并继续发送任务？',
  ].join('\n')
  const confirmAccess = dependencies.confirm ?? ((message) => window.confirm(message))
  if (!confirmAccess(confirmation)) {
    throw new Error('已取消本机完全访问授权，本轮任务未发送。')
  }

  await request('/api/full-access/grants', {
    method: 'POST',
    body: JSON.stringify({
      project_id: projectId,
      workspace_path: workspacePath,
      confirm_full_access: true,
    }),
  })
  return 'granted'
}
