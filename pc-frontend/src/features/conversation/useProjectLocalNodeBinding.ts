import { useEffect, useRef, useState } from 'react'
import { api } from '../../api/client'
import { projectRoleCanAutoBind } from './conversationPageUtils'
import type { Project } from './types'

interface UseProjectLocalNodeBindingOptions {
  activeProjectId?: string | null
  activeProject?: Project
  activeWorkspacePath: string
  activeProjectRole: string
  localNodeReady: boolean
  localNodeId: string
  shouldPreferLocalNode: boolean
  loadProjects: () => Promise<void>
  reloadProjectSpace: () => Promise<void>
}

export function useProjectLocalNodeBinding({
  activeProjectId,
  activeProject,
  activeWorkspacePath,
  activeProjectRole,
  localNodeReady,
  localNodeId,
  shouldPreferLocalNode,
  loadProjects,
  reloadProjectSpace,
}: UseProjectLocalNodeBindingOptions): string {
  const [localBindStatus, setLocalBindStatus] = useState('')
  const autoBindRef = useRef('')

  useEffect(() => {
    if (!activeProjectId || !activeProject || !localNodeReady || !localNodeId) return
    if (!shouldPreferLocalNode) return
    if (!projectRoleCanAutoBind(activeProjectRole)) {
      setLocalBindStatus(activeProjectRole ? '当前项目不是 owner，不自动切换节点' : '')
      return
    }
    if (activeProject.node_id === localNodeId) {
      setLocalBindStatus('')
      return
    }
    if (!activeWorkspacePath) {
      setLocalBindStatus('当前项目缺少工作区路径，暂不自动切换')
      return
    }
    const projectId = activeProjectId
    const key = `${activeProjectId}:${localNodeId}:${activeProject.node_id ?? ''}:${activeWorkspacePath || 'no-path'}`
    if (autoBindRef.current === key) return
    autoBindRef.current = key
    setLocalBindStatus('正在切换到当前电脑…')
    let canceled = false
    async function recoverOnLocalNode() {
      const endpoint = `/api/projects/${encodeURIComponent(projectId)}/workspace/recover`
      const bindPayload: { action: string; node_id: string; workspacePath: string } = {
        action: 'bind_pc_node',
        node_id: localNodeId,
        workspacePath: activeWorkspacePath,
      }
      await api.post<{ project?: unknown; message?: string }>(endpoint, bindPayload)
      if (canceled) return
      setLocalBindStatus('已优先使用当前电脑')
      await loadProjects()
      await reloadProjectSpace()
    }
    recoverOnLocalNode().catch((err: { message?: string }) => {
      if (!canceled) setLocalBindStatus(err.message ?? '当前电脑自动绑定失败')
    })
    return () => { canceled = true }
  }, [
    activeProjectId,
    activeProject,
    activeWorkspacePath,
    activeProjectRole,
    localNodeReady,
    localNodeId,
    shouldPreferLocalNode,
    loadProjects,
    reloadProjectSpace,
  ])

  return localBindStatus
}
