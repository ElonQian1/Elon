import { ArrowRight, KeyRound, MessageCircleMore, ShieldCheck } from 'lucide-react'
import { Link } from 'react-router-dom'
import styles from './ChatGptAccountCard.module.css'

export default function OpenAiChatKitCard() {
  return (
    <article className={styles.card}>
      <header className={styles.header}>
        <span className={styles.icon} aria-hidden="true"><MessageCircleMore size={21} /></span>
        <div>
          <strong>OpenAI ChatKit（API 聊天）</strong>
          <small>使用一龙账号进入，不需要再次登录 ChatGPT</small>
        </div>
        <span className={styles.status}>平台配置</span>
      </header>

      <ol className={styles.steps}>
        <li><span>1</span><strong>登录当前一龙账号</strong></li>
        <li><span>2</span><strong>由服务端签发短时 ChatKit 会话</strong></li>
        <li><span>3</span><strong>在一龙界面使用官方 ChatKit</strong></li>
      </ol>

      <p className={styles.privacy}>
        <ShieldCheck size={15} aria-hidden="true" />
        <span><KeyRound size={13} aria-hidden="true" /> OpenAI API Key 不下发客户端；不读取 ChatGPT Cookie、历史或 Plus 额度。</span>
      </p>

      <Link className={styles.open} to="/chatkit">
        打开 ChatKit
        <ArrowRight size={16} aria-hidden="true" />
      </Link>
    </article>
  )
}
