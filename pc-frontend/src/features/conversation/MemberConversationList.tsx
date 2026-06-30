import { Plus, User } from 'lucide-react'
import { formatTime } from '../../lib/utils'
import type { MemberConversationEntry } from './memberConversationApi'
import styles from './MemberConversationList.module.css'

interface Props {
  conversations: MemberConversationEntry[]
  selectedId: string | 'new' | null
  targetName: string
  isOwnTarget: boolean
  onOpen: (conversationId: string) => void
  onStartNew: () => void
  onResetTarget: () => void
}

export default function MemberConversationList({
  conversations,
  selectedId,
  targetName,
  isOwnTarget,
  onOpen,
  onStartNew,
  onResetTarget,
}: Props) {
  return (
    <section className={styles.section} aria-label={`${targetName} 的会话`}>
      <div className={styles.header}>
        <div className={styles.headerCopy}>
          <span>{isOwnTarget ? '我的会话' : `${targetName} 的会话`}</span>
          {!isOwnTarget && <em>以你的账号继续协助</em>}
        </div>
        <div className={styles.actions}>
          {!isOwnTarget && (
            <button
              className={styles.actionBtn}
              type="button"
              onClick={onResetTarget}
              title="回到我的会话"
              aria-label="回到我的会话"
            >
              <User size={14} strokeWidth={2.2} aria-hidden="true" />
            </button>
          )}
          {isOwnTarget && (
            <button
              className={[styles.actionBtn, selectedId === 'new' ? styles.actionBtnActive : ''].join(' ')}
              type="button"
              onClick={onStartNew}
              title="新建会话"
              aria-label="新建会话"
            >
              <Plus size={15} strokeWidth={2.4} aria-hidden="true" />
            </button>
          )}
        </div>
      </div>

      {conversations.length === 0 && (
        <div className={styles.empty}>
          {isOwnTarget ? '发送第一条消息自动创建会话' : '该成员暂无可见会话'}
        </div>
      )}

      <div className={styles.list}>
        {conversations.map((conversation) => {
          const failed = conversation.last_task_status === 'error' || conversation.last_task_status === 'failed'
          const title = conversation.title || conversation.last_message || '新会话'
          const active = conversation.id === selectedId
          return (
            <button
              key={conversation.id}
              type="button"
              className={[styles.item, active ? styles.itemActive : ''].join(' ')}
              onClick={() => onOpen(conversation.id)}
            >
              <span className={styles.itemTitle}>
                {failed ? '✗ ' : ''}{title.slice(0, 40)}
              </span>
              <span className={styles.itemMeta}>
                {conversation.updated_at ? formatTime(conversation.updated_at) : '未更新'}
                {typeof conversation.message_count === 'number' && ` · ${conversation.message_count} 条`}
              </span>
            </button>
          )
        })}
      </div>
    </section>
  )
}
