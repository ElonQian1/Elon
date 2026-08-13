import { EyeOff, MonitorUp, ShieldCheck } from 'lucide-react'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import styles from './AiWebChatSidebar.module.css'

export default function AiWebChatSidebar({ web }: { web: AiWebChatBackend }) {
  const busy = Boolean(web.controller.busyAction)
  const officialVisible = Boolean(web.controller.sessionState?.windowVisible)

  return (
    <>
      <section className={styles.quickActions} aria-label="网页 AI 会话操作">
        <button type="button" onClick={() => void web.controller.openOfficial()} disabled={!web.ready || busy}>
          <MonitorUp size={16} />
          <span><strong>{web.controller.sessionOpen ? '显示官方登录页' : '登录 / 打开官方页'}</strong><small>仅登录、验证或故障回退时显示</small></span>
        </button>
        <button
          type="button"
          onClick={() => void web.controller.control('background')}
          disabled={!web.ready || !officialVisible || busy}
        >
          <EyeOff size={16} />
          <span><strong>收起官方页到后台</strong><small>继续使用当前一龙聊天界面</small></span>
        </button>
      </section>
      <div className={styles.providerPane}>
        <div className={styles.heading}>当前 Chat 来源</div>
        {web.providers.map((provider) => (
          <button
            className={styles.provider}
            data-active={provider.id === web.provider?.id}
            key={provider.id}
            type="button"
            onClick={() => web.selectProvider(provider.id)}
          >
            <span className={styles.logo}>{provider.id === 'chatgpt' ? '◎' : 'G'}</span>
            <span><strong>{provider.displayName}</strong><small>{provider.id === 'chatgpt' ? '官方网页 Chat' : 'Google 搜索 AI 模式'}</small></span>
          </button>
        ))}
        <div className={styles.status} data-error={Boolean(web.controller.sessionState?.lastError)}>
          <strong>{web.status}</strong>
          {web.message && <span>{web.message}</span>}
        </div>
        <p className={styles.privacy}><ShieldCheck size={14} />Cookie 仅保存在这台电脑的 WebView2 Profile</p>
      </div>
    </>
  )
}
