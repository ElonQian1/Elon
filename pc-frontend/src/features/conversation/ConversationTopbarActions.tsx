import { ChevronLeft, RefreshCw } from 'lucide-react'
import { useProjectStore } from './useProjectStore'
import styles from './ConversationPage.module.css'

interface ConversationTopbarActionsProps {
  activeProjectId?: string | null
  activeChannelId?: string | null
  memberCollapsed: boolean
  onToggleMemberPanel: () => void
}

export default function ConversationTopbarActions({
  activeProjectId,
  activeChannelId,
  memberCollapsed,
  onToggleMemberPanel,
}: ConversationTopbarActionsProps) {
  function refreshMessages() {
    if (!activeProjectId || !activeChannelId) return
    useProjectStore.getState().loadMessages(activeProjectId, activeChannelId)
  }

  return (
    <div className={styles.topbarActions}>
      {memberCollapsed && (
        <button
          className={[styles.textBtn, styles.panelControlBtn].join(' ')}
          type="button"
          title="显示右侧栏"
          aria-label="显示右侧栏"
          aria-pressed="false"
          onClick={onToggleMemberPanel}
        >
          <ChevronLeft size={15} aria-hidden="true" /><span>项目详情</span>
        </button>
      )}
      {activeProjectId && activeChannelId && (
        <button className={styles.textBtn} type="button" title="刷新消息" onClick={refreshMessages}>
          <RefreshCw size={15} aria-hidden="true" /><span>刷新</span>
        </button>
      )}
    </div>
  )
}
