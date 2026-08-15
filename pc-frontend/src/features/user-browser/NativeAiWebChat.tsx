import { MessageSquarePlus, MonitorUp, Send, Square } from 'lucide-react'
import type {
  LocalAiAdapterAction,
  LocalAiMessageSnapshot,
  LocalAiWebProvider,
} from './localAiBrowserApi'
import type { LocalAiUserState } from './localAiUserState'
import styles from './LocalAiBrowserPanel.module.css'

interface NativeAiWebChatProps {
  provider: LocalAiWebProvider
  userState: LocalAiUserState
  snapshot: LocalAiMessageSnapshot | null
  busy: boolean
  draft: string
  onDraftChange: (value: string) => void
  onRun: (action: LocalAiAdapterAction, value?: string, expectedDraft?: string) => void
  standalone?: boolean
  emptyTitle?: string
}

export default function NativeAiWebChat({
  provider,
  userState,
  snapshot,
  busy,
  draft,
  onDraftChange,
  onRun,
  standalone = false,
  emptyTitle,
}: NativeAiWebChatProps) {
  const canCompose = userState.canSend
  const providerName = provider.displayName

  return (
    <section
      className={styles.nativeChat}
      data-standalone={standalone}
      aria-label={`一龙 ${providerName} 原生聊天区`}
    >
      <header>
        <div>
          <strong>{snapshot?.title || `${providerName} · 一龙界面`}</strong>
          <small>
            {snapshot?.currentModel && userState.canSend
              ? `${snapshot.currentModel} · 本机同步`
              : userState.title}
          </small>
        </div>
        <button
          type="button"
          title="新建对话"
          onClick={() => onRun('new_conversation')}
          disabled={!userState.canNewConversation || busy}
        >
          <MessageSquarePlus size={17} />
        </button>
      </header>

      <div className={styles.messageList} aria-live="polite">
        {snapshot?.messages.length ? snapshot.messages.map((item) => (
          <article className={item.role === 'user' ? styles.userMessage : styles.assistantMessage} key={item.id}>
            <span>{item.role === 'user' ? '你' : providerName}</span>
            {item.content.map((part, index) => part.type === 'text' || part.type === 'markdown' ? (
              <p key={`${item.id}-${index}`}>{part.text}</p>
            ) : part.type === 'citation' && part.url ? (
              <a
                className={styles.citation}
                href={part.url}
                key={`${item.id}-${index}`}
                target="_blank"
                rel="noopener noreferrer"
              >
                {part.text || publicHost(part.url)}
              </a>
            ) : (
              <p key={`${item.id}-${index}`}>
                {`${part.text}${part.language ? ` · ${part.language}` : ''}`}
              </p>
            ))}
          </article>
        )) : (
          <div className={styles.emptyChat}>
            <MonitorUp size={24} />
            <strong>{emptyTitle || userState.title}</strong>
            <p>{userState.detail}</p>
            {provider.id === 'chatgpt' && userState.canStartGoogleLogin && (
              <button type="button" onClick={() => onRun('start_google_login')} disabled={busy}>
                尝试打开官方 Google 登录
              </button>
            )}
          </div>
        )}
      </div>

      <div className={styles.composer}>
        <textarea
          value={draft}
          onChange={(event) => onDraftChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== 'Enter' || event.shiftKey || event.nativeEvent.isComposing) return
            event.preventDefault()
            if (canCompose && draft.trim() && !busy) {
              onRun('send_prompt', draft, snapshot?.draft ?? '')
            }
          }}
          placeholder={canCompose ? `向 ${providerName} 发送消息…` : userState.detail}
          disabled={!canCompose || busy}
          maxLength={20_000}
        />
        {snapshot?.streaming ? (
          <button type="button" title="停止生成" onClick={() => onRun('stop_generation')} disabled={!userState.canStop || busy}>
            <Square size={16} />
          </button>
        ) : (
          <button
            type="button"
            title="发送"
            onClick={() => onRun('send_prompt', draft, snapshot?.draft ?? '')}
            disabled={!canCompose || !draft.trim() || busy}
          >
            <Send size={16} />
          </button>
        )}
      </div>
    </section>
  )
}

function publicHost(url: string) {
  try { return new URL(url).hostname }
  catch { return url }
}
