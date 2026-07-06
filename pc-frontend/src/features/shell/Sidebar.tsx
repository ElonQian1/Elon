import { NavLink } from 'react-router-dom'
import { ModelPickerButton } from '../models/ModelPicker'
import SidebarUserStrip from './SidebarUserStrip'
import styles from './Sidebar.module.css'

interface NavItem {
  to: string
  label: string
  icon: string
}

const NAV_ITEMS: NavItem[] = [
  { to: '/workspace', icon: '💬', label: '项目对话' },
  { to: '/voice', icon: '🎙️', label: '声音' },
  { to: '/doctor', icon: '🩺', label: '电脑医生' },
  { to: '/node', icon: '🖥️', label: '本机节点' },
]

export default function Sidebar() {
  return (
    <nav className={styles.sidebar}>
      <div className={styles.brand}>
        <span className={styles.brandName}>一龙</span>
        <span className={styles.brandBadge}>Beta</span>
      </div>

      <ul className={styles.navList}>
        {NAV_ITEMS.map((item) => (
          <li key={item.to}>
            <NavLink
              to={item.to}
              end={item.to === '/'}
              className={({ isActive }) =>
                [styles.navItem, isActive ? styles.active : ''].join(' ')
              }
            >
              <span className={styles.icon}>{item.icon}</span>
              <span className={styles.label}>{item.label}</span>
            </NavLink>
          </li>
        ))}
      </ul>

      <div className={styles.footer}>
        <ModelPickerButton />
        <SidebarUserStrip />
      </div>
    </nav>
  )
}
