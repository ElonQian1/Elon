import { Outlet } from 'react-router-dom'
import ServerRail from './ServerRail'
import { useNotifications } from '../notifications/useNotifications'
import styles from './Shell.module.css'

export default function Shell() {
  useNotifications()
  return (
    <div className={styles.shell}>
      <ServerRail />
      <div className={styles.content}>
        <Outlet />
      </div>
    </div>
  )
}
