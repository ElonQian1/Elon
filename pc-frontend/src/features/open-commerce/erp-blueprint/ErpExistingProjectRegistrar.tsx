import { useState } from 'react'
import { FolderOpen, LoaderCircle } from 'lucide-react'
import { safeNodeAdminUrl } from '../../../lib/utils'
import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import { errorMessage } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

type LocalProjectPayload = {
  name: string
  workspace_path: string
  description?: string | null
  repo_url?: string | null
  branch?: string | null
  dev_profile?: Record<string, unknown> | null
}

type ProjectFolderResponse = {
  cancelled?: boolean
  registration?: {
    can_register?: boolean
    summary?: string
    register_payload?: LocalProjectPayload
  }
}

type LocalNodeStatus = {
  logged_in?: boolean
  user_token_configured?: boolean
}

type RegisterProjectResponse = {
  cloud?: {
    project?: { id?: string }
    reused_existing?: boolean
  }
}

export default function ErpExistingProjectRegistrar({
  canEdit,
  disabled,
  onRegistered,
}: {
  canEdit: boolean
  disabled: boolean
  onRegistered: (projectId: string) => Promise<void>
}) {
  const [busy, setBusy] = useState(false)
  const [status, setStatus] = useState('')

  async function pickAndRegister() {
    setBusy(true)
    setStatus('')
    try {
      const adminUrl = safeNodeAdminUrl()
      const node = await probeLocalNode(adminUrl) as LocalNodeStatus
      if (!node.logged_in || !node.user_token_configured) {
        throw new Error('本机节点尚未登录一龙账号，请先完成节点登录。')
      }

      const folder = await nodeApi<ProjectFolderResponse>(adminUrl, '/api/project-folder/pick', {
        method: 'POST',
      }, 120_000)
      if (folder.cancelled) {
        setStatus('已取消选择。')
        return
      }

      const payload = folder.registration?.register_payload
      if (!payload?.workspace_path || folder.registration?.can_register === false) {
        throw new Error(folder.registration?.summary || '该目录无法登记为平台项目。')
      }

      const response = await nodeApi<RegisterProjectResponse>(adminUrl, '/api/register-project', {
        method: 'POST',
        body: JSON.stringify(payload),
      }, 20_000)
      const registeredProjectId = response.cloud?.project?.id?.trim()
      if (!registeredProjectId) {
        throw new Error('项目登记成功，但云端未返回项目标识。')
      }

      await onRegistered(registeredProjectId)
      setStatus(response.cloud?.reused_existing ? '已复用并选中平台项目。' : '已登记并选中平台项目。')
    } catch (error) {
      setStatus(errorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={styles.inlineForm}>
      <button type="button" disabled={!canEdit || disabled || busy} onClick={pickAndRegister}>
        {busy ? <LoaderCircle className={styles.spin} size={15} /> : <FolderOpen size={15} />}
        登记本机仓库
      </button>
      {status && <small className={styles.mutedLine} role="status">{status}</small>}
    </div>
  )
}
