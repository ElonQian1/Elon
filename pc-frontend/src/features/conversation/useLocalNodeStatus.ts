import { useEffect, useState } from 'react'
import { safeNodeAdminUrl } from '../../lib/utils'
import { localJson } from '../doctor/localApi'

export interface LocalNodeStatus {
  agent_id?: string
  owner_user_id?: string
  device_name?: string
  connected?: boolean
  codex_cli?: { available?: boolean; logged_in?: boolean; status?: string }
}

export function useLocalNodeStatus() {
  const [localNode, setLocalNode] = useState<LocalNodeStatus | null>(null)
  const [localNodeError, setLocalNodeError] = useState('')

  useEffect(() => {
    let canceled = false
    async function loadLocalNode() {
      try {
        const status = await localJson<LocalNodeStatus>(safeNodeAdminUrl(), '/api/status')
        if (canceled) return
        setLocalNode(status)
        setLocalNodeError('')
      } catch (err) {
        if (canceled) return
        setLocalNode(null)
        setLocalNodeError((err as { message?: string }).message ?? '未检测到本机节点')
      }
    }
    loadLocalNode()
    const timer = window.setInterval(loadLocalNode, 10000)
    return () => {
      canceled = true
      window.clearInterval(timer)
    }
  }, [])

  return { localNode, localNodeError }
}
