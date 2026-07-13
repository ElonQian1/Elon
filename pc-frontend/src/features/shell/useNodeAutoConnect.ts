/**
 * useNodeAutoConnect — 登录后自动探测并绑定本机节点
 *
 * 逻辑：只要用户已登录，就尝试探测启动器允许的本机端口范围，
 * 探测到节点后自动发送 token 完成绑定。
 * 每 30s 持续确认一次，避免顶部"已绑定"状态在 Win 端退出后变成过期绿条。
 * 不依赖任何 URL 参数。
 */
import { useEffect, useRef, useState } from 'react'
import { useAuthStore } from '../../store/auth'
import { localNodeProbeBaseUrls, rememberLocalNodeBaseUrl } from '../../api/runtime'

export type NodeConnectStatus =
  | 'idle'        // 未登录或节点探测未开始
  | 'connecting'  // 正在发送 token 给本地节点
  | 'success'     // 绑定成功
  | 'offline'     // 曾经绑定成功，但当前本机 Win 端不可达
  | 'error'       // 发生错误（会自动重试）

interface NodeConnectState {
  status: NodeConnectStatus
  errorMessage: string
  detailMessage: string
}

const PROBE_INTERVAL_MS = 30_000
const PRIMARY_PROBE_TIMEOUT_MS = 1200
const FALLBACK_PROBE_TIMEOUT_MS = 900

export function useNodeAutoConnect(): NodeConnectState {
  const token = useAuthStore((s: { token: string | null }) => s.token)
  const userId = useAuthStore((s) => s.user?.id ?? '')
  const [state, setState] = useState<NodeConnectState>({
    status: 'idle',
    errorMessage: '',
    detailMessage: '',
  })
  const seenLocalNodeRef = useRef(false)
  const tokenRef = useRef<string | null>(null)

  useEffect(() => {
    if (tokenRef.current !== token) {
      tokenRef.current = token
      seenLocalNodeRef.current = false
      setState({ status: 'idle', errorMessage: '', detailMessage: '' })
    }
    if (!token) return

    // 立即探测一次
    probe(token, userId)

    // 每 30s 持续确认（覆盖"先登录、后启动节点"和"节点后来退出"场景）
    const timer = setInterval(() => {
      probe(token, userId)
    }, PROBE_INTERVAL_MS)

    return () => clearInterval(timer)
  }, [token, userId])

  async function probe(userToken: string, currentUserId: string) {
    try {
      const found = await probeLocalNode()
      if (!found) {
        setState((current) => {
          if (!seenLocalNodeRef.current && current.status !== 'success') {
            return { status: 'idle', errorMessage: '', detailMessage: '' }
          }
          return {
            status: 'offline',
            errorMessage: '',
            detailMessage: '本机 Win 端当前不可达；启动后会自动重新绑定。',
          }
        })
        return
      }
      seenLocalNodeRef.current = true
      const { baseUrl, localAdminToken, status } = found
      rememberLocalNodeBaseUrl(baseUrl)
      const ownerOk = !currentUserId || !status.owner_user_id || status.owner_user_id === currentUserId
      const alreadyBound = !!status.logged_in && ownerOk

      if (alreadyBound && status.connected !== false) {
        setState({
          status: 'success',
          errorMessage: '',
          detailMessage: '本机节点已绑定到你的账号，可在 AI 对话页直接操控这台电脑。',
        })
        return
      }

      if (alreadyBound && status.connected === false) {
        setState({
          status: 'connecting',
          errorMessage: '',
          detailMessage: '本机 Win 端已启动，正在等待云端连接恢复…',
        })
        return
      }

      setState({
        status: 'connecting',
        errorMessage: '',
        detailMessage: status.connected === false
          ? '本机 Win 端已启动，正在等待云端连接恢复…'
          : '检测到本机节点，正在自动绑定到你的账号…',
      })

      const loginRes = await fetch(`${baseUrl}/api/login`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-elon-local-admin-token': localAdminToken,
        },
        body: JSON.stringify({ token: userToken }),
        credentials: 'omit',
      })
      const loginData = await loginRes.json()
      if (!loginRes.ok || !loginData.ok) {
        setState({
          status: 'error',
          errorMessage: loginData.error ?? '绑定失败',
          detailMessage: '',
        })
        return
      }

      setState({
        status: 'success',
        errorMessage: '',
        detailMessage: '本机节点已绑定到你的账号，可在 AI 对话页直接操控这台电脑。',
      })
    } catch {
      setState((current) => {
        if (!seenLocalNodeRef.current && current.status !== 'success') {
          return { status: 'idle', errorMessage: '', detailMessage: '' }
        }
        return {
          status: 'offline',
          errorMessage: '',
          detailMessage: '本机 Win 端当前不可达；启动后会自动重新绑定。',
        }
      })
    }
  }

  return state
}

interface LocalNodeProbeStatus {
  logged_in?: boolean
  owner_user_id?: string
  connected?: boolean
  local_admin_token?: string
  local_admin_token_header?: string
  agent_id?: string
  version?: string
}

async function probeLocalNode(): Promise<{
  baseUrl: string
  localAdminToken: string
  status: LocalNodeProbeStatus
} | null> {
  const [primary, ...fallbacks] = localNodeProbeBaseUrls()
  const preferred = primary ? await probeLocalNodeBase(primary, PRIMARY_PROBE_TIMEOUT_MS) : null
  if (preferred) return preferred
  const matches = await Promise.all(
    fallbacks.map((baseUrl) => probeLocalNodeBase(baseUrl, FALLBACK_PROBE_TIMEOUT_MS)),
  )
  return matches.find((match) => match !== null) ?? null
}

async function probeLocalNodeBase(baseUrl: string, timeoutMs: number): Promise<{
  baseUrl: string
  localAdminToken: string
  status: LocalNodeProbeStatus
} | null> {
  try {
    const res = await fetch(`${baseUrl}/api/status`, {
      credentials: 'omit',
      signal: AbortSignal.timeout(timeoutMs),
    })
    if (!res.ok) return null
    const status = await res.json() as LocalNodeProbeStatus
    const localAdminToken = status.local_admin_token ?? ''
    const tokenHeader = status.local_admin_token_header?.trim().toLowerCase()
    const isElonNode = tokenHeader === 'x-elon-local-admin-token'
      && status.agent_id?.startsWith('node-')
      && !!status.version?.trim()
    return localAdminToken && isElonNode ? { baseUrl, localAdminToken, status } : null
  } catch {
    return null
  }
}

