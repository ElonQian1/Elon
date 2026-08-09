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
}

export default function NativeAiWebChat({
  provider,
  snapshot,
  sessionOpen,
  busy,
  draft,
  onDraftChange,
  onRun,
}: NativeAiWebChatProps) {
  return (
    <section className={styles.nativeChat} aria-label="一龙 ChatGPT 原生聊天区">
      <header>
        <div>
          <strong>{snapshot?.title || 'ChatGPT 原生聊天'}</strong>
          <small>
            {snapshot?.authenticated
              ? `${snapshot.currentModel || '官方网页模型'} · 本机同步`
              : '请先在官方窗口登录'}
          </small>
        </div>
        <button
          type="button"
          title="新建对话"
          onClick={() => onRun('new_conversation')}
          disabled={!snapshot?.authenticated || busy}
        >
          <MessageSquarePlus size={17} />
        </button>
      </header>

      <div className={styles.messageList} aria-live="polite">
        {snapshot?.messages.length ? snapshot.messages.map((item) => (
          <article className={item.role === 'user' ? styles.userMessage : styles.assistantMessage} key={item.id}>
            <span>{item.role === 'user' ? '你' : 'ChatGPT'}</span>
            {item.content.map((part, index) => <p key={`${item.id}-${index}`}>{part.text}</p>)}
          </article>
        )) : (
          <div className={styles.emptyChat}>
            <MonitorUp size={24} />
            <strong>{sessionOpen ? '等待 ChatGPT 官方页面' : '尚未打开 ChatGPT'}</strong>
            <p>完成官方登录后，可见对话会自动同步到这里；遇到真人验证请在官方窗口本人点击。</p>
            {sessionOpen && !snapshot?.authenticated && (
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
          placeholder={snapshot?.composerReady ? '向 ChatGPT 发送消息…' : '登录后即可使用原生输入框'}
          disabled={!snapshot?.authenticated || !snapshot.composerReady || busy}
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
            disabled={!snapshot?.composerReady || !draft.trim() || busy}
          >
            <Send size={16} />
          </button>
        )}
      </div>
    </section>
  )
}
