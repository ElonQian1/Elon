import type { TaskTimelineModel } from './taskTimelineModel'
import { taskCompletionMetaModel } from './taskCompletionMetaModel'
import styles from './DevTaskGroup.module.css'

export default function TaskCompletionMeta({ timeline }: { timeline: TaskTimelineModel }) {
  const meta = taskCompletionMetaModel(timeline)
  if (!meta) return null
  return (
    <div className={styles.completionMeta} aria-label="本轮用量">
      {meta.model && <span>{meta.model}</span>}
      {meta.model && meta.usage && <i aria-hidden="true">·</i>}
      {meta.usage && <span>{meta.usage}</span>}
    </div>
  )
}
