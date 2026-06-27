import { useEffect } from 'react'
import { Outlet } from 'react-router-dom'
import ServerRail from './ServerRail'
import { useNotifications } from '../notifications/useNotifications'
import { useNodeAutoConnect } from './useNodeAutoConnect'
import { useAuthStore } from '../../store/auth'
import styles from './Shell.module.css'

function NodeConnectBanner() {
  const { status, errorMessage } = useNodeAutoConnect()
  if (status === 'idle') return null
  if (status === 'connecting') return (
    <div style={{ background: '#1a3d5c', color: '#7ab4e8', padding: '6px 16px', fontSize: 12, textAlign: 'center', flexShrink: 0 }}>
      🔗 检测到本机节点，正在自动绑定到你的账号…
    </div>
  )
  if (status === 'success') return (
    <div style={{ background: '#1f3a26', color: '#4caf78', padding: '6px 16px', fontSize: 12, textAlign: 'center', flexShrink: 0 }}>
      ✓ 本机节点已绑定到你的账号！可在 AI 对话页直接操控这台电脑。
    </div>
  )
  if (status === 'error') return (
    <div style={{ background: '#3a1f26', color: '#f85149', padding: '6px 16px', fontSize: 12, textAlign: 'center', flexShrink: 0 }}>
      ⚠ 节点绑定失败：{errorMessage}
    </div>
  )
  return null
}

export default function Shell() {
  useNotifications()
  const token = useAuthStore((s) => s.token)
  const user = useAuthStore((s) => s.user)
  const fetchMe = useAuthStore((s) => s.fetchMe)

  // token 存在但 user 为空时（页面刷新后等），自动拉取用户信息
  useEffect(() => {
    if (!token || user) return
    fetchMe().catch((err: { status?: number }) => {
      // 401 = token 已失效，直接登出重新登录
      if (err?.status === 401) {
        useAuthStore.getState().logout()
      }
      // 其他错误（网络异常等）默默剩下，下次订阅变化时再试
    })
  }, [token]) // eslint-disable-line

  return (
    <div className={styles.shell}>
      <ServerRail />
      <div className={styles.content} style={{ display: 'flex', flexDirection: 'column' }}>
        <NodeConnectBanner />
        <Outlet />
      </div>
    </div>
  )
}
