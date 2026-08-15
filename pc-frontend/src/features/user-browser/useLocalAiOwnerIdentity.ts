import { useCallback, useEffect, useMemo, useState } from 'react'
import { LOCAL_NODE_BASE_CHANGED_EVENT } from '../../api/runtime'
import { useAuthStore } from '../../store/auth'
import { probeLocalNode } from '../node/localNodeApi'
import { safeNodeAdminUrl } from '../../lib/utils'

export type LocalAiOwnerSource = 'cloud_account' | 'local_node' | 'anonymous_device' | 'conflict' | 'none'

export interface LocalAiOwnerIdentity {
  ownerKey: string
  ownerLabel: string
  source: LocalAiOwnerSource
  checking: boolean
  detail: string
  refresh: () => Promise<void>
}

interface LocalOwnerStatus {
  logged_in?: boolean
  owner_user_id?: string
  agent_id?: string
}

export default function useLocalAiOwnerIdentity(): LocalAiOwnerIdentity {
  const user = useAuthStore((state) => state.user)
  const [localOwnerKey, setLocalOwnerKey] = useState('')
  const [anonymousOwnerKey] = useState(readAnonymousOwnerKey)
  const [checking, setChecking] = useState(true)

  const refresh = useCallback(async () => {
    setChecking(true)
    try {
      const status = await probeLocalNode(safeNodeAdminUrl()) as LocalOwnerStatus
      setLocalOwnerKey(status.logged_in ? clean(status.owner_user_id) : '')
    } catch {
      setLocalOwnerKey('')
    } finally {
      setChecking(false)
    }
  }, [])

  useEffect(() => {
    void refresh()
    const handleNodeChange = () => { void refresh() }
    window.addEventListener(LOCAL_NODE_BASE_CHANGED_EVENT, handleNodeChange)
    return () => window.removeEventListener(LOCAL_NODE_BASE_CHANGED_EVENT, handleNodeChange)
  }, [refresh])

  return useMemo(() => {
    const cloudOwnerKey = clean(user?.id)
    if (cloudOwnerKey && localOwnerKey && cloudOwnerKey !== localOwnerKey) {
      return {
        ownerKey: '',
        ownerLabel: '账号不一致',
        source: 'conflict',
        checking: false,
        detail: '当前一龙账号与本机节点绑定账号不同。为防止混用本地网页登录数据，已暂停打开官方 AI。',
        refresh,
      }
    }
    if (cloudOwnerKey) {
      return {
        ownerKey: cloudOwnerKey,
        ownerLabel: user?.nickname || user?.account || shortOwner(cloudOwnerKey),
        source: 'cloud_account',
        checking: false,
        detail: localOwnerKey
          ? '一龙云端账号与本机节点身份一致。'
          : '使用当前一龙账号隔离这台电脑上的厂商网页会话。',
        refresh,
      }
    }
    if (localOwnerKey) {
      return {
        ownerKey: localOwnerKey,
        ownerLabel: `本机账号 ${shortOwner(localOwnerKey)}`,
        source: 'local_node',
        checking: false,
        detail: '云端页面暂未恢复账号资料，已从本机节点恢复稳定身份；厂商网页登录数据仍只保存在这台电脑。',
        refresh,
      }
    }
    if (!checking) {
      return {
        ownerKey: anonymousOwnerKey,
        ownerLabel: '本机访客',
        source: 'anonymous_device',
        checking: false,
        detail: '无需登录一龙账号即可使用官网允许的访客聊天；厂商 Cookie 和缓存按这台电脑的访客身份隔离。登录一龙账号后会切换到独立账号空间。',
        refresh,
      }
    }
    return {
      ownerKey: '',
      ownerLabel: '未识别',
      source: 'none',
      checking,
      detail: checking
        ? '正在读取一龙账号与本机节点身份…'
        : '暂时无法建立本机访客身份。',
      refresh,
    }
  }, [anonymousOwnerKey, checking, localOwnerKey, refresh, user?.account, user?.id, user?.nickname])
}

function readAnonymousOwnerKey(): string {
  const storageKey = 'elon_auth_client_instance_id'
  try {
    const existing = window.localStorage.getItem(storageKey)?.trim()
    if (existing) return `anonymous-device:${existing}`
    const created = `pc:${window.crypto.randomUUID()}`
    window.localStorage.setItem(storageKey, created)
    return `anonymous-device:${created}`
  } catch {
    return `anonymous-session:${window.crypto.randomUUID()}`
  }
}

function clean(value: unknown): string {
  return typeof value === 'string' ? value.trim() : ''
}

function shortOwner(value: string): string {
  if (value.length <= 12) return value
  return `${value.slice(0, 6)}…${value.slice(-4)}`
}
