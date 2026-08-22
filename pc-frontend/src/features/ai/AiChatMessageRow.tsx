import type { User } from '../../store/auth'
import { displayMessageContentOrAttachment } from '../../lib/messageDisplay'
import { formatTime } from '../../lib/utils'
import { forkAiConversation, forkTitleFromContent } from '../conversation/conversationForkApi'
import MarkdownContent from '../markdown/MarkdownContent'
import MessageActions, { messageActionsHostClassName, messageCopySourceId } from '../message-actions/MessageActions'
import UserAvatar from '../shell/UserAvatar'
import AiWebProviderAvatar, { aiWebProviderDisplayName } from '../user-browser/AiWebProviderAvatar'
import styles from './AiChatPage.module.css'
import AiStructuredContent, { type AiStructuredPart } from './AiStructuredContent'
import AiSourceLinks from './AiSourceLinks'
import { hasVisibleAiMessageContent } from './aiMessageVisibility'

export interface AiMessage {
  id?: string
  role: 'user' | 'assistant' | 'system'
  content: string
  content_format?: 'plain' | 'markdown'
  created_at?: string
  node_exec?: boolean
  node_display_name?: string
  node_remote?: boolean
  exit_ok?: boolean
  model?: string
  assistant_mode?: 'deterministic' | 'model' | 'handoff'
  tool_used?: string | null
  assistant_provider_id?: string
  sources?: AiSource[]
  handoff?: AiHandoff | null
  structured_parts?: AiStructuredPart[]
}

export interface AiSource {
  title: string
  url: string
  icon_url?: string
}

export interface AiProjectCandidate {
  id: string
  name: string
  description?: string | null
}

export interface AiHandoff {
  request: string
  reason: string
  candidates: AiProjectCandidate[]
}

interface AiChatMessageRowProps {
  activeConvId: string
  index: number
  message: AiMessage
  user: User | null
  streaming?: boolean
  streamingStatus?: string
  onConversationForked?: (conversationId: string) => void | Promise<void>
  onProjectHandoff?: (handoff: AiHandoff, candidate?: AiProjectCandidate) => void | Promise<void>
  onRegenerate?: () => void | Promise<void>
}

export default function AiChatMessageRow({
  activeConvId,
  index,
  message,
  user,
  streaming = false,
  streamingStatus = '正在生成回答…',
  onConversationForked,
  onProjectHandoff,
  onRegenerate,
}: AiChatMessageRowProps) {
  const isUser = message.role === 'user'
  const isNode = !isUser && message.node_exec === true
  const content = displayMessageContentOrAttachment(message.content)
  const hasVisibleContent = hasVisibleAiMessageContent(content)
  const hasMarkdown = !isUser && (
    message.content_format === 'markdown' || /[#*`\[\]>|]/.test(content)
  )
  const messageActionKey = message.id ?? `${activeConvId}:${message.role}:${message.created_at ?? index}:${content.slice(0, 80)}`
  const copySourceId = messageCopySourceId('ai-chat', messageActionKey)
  const nodePrefix = message.node_remote ? '远程' : '本机'
  const webProviderName = aiWebProviderDisplayName(message.assistant_provider_id)
  const nameLabel = isUser
    ? (user?.nickname ?? user?.account ?? '我')
    : (isNode ? `${nodePrefix} · ${message.node_display_name ?? ''}` : webProviderName ?? 'AI')
  const canFork = !!activeConvId && !!message.id

  return (
    <div className={[styles.msgRow, messageActionsHostClassName, isUser ? styles.ownRow : ''].join(' ')}>
      {isUser
        ? <UserAvatar user={user} size="compact" className={styles.avatar} />
        : <div
            className={[styles.avatar, isNode ? styles.nodeAvatar : '', webProviderName ? styles.providerAvatar : ''].join(' ')}
            role={webProviderName ? 'img' : undefined}
            aria-label={webProviderName ? `${webProviderName} 头像` : undefined}
            data-provider={webProviderName ? message.assistant_provider_id : undefined}
          >
            {isNode ? '\u{1F5A5}\uFE0F' : webProviderName
              ? <AiWebProviderAvatar providerId={message.assistant_provider_id} />
              : 'AI'}
          </div>}
      <div className={styles.msgBody}>
        <div className={styles.msgMeta}>
          <strong className={isNode ? styles.nodeLabel : ''}>{nameLabel}</strong>
          {message.created_at && <span>{formatTime(message.created_at)}</span>}
          {isNode && message.model && <span className={styles.modelTag}>{message.model}</span>}
          {isNode && message.exit_ok === false && <span className={styles.exitFail}>执行失败</span>}
          {!isUser && !isNode && message.tool_used && (
            <span className={styles.toolTag}>{message.tool_used === 'web_search' ? '已联网查询' : message.tool_used === 'calculator' ? '计算器' : message.tool_used === 'current_datetime' ? '实时时间' : '已使用工具'}</span>
          )}
        </div>
        {streaming && !hasVisibleContent
          ? <div className={styles.typing} aria-live="polite">
            <span className={styles.typingText}>{streamingStatus}</span>
            <span className={styles.typingDot} /><span className={styles.typingDot} /><span className={styles.typingDot} />
          </div>
          : hasVisibleContent && (hasMarkdown
          ? <div id={copySourceId} className={styles.msgContent}>
            <MarkdownContent content={content} copy citations={message.sources} />
          </div>
          : <div id={copySourceId} className={styles.msgContent}>{content}</div>)}
        {!isUser && <AiSourceLinks sources={message.sources} />}
        {!isUser && <AiStructuredContent parts={message.structured_parts} />}
        {!isUser && message.handoff && (
          <div className={styles.handoffCard}>
            <strong>继续到项目 AI</strong>
            <span>{message.handoff.reason}</span>
            {message.handoff.candidates.length > 0 ? (
              <div className={styles.handoffCandidates}>
                {message.handoff.candidates.map((candidate) => (
                  <button
                    key={candidate.id}
                    type="button"
                    onClick={() => { void onProjectHandoff?.(message.handoff!, candidate) }}
                  >
                    {candidate.name}
                  </button>
                ))}
              </div>
            ) : (
              <button type="button" onClick={() => { void onProjectHandoff?.(message.handoff!) }}>
                打开项目列表
              </button>
            )}
          </div>
        )}
        {!streaming && hasVisibleContent && <MessageActions
          content={content}
          messageKey={messageActionKey}
          storageScope="ai-chat"
          richCopySourceId={copySourceId}
          align={isUser ? 'right' : 'left'}
          onFork={canFork ? async () => {
            const fork = await forkAiConversation(activeConvId, message.id!, forkTitleFromContent(content))
            await onConversationForked?.(fork.conversation_id)
          } : undefined}
          onRegenerate={!isUser ? onRegenerate : undefined}
        />}
      </div>
    </div>
  )
}
