import { useEffect } from 'react'
import { Outlet } from 'react-router-dom'
import { CircleCheck, Link2, TriangleAlert } from 'lucide-react'
import ServerRail from './ServerRail'
import { useNotifications } from '../notifications/useNotifications'
import { useNodeAutoConnect } from './useNodeAutoConnect'
import { useAuthStore } from '../../store/auth'
import styles from './Shell.module.css'

function NodeConnectBanner() {
  const { status, errorMessage } = useNodeAutoConnect()
  if (status === 'idle') return null
  const bannerClass = [
    styles.nodeBanner,
    status === 'connecting' ? styles.nodeBannerConnecting : '',
    status === 'success' ? styles.nodeBannerSuccess : '',
    status === 'error' ? styles.nodeBannerError : '',
  ].filter(Boolean).join(' ')
  if (status === 'connecting') return (
    <div className={bannerClass}>
      <Link2 className={styles.nodeBannerIcon} aria-hidden="true" size={14} />
      <span>检测到本机节点，正在自动绑定到你的账号…</span>
    </div>
  )
  if (status === 'success') return (
    <div className={bannerClass}>
      <CircleCheck className={styles.nodeBannerIcon} aria-hidden="true" size={14} />
      <span>本机节点已绑定到你的账号，可在 AI 对话页直接操控这台电脑。</span>
    </div>
  )
  if (status === 'error') return (
    <div className={bannerClass}>
      <TriangleAlert className={styles.nodeBannerIcon} aria-hidden="true" size={14} />
      <span>节点绑定失败：{errorMessage}</span>
    </div>
  )
  return null
}

export default function Shell() {
  useNotifications()
  const token = useAuthStore((s) => s.token)
  const fetchMe = useAuthStore((s) => s.fetchMe)

  // token 存在时始终刷新用户信息（确保 user.id 格式正确）
  // 不再依赖 !user 判断，因为旧版 localStorage 可能存了格式错误的 user 对象
  useEffect(() => {
    if (!token) return
    fetchMe().catch((err: { status?: number }) => {
      if (err?.status === 401) {
        useAuthStore.getState().logout()
      }
    })
  }, [token]) // eslint-disable-line

  return (
    <div className={styles.shell}>
      <ServerRail />
      <div className={styles.content}>
        <NodeConnectBanner />
        <Outlet />
      </div>
    </div>
  )
}
