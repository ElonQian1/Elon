import { useCallback, useEffect, useMemo, useState } from 'react'
import { FolderOpen, RefreshCw, ShieldCheck } from 'lucide-react'
import { safeNodeAdminUrl } from '../../lib/utils'
import { nodeApi, probeLocalNode } from '../node/localNodeApi'
import {
  type FullAccessGrant,
  type WorkspaceAccessLoadState,
  runtimePermissionLabel,
  sameWorkspacePath,
  workspaceAccessStatus,
} from './workspaceAccessModel'
import styles from './WorkspaceAccessPanel.module.css'

interface LocalProjectPayload {
  name: string
  workspace_path: string
  description?: string | null
  repo_url?: string | null
  branch?: string | null
  dev_profile?: Record<string, unknown> | null
}

interface ProjectFolderResponse {
  ok?: boolean
  cancelled?: boolean
  project?: { workspace_path?: string }
  registration?: {
    can_register?: boolean
    summary?: string
    warnings?: string[]
    register_payload?: LocalProjectPayload
  }
}

interface LocalNodeStatus {
  connected?: boolean
  logged_in?: boolean
  agent_id?: string
  device_name?: string
  full_access_grant_count?: number
}

interface Props {
  projectId: string
  projectName: string
  workspacePath?: string | null
  runtimePermission?: string | null
  boundNodeId?: string | null
  onChanged: () => void | Promise<void>
}

export default function WorkspaceAccessPanel({
  projectId,
  projectName,
  workspacePath,
  runtimePermission,
  boundNodeId,
  onChanged,
}: Props) {
  const adminUrl = safeNodeAdminUrl()
  const [loadState, setLoadState] = useState<WorkspaceAccessLoadState>('loading')
  const [node, setNode] = useState<LocalNodeStatus | null>(null)
  const [grants, setGrants] = useState<FullAccessGrant[]>([])
  const [busy, setBusy] = useState<'current' | 'pick' | ''>('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  const refresh = useCallback(async (quiet = false) => {
    if (!quiet) setLoadState('loading')
    setError('')
    try {
      const status = await probeLocalNode(adminUrl) as LocalNodeStatus
      setNode(status)
      if (!status.connected || !status.logged_in) {
        setGrants([])
        setLoadState('offline')
        return
      }
      const data = await nodeApi<{ grants?: FullAccessGrant[] }>(adminUrl, '/api/full-access/grants')
      setGrants(data.grants ?? [])
      setLoadState('ready')
    } catch (err) {
      setNode(null)
      setGrants([])
      setLoadState('error')
      setError(errorMessage(err, '无法读取本机授权状态'))
    }
  }, [adminUrl])

  useEffect(() => {
    refresh()
  }, [refresh])

  const matchingGrant = useMemo(() => grants.find((grant) => (
    grant.project_id === projectId
      && sameWorkspacePath(grant.workspace_path, workspacePath ?? '')
  )), [grants, projectId, workspacePath])
  const fullAccessRequired = runtimePermission === 'full_access' || runtimePermission === 'danger_full_access'
  const localNodeIsBound = !!node?.agent_id && !!boundNodeId && node.agent_id === boundNodeId
  const canUseCurrentPath = loadState === 'ready' && !!workspacePath

  async function authorizeFolder(folder: ProjectFolderResponse, source: 'current' | 'pick') {
    const payload = folder.registration?.register_payload
    if (!payload?.workspace_path || folder.registration?.can_register === false) {
      throw new Error(folder.registration?.summary || '选择的目录缺少项目注册信息，无法授权')
    }

    const actionVerb = fullAccessRequired ? '授权' : '绑定'

    setBusy(source)
    setMessage(source === 'pick'
      ? `正在为“${projectName}”${actionVerb}所选目录…`
      : `正在为“${projectName}”${actionVerb}当前目录…`)
    setError('')
    try {
      await nodeApi(adminUrl, '/api/register-project', {
        method: 'POST',
        body: JSON.stringify({ ...payload, project_id: projectId }),
      }, 20000)
      if (fullAccessRequired) {
        await nodeApi(adminUrl, '/api/full-access/grants', {
          method: 'POST',
          body: JSON.stringify({
            project_id: projectId,
            workspace_path: payload.workspace_path,
            confirm_full_access: true,
          }),
        })
      }
      setMessage(`${fullAccessRequired ? '已授权' : '已绑定'}本机目录：${payload.workspace_path}`)
      await refresh(true)
      await onChanged()
    } catch (err) {
      setMessage('')
      setError(errorMessage(err, '本机目录授权失败'))
    } finally {
      setBusy('')
    }
  }

  async function authorizeCurrentPath() {
    if (!workspacePath) return
    setBusy('current')
    setMessage('正在检查当前项目目录…')
    setError('')
    try {
      const folder = await nodeApi<ProjectFolderResponse>(adminUrl, '/api/project-folder/inspect', {
        method: 'POST',
        body: JSON.stringify({ workspace_path: workspacePath }),
      }, 20000)
      setBusy('')
      await authorizeFolder(folder, 'current')
    } catch (err) {
      setBusy('')
      setMessage('')
      setError(errorMessage(err, '当前目录在这台电脑上不可用，请重新选择目录'))
    }
  }

  async function pickAndAuthorize() {
    setBusy('pick')
    setMessage('请在系统窗口中选择项目目录…')
    setError('')
    try {
      const folder = await nodeApi<ProjectFolderResponse>(adminUrl, '/api/project-folder/pick', {
        method: 'POST',
      }, 120000)
      if (folder.cancelled) {
        setMessage('已取消选择目录。')
        return
      }
      setBusy('')
      await authorizeFolder(folder, 'pick')
    } catch (err) {
      setMessage('')
      setError(errorMessage(err, '无法打开本机目录选择器'))
    } finally {
      setBusy('')
    }
  }

  const status = workspaceAccessStatus({
    loadState,
    matchingGrant,
    fullAccessRequired,
    localNodeIsBound,
    hasBoundNode: !!boundNodeId,
  })

  return (
    <section className={styles.panel} aria-labelledby="workspace-access-title">
      <div className={styles.heading}>
        <div className={styles.icon}><ShieldCheck size={20} strokeWidth={1.9} aria-hidden="true" /></div>
        <div>
          <h2 id="workspace-access-title">本机目录与完全访问</h2>
          <p>开发模式会为当前登录账号已绑定的项目目录自动登记本机授权，不再反复弹窗。</p>
        </div>
        <span className={styles.status} data-tone={status.tone}>{status.label}</span>
      </div>

      <div className={styles.details}>
        <div><span>当前电脑</span><strong>{node?.device_name || node?.agent_id || '未检测到'}</strong></div>
        <div><span>项目目录</span><strong title={workspacePath ?? ''}>{workspacePath || '尚未绑定'}</strong></div>
        <div><span>运行权限</span><strong>{runtimePermissionLabel(runtimePermission)}</strong></div>
      </div>

      <p className={styles.summary}>{status.summary}</p>
      <div className={styles.actions}>
        <button
          className={styles.primaryAction}
          type="button"
          disabled={busy !== '' || loadState !== 'ready'}
          onClick={pickAndAuthorize}
        >
          <FolderOpen size={17} aria-hidden="true" />
          {busy === 'pick' ? '等待选择…' : `重新选择目录并${fullAccessRequired ? '授权' : '绑定'}`}
        </button>
        <button
          className={styles.secondaryAction}
          type="button"
          disabled={!canUseCurrentPath || busy !== ''}
          onClick={authorizeCurrentPath}
        >
          <ShieldCheck size={17} aria-hidden="true" />
          {busy === 'current' ? '正在检查…' : `${fullAccessRequired ? '授权' : '绑定'}当前目录`}
        </button>
        <button
          className={styles.iconAction}
          type="button"
          disabled={busy !== '' || loadState === 'loading'}
          onClick={() => refresh()}
          aria-label="刷新本机授权状态"
          title="刷新本机授权状态"
        >
          <RefreshCw size={17} aria-hidden="true" />
        </button>
      </div>
      {message && <p className={styles.feedback} data-tone="success">{message}</p>}
      {error && <p className={styles.feedback} data-tone="danger">{error}</p>}
    </section>
  )
}

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error && error.message ? error.message : fallback
}
