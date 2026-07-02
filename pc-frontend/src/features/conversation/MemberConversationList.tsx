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
          const title = conversationDisplayTitle(conversation)
          const active = conversation.id === selectedId
          return (
            <button
              key={conversation.id}
              type="button"
              className={[styles.item, active ? styles.itemActive : ''].join(' ')}
              onClick={() => onOpen(conversation.id)}
            >
              <span className={styles.itemTitleRow}>
                <span className={styles.itemTitle}>{title}</span>
                {failed && <span className={styles.statusPill}>失败</span>}
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

function conversationDisplayTitle(conversation: MemberConversationEntry): string {
  const raw = String(conversation.title || conversation.last_message || '').trim()
  if (!raw) return '新会话'

  const normalized = raw
    .replace(/^MCP\s+Display\s*/i, 'MCP 验收 ')
    .replace(/\bmcp_display_e2e_\d+_\d+\b/gi, '')
    .replace(/\bmcp_native_e2e_\d+_\d+(?:_[a-z]+)?\b/gi, '')
    .replace(/\bpch_[a-f0-9]+\b/gi, '')
    .replace(/\bforce-cli-parallel-[ab]-\d+\b/gi, '并行任务测试')
    .replace(/^Force\s+CLI\s+cancellation\s+smoke\s+test\.?.*/i, 'CLI 取消验证')
    .replace(/\bpost-publish-casual(?:-lookup)?-\d+\b/gi, '发布后验证')
    .replace(/\bparallel\s+real\s+([ab])\s+\d+\b/gi, '并行会话 $1')
    .replace(/\bsingle\s+node\s+lock\s+\d+\b/gi, '单节点锁定验证')
    .replace(/^MCP\s+Native\s+Absolute\s+Pub\S*/i, 'MCP 原生发布验证')
    .replace(/\s+/g, ' ')
    .replace(/[·\-\s]+$/g, '')
    .trim()

  if (normalized) return normalized.slice(0, 34)
  if (/mcp/i.test(raw)) return 'MCP 验收会话'
  if (/pch_[a-f0-9]+/i.test(raw) || raw.includes('项目频道')) return '项目频道会话'
  return raw.slice(0, 34)
}
