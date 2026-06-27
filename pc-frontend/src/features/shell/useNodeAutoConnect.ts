/**
 * useNodeAutoConnect — 云端 PC 页面自动绑定本机节点
 *
 * 触发条件：URL 携带 ?node_port=PORT（由 elon-pc-node.exe 打开浏览器时附带）
 *
 * 流程：
 *   1. 检测 ?node_port=PORT
 *   2. GET http://localhost:PORT/api/status （云端 origin 在 CORS 白名单）
 *      → 拿到 local_admin_token
 *   3. 等用户登录（token 有值）后，自动 POST http://localhost:PORT/api/login
 *      { token: <用户 JWT> }  + x-elon-local-admin-token header
 *   4. 节点收到 token → 向云端注册 → 连接成功
 *   5. 清除 URL 中的 node_port 参数，显示成功提示
 */
import { useEffect, useRef, useState } from 'react'
import { useAuthStore } from '../../store/auth'

export type NodeConnectStatus =
  | 'idle'           // 没有 node_port 参数，不做任何事
  | 'waiting_login'  // 已检测到本地节点但用户还没登录
  | 'connecting'     // 正在向本地节点发送 token
  | 'success'        // 绑定成功
  | 'error'          // 发生错误

interface NodeConnectState {
  status: NodeConnectStatus
  nodePort: number | null
  errorMessage: string
}

export function useNodeAutoConnect(): NodeConnectState {
  const token = useAuthStore((s: { token: string | null }) => s.token)
  const [state, setState] = useState<NodeConnectState>({
    status: 'idle',
    nodePort: null,
    errorMessage: '',
  })
  const attemptedRef = useRef(false)

  // 检测 URL 参数
  useEffect(() => {
    const params = new URLSearchParams(window.location.search)
    const portStr = params.get('node_port')
    if (!portStr) return
    const port = parseInt(portStr, 10)
    if (!port || isNaN(port) || port < 1024 || port > 65535) return

    setState((s) => ({ ...s, status: 'waiting_login', nodePort: port }))
  }, [])

  // 当用户登录且有待绑定的节点端口时，自动发送 token
  useEffect(() => {
    if (!token || !state.nodePort || state.status !== 'waiting_login' || attemptedRef.current) return
    attemptedRef.current = true
    connectToNode(state.nodePort, token)
  }, [token, state.nodePort, state.status]) // eslint-disable-line

  async function connectToNode(port: number, userToken: string) {
    setState((s) => ({ ...s, status: 'connecting' }))
    try {
      // 1. 获取 local_admin_token（云端 origin 已在节点 CORS 白名单）
      const statusRes = await fetch(`http://localhost:${port}/api/status`, {
        credentials: 'omit',
      })
      if (!statusRes.ok) throw new Error(`本地节点未响应（${statusRes.status}）`)
      const statusData = await statusRes.json()
      const localAdminToken: string = statusData.local_admin_token ?? ''
      if (!localAdminToken) throw new Error('本地节点未返回管理令牌，请重启节点后重试')

      // 2. 把用户 token 发给本地节点，触发自动注册
      const loginRes = await fetch(`http://localhost:${port}/api/login`, {
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
        throw new Error(loginData.error ?? `登录失败（${loginRes.status}）`)
      }

      // 3. 成功：清除 URL 参数
      const url = new URL(window.location.href)
      url.searchParams.delete('node_port')
      window.history.replaceState({}, '', url.toString())

      setState((s) => ({ ...s, status: 'success' }))
    } catch (err) {
      setState((s) => ({
        ...s,
        status: 'error',
        errorMessage: (err as Error).message ?? '连接失败',
      }))
      attemptedRef.current = false // 允许重试
    }
  }

  return state
}
