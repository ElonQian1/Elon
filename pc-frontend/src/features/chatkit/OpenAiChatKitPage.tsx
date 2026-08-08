import { useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { ArrowLeft, BadgeCheck, LockKeyhole, RefreshCw } from 'lucide-react'
import { api } from '../../api/client'
import styles from './OpenAiChatKitPage.module.css'

interface ChatKitCapability {
  configured: boolean
  implementation_state: 'available' | 'configuration_required'
  integration_mode: 'hosted_workflow_transition'
  message: string
  official_docs: string
  transition: {
    agent_builder_shutdown_date: string
    recommended_new_architecture: string
  }
}

interface ChatKitElement extends HTMLElement {
  setOptions(options: {
    api: { getClientSecret: (existing?: string) => Promise<string> }
    theme: 'dark'
  }): void
}

interface SessionResponse {
  client_secret: string
}

export default function OpenAiChatKitPage() {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const [capability, setCapability] = useState<ChatKitCapability | null>(null)
  const [status, setStatus] = useState('正在检查 ChatKit 配置…')
  const [error, setError] = useState('')
  const [revision, setRevision] = useState(0)

  useEffect(() => {
    let active = true
    setError('')
    setStatus('正在检查 ChatKit 配置…')
    api.get<ChatKitCapability>('/api/openai-chatkit/capability')
      .then((value) => {
        if (!active) return
        setCapability(value)
        setStatus(value.configured ? '已使用当前一龙账号连接官方 ChatKit API' : value.message)
      })
      .catch((reason: { message?: string }) => {
        if (!active) return
        setError(reason.message || '无法读取 ChatKit 配置')
      })
    return () => { active = false }
  }, [revision])

  useEffect(() => {
    const host = hostRef.current
    if (!host || !capability?.configured) return
    let disposed = false
    let chatkit: ChatKitElement | null = null

    async function mountChatKit() {
      try {
        await Promise.race([
          window.customElements.whenDefined('openai-chatkit'),
          new Promise((_, reject) => window.setTimeout(() => reject(new Error('ChatKit 组件加载超时')), 12_000)),
        ])
        if (disposed || !host) return
        chatkit = document.createElement('openai-chatkit') as ChatKitElement
        chatkit.className = styles.chatkit
        chatkit.addEventListener('chatkit.response.start', handleResponseStart)
        chatkit.addEventListener('chatkit.response.end', handleResponseEnd)
        chatkit.addEventListener('chatkit.error', handleChatKitError)
        chatkit.setOptions({
          api: {
            async getClientSecret() {
              const session = await api.post<SessionResponse>('/api/openai-chatkit/session', {})
              if (!session.client_secret) throw new Error('服务器没有返回 ChatKit 会话')
              return session.client_secret
            },
          },
          theme: 'dark',
        })
        host.replaceChildren(chatkit)
        setStatus('ChatKit 已就绪 · 使用一龙账号与平台 API')
      } catch (reason) {
        if (disposed) return
        setError(reason instanceof Error ? reason.message : 'ChatKit 加载失败')
      }
    }

    function handleResponseStart() { setStatus('OpenAI 正在回复…') }
    function handleResponseEnd() { setStatus('回复完成') }
    function handleChatKitError(event: Event) {
      const detail = (event as CustomEvent<{ error?: Error }>).detail
      setError(detail?.error?.message || 'ChatKit 会话发生错误')
    }

    void mountChatKit()
    return () => {
      disposed = true
      if (chatkit) {
        chatkit.removeEventListener('chatkit.response.start', handleResponseStart)
        chatkit.removeEventListener('chatkit.response.end', handleResponseEnd)
        chatkit.removeEventListener('chatkit.error', handleChatKitError)
      }
      host.replaceChildren()
    }
  }, [capability])

  return (
    <main className={styles.page}>
      <header className={styles.header}>
        <Link className={styles.back} to="/account" aria-label="返回账号设置"><ArrowLeft size={18} /></Link>
        <div>
          <span className={styles.eyebrow}>OPENAI · API CHAT</span>
          <h1>OpenAI ChatKit</h1>
          <p>使用一龙账号进入，聊天费用与能力来自平台配置的 OpenAI API。</p>
        </div>
        <button className={styles.refresh} type="button" onClick={() => setRevision((value) => value + 1)}>
          <RefreshCw size={16} />重新检查
        </button>
      </header>

      <section className={styles.boundary}>
        <span><BadgeCheck size={16} />当前身份：一龙账号</span>
        <span><LockKeyhole size={16} />不读取 ChatGPT Cookie、历史或 Plus 额度</span>
      </section>

      <div className={styles.status} data-error={Boolean(error)} role="status">
        {error || status}
      </div>

      {!capability?.configured && !error && (
        <section className={styles.empty}>
          <strong>管理员尚未完成 ChatKit 配置</strong>
          <p>需要在服务端配置 OpenAI API Key 和现有 ChatKit workflow。当前官方建议的新项目后续迁移到自托管 ChatKit Server。</p>
          <a href={capability?.official_docs} target="_blank" rel="noreferrer">查看 OpenAI 官方 ChatKit 文档</a>
        </section>
      )}

      {capability?.configured && <div className={styles.chatHost} ref={hostRef} />}

      <footer className={styles.footer}>
        这不是 ChatGPT 官网账号登录；ChatGPT 网页会话仍在独立的“ChatGPT 网页”入口中。
      </footer>
    </main>
  )
}
