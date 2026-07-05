import { lifecycleView } from './nodeLifecycle'
import type { LocalNodeStatus, NodeSummary } from './types'
import styles from './NodePage.module.css'

type Props = {
  localStatus?: LocalNodeStatus
  node?: NodeSummary
}

export default function NodeLifecycleStatusCard({ localStatus, node }: Props) {
  const view = localStatus
    ? lifecycleView(localStatus.lifecycle, {
      connected: localStatus.connected,
      loggedIn: localStatus.logged_in,
      lastEvent: localStatus.last_event,
    })
    : lifecycleView(node?.lifecycle, { online: node?.online, connected: node?.online })

  return (
    <section className={[styles.lifecycleCard, styles[`lifecycle_${view.tone}`]].join(' ')}>
      <div className={styles.lifecycleHeader}>
        <div>
          <h4>{view.title}</h4>
          <p>{view.detail}</p>
        </div>
        <span className={styles.lifecycleBadge}>{view.badge}</span>
      </div>
      <div className={styles.lifecycleFacts}>
        {view.facts.map((fact) => (
          <div key={fact.label}>
            <span>{fact.label}</span>
            <strong>{fact.value}</strong>
          </div>
        ))}
      </div>
      <div className={styles.lifecycleAction}>{view.actionLabel}</div>
    </section>
  )
}
