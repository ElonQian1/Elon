import { ChevronLeft, Cpu, History, RefreshCw, Smartphone, UsersRound } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { useProjectStore } from './useProjectStore'
import styles from './ConversationPage.module.css'

interface ConversationTopbarActionsProps {
  activeProjectId?: string | null
  activeChannelId?: string | null
  memberCollapsed: boolean
  memberSelectionMode: boolean
  onToggleMemberPanel: () => void
  onEnableMemberSelection: () => void
  onNavigateNode: () => void
}

export default function ConversationTopbarActions({
  activeProjectId,
  activeChannelId,
  memberCollapsed,
  memberSelectionMode,
  onToggleMemberPanel,
  onEnableMemberSelection,
  onNavigateNode,
}: ConversationTopbarActionsProps) {
  function selectMember() {
    if (memberCollapsed) onToggleMemberPanel()
    onEnableMemberSelection()
  }

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
          <ChevronLeft size={15} aria-hidden="true" /><span>右栏</span>
        </button>
      )}
      <button
        className={[styles.textBtn, styles.panelControlBtn].join(' ')}
        type="button"
        title={memberCollapsed ? '显示右侧成员栏并选择成员' : '在右侧成员栏选择成员'}
        aria-label={memberCollapsed ? '显示右侧成员栏并选择成员' : '在右侧成员栏选择成员'}
        aria-pressed={memberSelectionMode}
        onClick={selectMember}
      >
        <UsersRound size={15} aria-hidden="true" /><span>选择成员</span>
      </button>
      {activeProjectId && activeChannelId && (
        <button className={styles.textBtn} type="button" title="刷新消息" onClick={refreshMessages}>
          <RefreshCw size={15} aria-hidden="true" /><span>刷新</span>
        </button>
      )}
      <button className={styles.textBtn} type="button" title="分享这台电脑的算力并查看连接状态" onClick={onNavigateNode}>
        <Cpu size={15} aria-hidden="true" /><span>分享算力</span>
      </button>
      <button className={styles.textBtn} type="button" title="打开移动端入口" onClick={() => window.open('/app/download', '_blank', 'noopener')}>
        <Smartphone size={15} aria-hidden="true" /><span>移动端</span>
      </button>
      <button className={styles.textBtn} type="button" title="切换到旧版 PC 工作台" onClick={openLegacyPc}>
        <History size={15} aria-hidden="true" /><span>旧版</span>
      </button>
    </div>
  )
}

function openLegacyPc() {
  const token = useAuthStore.getState().token
  if (token) {
    localStorage.setItem('lodex_token', token)
    localStorage.setItem('elon_token', token)
  }
  window.open('/pc-legacy', '_blank', 'noopener')
}
