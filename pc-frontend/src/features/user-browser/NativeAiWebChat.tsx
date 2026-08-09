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
}: NativeAiWebChatProps) {
  const guestMode = provider.loginMode === 'guest_web_system_login'
  const canCompose = Boolean(snapshot?.composerReady && (snapshot.authenticated || guestMode))
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
              : guestMode && snapshot?.composerReady
                ? '官方访客模式 · 本机同步'
                : guestMode
                  ? '请在官方窗口确认 AI 模式可用'
                  : '请先在官方窗口登录'}
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
            <strong>{sessionOpen ? `等待 ${providerName} 官方页面` : `尚未打开 ${providerName}`}</strong>
            <p>
              {guestMode
                ? 'AI 模式可用后，可见问题、回答和来源会同步到这里；地区或账号未开放时请使用官方窗口。'
                : '完成官方登录后，可见对话会自动同步到这里；遇到真人验证请在官方窗口本人点击。'}
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
