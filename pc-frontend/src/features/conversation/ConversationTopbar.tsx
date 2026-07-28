import type { Channel, Project } from './types'
import styles from './ConversationTopbar.module.css'

interface ConversationTopbarProps {
  activeChannel?: Channel
  activeProject?: Project
  canRefresh: boolean
  onRefresh: () => void | Promise<void>
  onOpenNode: () => void
  onOpenMobile: () => void
  onOpenLegacy: () => void
}

export default function ConversationTopbar({
  activeChannel,
  activeProject,
  canRefresh,
  onRefresh,
  onOpenNode,
  onOpenMobile,
  onOpenLegacy,
}: ConversationTopbarProps) {
  return (
    <header className={styles.chatTopbar}>
      <div className={styles.chatTitle}>
        <span className={styles.chatTitleGlyph}>
          {activeChannel?.kind === 'ai_development' ? '🛠' : (activeChannel ? '#' : '💬')}
        </span>
        <div>
          <strong className={styles.chatTitleText}>
            {activeChannel?.name ?? activeProject?.name ?? '选择项目开始对话'}
          </strong>
          {activeChannel?.description && (
            <span className={styles.chatTitleSub}>{activeChannel.description}</span>
          )}
        </div>
      </div>
      <div className={styles.topbarActions}>
        {canRefresh && (
          <button className={styles.textBtn} type="button" onClick={onRefresh}>
            刷新
          </button>
        )}
        <button
          className={styles.textBtn}
          type="button"
          title="分享这台电脑的算力并查看连接状态"
          onClick={onOpenNode}
        >
          分享算力
        </button>
        <button
          className={styles.textBtn}
          type="button"
          title="打开移动端入口"
          onClick={onOpenMobile}
        >
          打开移动端
        </button>
        <button
          className={styles.textBtn}
          type="button"
          title="切换到旧版 PC 工作台"
          onClick={onOpenLegacy}
        >
          旧版
        </button>
      </div>
    </header>
  )
}
