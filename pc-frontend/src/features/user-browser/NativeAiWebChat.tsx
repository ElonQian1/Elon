import { MessageSquarePlus, MonitorUp, Send, Square } from 'lucide-react'
import type {
  LocalAiAdapterAction,
  LocalAiMessageSnapshot,
  LocalAiWebProvider,
} from './localAiBrowserApi'
import styles from './LocalAiBrowserPanel.module.css'

interface NativeAiWebChatProps {
  provider: LocalAiWebProvider
  snapshot: LocalAiMessageSnapshot | null
  sessionOpen: boolean
  busy: boolean
  draft: string
  onDraftChange: (value: string) => void
  onRun: (action: LocalAiAdapterAction, value?: string, expectedDraft?: string) => void
  standalone?: boolean
  emptyTitle?: string
}

export default function NativeAiWebChat({
  provider,
  snapshot,
  sessionOpen,
  busy,
  draft,
  onDraftChange,
  onRun,
  standalone = false,
  emptyTitle,
}: NativeAiWebChatProps) {
  const canCompose = Boolean(snapshot?.composerReady)
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
            {snapshot?.authenticated
              ? `${snapshot.currentModel || '官方网页模型'} · 本机同步`
              : snapshot?.composerReady
                ? '官方访客模式 · 本机同步'
                : '正在检测访客能力 · 登录可选'}
          </small>
        </div>
        <button
          type="button"
          title="新建对话"
          onClick={() => onRun('new_conversation')}
          disabled={!canCompose || busy}
        >
          <MessageSquarePlus size={17} />
        </button>
      </header>

      <div className={styles.messageList} aria-live="polite">
        {snapshot?.messages.length ? snapshot.messages.map((item) => (
          <article className={item.role === 'user' ? styles.userMessage : styles.assistantMessage} key={item.id}>
            <span>{item.role === 'user' ? '你' : providerName}</span>
            {item.content.map((part, index) => part.type === 'text' ? (
              <p key={`${item.id}-${index}`}>{part.text}</p>
            ) : (
              <a
                className={styles.citation}
                href={part.url}
                key={`${item.id}-${index}`}
                target="_blank"
                rel="noopener noreferrer"
              >
                {part.title || new URL(part.url).hostname}
              </a>
            ))}
          </article>
        )) : (
          <div className={styles.emptyChat}>
            <MonitorUp size={24} />
            <strong>{emptyTitle || (sessionOpen ? `等待 ${providerName} 官方页面` : `尚未打开 ${providerName}`)}</strong>
            <p>
              官网提供访客输入框时会直接启用；需要历史、项目或官网要求验证时，再显示官方窗口登录。
            </p>
            {provider.id === 'chatgpt' && sessionOpen && !snapshot?.authenticated && (
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
          placeholder={canCompose ? `向 ${providerName} 发送消息…` : '官方页面就绪后即可使用原生输入框'}
          disabled={!canCompose || busy}
          maxLength={20_000}
        />
        {snapshot?.streaming ? (
          <button type="button" title="停止生成" onClick={() => onRun('stop_generation')} disabled={busy}>
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
