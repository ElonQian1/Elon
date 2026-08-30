import { Outlet } from 'react-router-dom'
import WorkspaceFeatureNav from '../shell/WorkspaceFeatureNav'
import styles from './ComputeWorkspaceLayout.module.css'

/**
 * Shared shell for the compute workspace.
 *
 * The compute pages keep their own list/detail workbench, while this column
 * owns the workspace-level navigation shared by every compute route.
 */
export default function ComputeWorkspaceLayout() {
  return (
    <div className={styles.layout}>
      <aside className={styles.sidebar} aria-label="算力工作区导航">
        <WorkspaceFeatureNav />
      </aside>
      <div className={styles.content}>
        <Outlet />
      </div>
    </div>
  )
}
