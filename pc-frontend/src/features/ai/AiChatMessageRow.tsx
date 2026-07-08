import type { User } from '../../store/auth'
import { displayMessageContentOrAttachment } from '../../lib/messageDisplay'
import { formatTime } from '../../lib/utils'
import { forkAiConversation, forkTitleFromContent } from '../conversation/conversationForkApi'
import MarkdownContent from '../markdown/MarkdownContent'
import MessageActions, { messageCopySourceId } from '../message-actions/MessageActions'
import UserAvatar from '../shell/UserAvatar'
import styles from './AiChatPage.module.css'

export interface AiMessage {
  id?: string
  role: 'user' | 'assistant' | 'system'
  content: string
  created_at?: string
  node_exec?: boolean
  node_display_name?: string
  node_remote?: boolean
  exit_ok?: boolean
  model?: string
}

interface AiChatMessageRowProps {
  activeConvId: string
  index: number
  message: AiMessage
  user: User | null
  onConversationForked?: (conversationId: string) => void | Promise<void>
}

export default function AiChatMessageRow({
  activeConvId,
  index,
  message,
  user,
  onConversationForked,
}: AiChatMessageRowProps) {
  const isUser = message.role === 'user'
  const isNode = !isUser && message.node_exec === true
  const content = displayMessageContentOrAttachment(message.content)
  const hasMarkdown = !isUser && /[#*`\[\]>|]/.test(content)
  const messageActionKey = message.id ?? `${activeConvId}:${message.role}:${message.created_at ?? index}:${content.slice(0, 80)}`
  const copySourceId = messageCopySourceId('ai-chat', messageActionKey)
  const nodePrefix = message.node_remote ? '远程' : '本机'
  const nameLabel = isUser
    ? (user?.nickname ?? user?.account ?? '我')
    : (isNode ? `${nodePrefix} · ${message.node_display_name ?? ''}` : 'AI')
  const canFork = !!activeConvId && !!message.id

  return (
    <div className={[styles.msgRow, isUser ? styles.ownRow : ''].join(' ')}>
      {isUser
        ? <UserAvatar user={user} size="compact" className={styles.avatar} />
        : <div className={[styles.avatar, isNode ? styles.nodeAvatar : ''].join(' ')}>{isNode ? '\u{1F5A5}\uFE0F' : 'AI'}</div>}
      <div className={styles.msgBody}>
        <div className={styles.msgMeta}>
          <strong className={isNode ? styles.nodeLabel : ''}>{nameLabel}</strong>
          {message.created_at && <span>{formatTime(message.created_at)}</span>}
          {isNode && message.model && <span className={styles.modelTag}>{message.model}</span>}
          {isNode && message.exit_ok === false && <span className={styles.exitFail}>执行失败</span>}
        </div>
        {hasMarkdown
          ? <div id={copySourceId} className={styles.msgContent}><MarkdownContent content={content} copy /></div>
          : <div id={copySourceId} className={styles.msgContent}>{content}</div>}
        <MessageActions
          content={content}
          messageKey={messageActionKey}
          storageScope="ai-chat"
          richCopySourceId={copySourceId}
          align={isUser ? 'right' : 'left'}
          onFork={canFork ? async () => {
            const fork = await forkAiConversation(activeConvId, message.id!, forkTitleFromContent(content))
            await onConversationForked?.(fork.conversation_id)
          } : undefined}
        />
      </div>
    </div>
  )
}
