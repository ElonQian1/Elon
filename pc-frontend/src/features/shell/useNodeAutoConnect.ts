/**
 * useNodeAutoConnect — 登录后自动探测并绑定本机节点
 *
 * 逻辑：只要用户已登录，就尝试探测 localhost:7799，
 * 探测到节点后自动发送 token 完成绑定。
 * 每 30s 重试一次（覆盖"先登录、后启动节点"的场景）。
 * 不依赖任何 URL 参数。
 */
import { useEffect, useRef, useState } from 'react'
import { useAuthStore } from '../../store/auth'

export type NodeConnectStatus =
  | 'idle'        // 未登录或节点探测未开始
  | 'connecting'  // 正在发送 token 给本地节点
  | 'success'     // 绑定成功
  | 'error'       // 发生错误（会自动重试）

interface NodeConnectState {
  status: NodeConnectStatus
  errorMessage: string
}

const LOCAL_NODE_PORT = 7799
const LOCAL_NODE_BASES = [
  `http://127.0.0.1:${LOCAL_NODE_PORT}`,
  `http://localhost:${LOCAL_NODE_PORT}`,
]
const PROBE_INTERVAL_MS = 30_000

export function useNodeAutoConnect(): NodeConnectState {
  const token = useAuthStore((s: { token: string | null }) => s.token)
  const [state, setState] = useState<NodeConnectState>({ status: 'idle', errorMessage: '' })
  const successRef = useRef(false)
  const tokenRef = useRef<string | null>(null)

  useEffect(() => {
    if (tokenRef.current !== token) {
      tokenRef.current = token
      successRef.current = false
      setState({ status: 'idle', errorMessage: '' })
    }
    if (!token || successRef.current) return

    // 立即探测一次
    probe(token)

    // 每 30s 重试（覆盖"先登录、后启动节点"场景）
    const timer = setInterval(() => {
      if (successRef.current) { clearInterval(timer); return }
      probe(token)
    }, PROBE_INTERVAL_MS)

    return () => clearInterval(timer)
  }, [token]) // eslint-disable-line

  async function probe(userToken: string) {
    try {
      const found = await probeLocalNode()
      if (!found) return // 节点没在跑，静默跳过
      const { baseUrl, localAdminToken } = found

      setState({ status: 'connecting', errorMessage: '' })

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
        setState({ status: 'error', errorMessage: loginData.error ?? '绑定失败' })
        return
      }

      successRef.current = true
      setState({ status: 'success', errorMessage: '' })
    } catch {
      // 节点没在跑，静默跳过（不显示错误）
    }
  }

  return state
}

async function probeLocalNode(): Promise<{ baseUrl: string; localAdminToken: string } | null> {
  for (const baseUrl of LOCAL_NODE_BASES) {
    try {
      const res = await fetch(`${baseUrl}/api/status`, {
        credentials: 'omit',
        signal: AbortSignal.timeout(2000),
      })
      if (!res.ok) continue
      const data = await res.json()
      const localAdminToken: string = data.local_admin_token ?? ''
      if (localAdminToken) return { baseUrl, localAdminToken }
    } catch {
      // Try the next loopback hostname.
    }
  }
  return null
}

