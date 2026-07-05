import { useEffect } from 'react'
import { Outlet, useNavigate } from 'react-router-dom'
import { CircleCheck, Link2, TriangleAlert, WifiOff } from 'lucide-react'
import ServerRail from './ServerRail'
import { useNotifications } from '../notifications/useNotifications'
import { useNodeAutoConnect } from './useNodeAutoConnect'
import { useWorkbenchTabCoordinator } from './useWorkbenchTabCoordinator'
import { useAuthStore } from '../../store/auth'
import AppUpdateWatcher from '../updates/AppUpdateWatcher'
import LocalModeBanner from './LocalModeBanner'
import styles from './Shell.module.css'

function NodeConnectBanner() {
  const { status, errorMessage, detailMessage } = useNodeAutoConnect()
  if (status === 'idle') return null
  const bannerClass = [
    styles.nodeBanner,
    status === 'connecting' ? styles.nodeBannerConnecting : '',
    status === 'success' ? styles.nodeBannerSuccess : '',
    status === 'offline' ? styles.nodeBannerOffline : '',
    status === 'error' ? styles.nodeBannerError : '',
  ].filter(Boolean).join(' ')
  if (status === 'connecting') return (
    <div className={bannerClass}>
      <Link2 className={styles.nodeBannerIcon} aria-hidden="true" size={14} />
      <span>{detailMessage || '检测到本机节点，正在自动绑定到你的账号…'}</span>
    </div>
  )
  if (status === 'success') return (
    <div className={bannerClass}>
      <CircleCheck className={styles.nodeBannerIcon} aria-hidden="true" size={14} />
      <span>{detailMessage || '本机节点已绑定到你的账号，可在 AI 对话页直接操控这台电脑。'}</span>
    </div>
  )
  if (status === 'offline') return (
    <div className={bannerClass}>
      <WifiOff className={styles.nodeBannerIcon} aria-hidden="true" size={14} />
      <span>{detailMessage || '本机 Win 端当前不可达；启动后会自动重新绑定。'}</span>
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

function AccountClaimBanner() {
  const navigate = useNavigate()
  const token = useAuthStore((s) => s.token)
  const user = useAuthStore((s) => s.user)
  if (token || user) return null

  return (
    <div className={styles.claimBanner}>
      <span>注册账号后同步项目、好友和电脑节点。</span>
      <button type="button" onClick={() => navigate('/login?mode=register')}>
        注册账号
      </button>
    </div>
  )
}

function DuplicateWorkbenchNotice() {
  return (
    <div className={styles.duplicateTabPage} role="status">
      <div className={styles.duplicateTabPanel}>
        <CircleCheck size={26} aria-hidden="true" />
        <h1>PC 工作台已在另一个标签打开</h1>
        <p>已切回已有标签；本页可以关闭。</p>
        <button type="button" onClick={() => window.close()}>
          关闭本页
        </button>
      </div>
    </div>
  )
}

export default function Shell() {
  const duplicateTab = useWorkbenchTabCoordinator()
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
  }, [token])

  if (duplicateTab) return <DuplicateWorkbenchNotice />

  return (
    <div className={styles.shell}>
      <ServerRail />
      <div className={styles.content}>
        <AccountClaimBanner />
        <LocalModeBanner />
        <NodeConnectBanner />
        <main className={styles.routeFrame}>
          <Outlet />
        </main>
      </div>
      <AppUpdateWatcher />
    </div>
  )
}
