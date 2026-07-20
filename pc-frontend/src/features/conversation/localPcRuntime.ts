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

interface LocalCloudProject {
  id?: string
  node_id?: string | null
  workspace_path?: string | null
}

interface LocalCloudProjectsResponse {
  node_id?: string
  projects?: LocalCloudProject[]
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

  const cloudProjects = await request('/api/cloud-projects') as LocalCloudProjectsResponse
  const currentNodeId = String(cloudProjects.node_id ?? '').trim()
  const projectIsBound = !!currentNodeId && (cloudProjects.projects ?? []).some((project) => (
    equivalentProjectId(String(project.id ?? ''), projectId)
    && String(project.node_id ?? '').trim() === currentNodeId
    && normalizeLocalWorkspacePath(String(project.workspace_path ?? '')) === normalizedWorkspace
  ))
  if (!projectIsBound) {
    const projectName = input.projectName?.trim() || projectId
    throw new Error(`项目“${projectName}”尚未绑定到当前登录账号、节点和本机目录，请先在项目工作区重新选择目录。`)
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

function equivalentProjectId(left: string, right: string): boolean {
  const canonical = (value: string) => value.trim().toLowerCase() === 'elon-project'
    ? 'elon-self'
    : value.trim().toLowerCase()
  return !!canonical(left) && canonical(left) === canonical(right)
}
