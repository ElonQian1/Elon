import { getAuthToken } from '../../../api/client'
import { resolveApiUrl } from '../../../api/runtime'
import { safeNodeAdminUrl } from '../../../lib/utils'
import { nodeApi } from '../../node/localNodeApi'
import type { AndroidDeviceLeaseProof } from './deviceLeaseApi'

export interface AndroidNodeTransportHost {
  mode: 'local' | 'shared'
  projectId?: string
  agentId?: string
  displayName: string
}

let activeHost: AndroidNodeTransportHost = {
  mode: 'local',
  displayName: '本机 PC 节点',
}
let activeLease: AndroidDeviceLeaseProof | undefined
const sessionRoutes = new Map<string, {
  host: AndroidNodeTransportHost
  lease?: AndroidDeviceLeaseProof
}>()

export function setAndroidNodeTransportHost(host?: AndroidNodeTransportHost) {
  activeHost = host ?? { mode: 'local', displayName: '本机 PC 节点' }
}

export function setAndroidNodeLeaseProof(lease?: AndroidDeviceLeaseProof) {
  activeLease = lease
}

export function currentAndroidNodeTransportHost() {
  return activeHost
}

function remotePath(host: AndroidNodeTransportHost, path: string) {
  if (!host.projectId || !host.agentId) {
    throw new Error('共享设备主机信息不完整，请刷新设备列表')
  }
  return `/api/projects/${encodeURIComponent(host.projectId)}`
    + `/modules/ui-tuner/android-device-hosts/${encodeURIComponent(host.agentId)}`
    + `/relay${path.startsWith('/') ? path : `/${path}`}`
}

function sessionIdFromPath(path: string) {
  return path.match(/^\/api\/android-live\/sessions\/([^/?]+)/)?.[1]
}

export async function androidNodeApi<T = Record<string, unknown>>(
  _baseUrl: string,
  path: string,
  options: RequestInit = {},
  timeoutMs = 8000,
): Promise<T> {
  const sessionId = sessionIdFromPath(path)
  const sessionRoute = sessionId ? sessionRoutes.get(decodeURIComponent(sessionId)) : undefined
  const host = sessionRoute?.host ?? activeHost
  const lease = sessionRoute?.lease ?? activeLease
  if (host.mode === 'local') {
    const data = await nodeApi<T>(safeNodeAdminUrl(), path, options, timeoutMs)
    if (path === '/api/android-live/sessions' && options.method === 'POST') {
      const created = (data as { session?: { id?: string } }).session?.id
      if (created) sessionRoutes.set(created, {
        host: { ...host },
        lease: lease ? { ...lease } : undefined,
      })
    } else if (sessionId && options.method === 'DELETE') {
      sessionRoutes.delete(decodeURIComponent(sessionId))
    }
    return data
  }
  if (!lease?.leaseId || !lease.hardwareSerial) {
    throw new Error('请先取得公共测试手机使用权')
  }
  const controller = new AbortController()
  const timer = window.setTimeout(() => controller.abort(), timeoutMs)
  try {
    const token = getAuthToken()
    const headers: Record<string, string> = {
      ...(options.headers as Record<string, string> | undefined),
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      'X-Elon-Device-Lease-Id': lease.leaseId,
      'X-Elon-Device-Hardware-Serial': lease.hardwareSerial,
    }
    if (options.body && !Object.keys(headers).some((name) => name.toLowerCase() === 'content-type')) {
      headers['Content-Type'] = 'application/json'
    }
    const response = await fetch(resolveApiUrl(remotePath(host, path)), {
      cache: 'no-store',
      ...options,
      headers,
      signal: controller.signal,
    })
    const text = await response.text()
    let data: Record<string, unknown> = {}
    if (text) {
      try {
        data = JSON.parse(text) as Record<string, unknown>
      } catch {
        data = { error: text }
      }
    }
    if (!response.ok) {
      throw new Error(String(data.error ?? data.message ?? `HTTP ${response.status}`))
    }
    if (path === '/api/android-live/sessions' && options.method === 'POST') {
      const created = (data.session as { id?: string } | undefined)?.id
      if (created) sessionRoutes.set(created, {
        host: { ...host },
        lease: lease ? { ...lease } : undefined,
      })
    } else if (sessionId && options.method === 'DELETE') {
      sessionRoutes.delete(decodeURIComponent(sessionId))
    }
    return data as T
  } finally {
    window.clearTimeout(timer)
  }
}
