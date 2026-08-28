import { Suspense } from 'react'
import type { User } from '../../store/auth'
import useAiWebChatBackend from '../user-browser/useAiWebChatBackend'
import {
  AiChatMessageRow,
} from './aiFeatureLazyComponents'
import type { AiHandoff, AiMessage, AiProjectCandidate } from './AiChatMessageRow'
import styles from './AiChatPage.module.css'

type AiWebChatBackend = ReturnType<typeof useAiWebChatBackend>

interface AiChatMessageRowsProps {
  messages: AiMessage[]
  chatMode: boolean
  activeConvId: string
  user: User | null
  web: AiWebChatBackend
  streamingMessageId: string | null
  streamingStatus: string
  lastVisibleAssistantId: string | null | undefined
  onConversationForked: (conversationId: string) => void | Promise<void>
  onProjectHandoff: (handoff: AiHandoff, candidate?: AiProjectCandidate) => void | Promise<void>
  onOpenOfficial: () => void
  onCheckUpdates: () => void
}

export default function AiChatMessageRows({
  messages,
  chatMode,
  activeConvId,
  user,
  web,
  streamingMessageId,
  streamingStatus,
  lastVisibleAssistantId,
  onConversationForked,
  onProjectHandoff,
  onOpenOfficial,
  onCheckUpdates,
}: AiChatMessageRowsProps) {
  const visibleMessages = messages.filter((message) => message.role !== 'system')
  if (visibleMessages.length === 0) return null

  return (
    <Suspense fallback={<p className={styles.hint}>正在加载消息…</p>}>
      {visibleMessages.map((message, index) => (
        <AiChatMessageRow
          key={message.id ?? `${message.role}:${message.created_at ?? index}`}
          activeConvId={chatMode ? '' : activeConvId}
          index={index}
          message={message}
          user={user}
          streaming={message.id === (chatMode ? web.streamingMessageId : streamingMessageId)}
          streamingStatus={chatMode ? web.streamingStatus : streamingStatus || '正在处理…'}
          onConversationForked={chatMode ? undefined : onConversationForked}
          onProjectHandoff={chatMode ? undefined : onProjectHandoff}
          onRegenerate={chatMode && message.id === lastVisibleAssistantId && web.provider?.adapterActions.includes('regenerate_response')
            ? async () => { await web.controller.run('regenerate_response') }
            : undefined}
          onOpenOfficial={chatMode && message.renderer_compatibility ? onOpenOfficial : undefined}
          onCheckUpdates={chatMode && message.renderer_compatibility ? onCheckUpdates : undefined}
        />
      ))}
    </Suspense>
  )
}
