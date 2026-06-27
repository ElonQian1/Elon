import { Outlet, useNavigate } from 'react-router-dom'
import ServerRail from './ServerRail'
import { useNotifications } from '../notifications/useNotifications'
import { useAuthStore } from '../../store/auth'
import styles from './Shell.module.css'

export default function Shell() {
  useNotifications()
  const navigate = useNavigate()
  const token = useAuthStore((s) => s.token)

  function openLegacy() {
    // 兼容旧版页面：旧版读取 lodex_token / elon_token
    if (token) {
      localStorage.setItem('lodex_token', token)
      localStorage.setItem('elon_token', token)
    }
    window.open('/pc-legacy', '_blank', 'noopener')
  }

  return (
    <div className={styles.shell}>
      <ServerRail />
      <div className={styles.content}>
        {/* 全局永久按鈕条：任何页面都显示 */}
        <div className={styles.globalBar}>
          <div className={styles.globalActions}>
            <button className={styles.globalBtn} type="button"
              title="切换到旧版 PC 工作台"
              onClick={openLegacy}>
              旧版
            </button>
            <button className={styles.globalBtn} type="button"
              title="分享这台电脑的算力并查看连接状态"
              onClick={() => navigate('/node')}>
              分享算力
            </button>
            <button className={styles.globalBtn} type="button"
              title="打开移动端入口"
              onClick={() => window.open('/app/download', '_blank', 'noopener')}>
              打开移动端
            </button>
          </div>
        </div>
        <div className={styles.outlet}>
          <Outlet />
        </div>
      </div>
    </div>
  )
}
