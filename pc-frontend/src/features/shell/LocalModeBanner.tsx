import { WifiOff } from 'lucide-react'
import { Link } from 'react-router-dom'
import { useWorkbenchConnection } from './useWorkbenchConnection'
import styles from './Shell.module.css'

/** Connectivity is a recoverable state of the same workbench, never another home page. */
export default function LocalModeBanner() {
  const { cloudState, checking, retry } = useWorkbenchConnection()
  if (cloudState !== 'offline') return null
  return (
    <div className={`${styles.nodeBanner} ${styles.localModeOffline}`} role="status">
      <WifiOff className={styles.nodeBannerIcon} aria-hidden="true" size={14} />
      <span>暂时无法连接云端，已打开的内容会保留。本机和 AI 功能以当前可用状态为准。</span>
      <button type="button" onClick={retry} disabled={checking}>
        {checking ? '正在重试…' : '重试连接'}
      </button>
      <Link to="/node">连接与设备</Link>
    </div>
  )
}
