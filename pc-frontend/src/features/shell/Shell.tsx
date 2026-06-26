import { Outlet } from 'react-router-dom'
import Sidebar from './Sidebar'
import styles from './Shell.module.css'

export default function Shell() {
  return (
    <div className={styles.shell}>
      <Sidebar />
      <main className={styles.content}>
        <Outlet />
      </main>
    </div>
  )
}
