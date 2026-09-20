import { ExternalLink, LockKeyhole, MessageSquareText } from 'lucide-react'
import { Link } from 'react-router-dom'
import useLocalAiBrowserCapability from '../user-browser/useLocalAiBrowserCapability'
import styles from './ChatGptAccountCard.module.css'

export default function ChatGptAccountCard() {
  const localBrowser = useLocalAiBrowserCapability()
  const desktopAvailable = localBrowser.state === 'ready'
  const status = localBrowser.state === 'upgrade_required'
    ? '需更新 Win 客户端'
    : localBrowser.state === 'error'
      ? '本地功能异常'
    : localBrowser.state === 'checking'
      ? '正在检查 Win 能力'
      : desktopAvailable ? 'Win 本地可用' : '需要 Win 客户端'

  return (
    <article className={styles.card}>
      <header className={styles.header}>
        <span className={styles.icon} aria-hidden="true"><MessageSquareText size={21} /></span>
        <div>
          <strong>OpenAI / ChatGPT 账户</strong>
          <small>在本机官网确认当前账户，或退出后登录另一账户</small>
        </div>
        <span className={styles.status} data-ready={desktopAvailable}>
          {status}
        </span>
      </header>

      <ol className={styles.steps}>
        <li><span>1</span><strong>打开官方 ChatGPT</strong></li>
        <li><span>2</span><strong>本人完成登录和真人验证</strong></li>
        <li><span>3</span><strong>直接开始聊天</strong></li>
      </ol>

      <p className={styles.privacy}>
        <LockKeyhole size={15} aria-hidden="true" />
        Cookie 和网页登录数据只保存在这台电脑，不会上传到一龙云端。
      </p>

      <Link className={styles.open} to="/user-browser">
        查看／切换 OpenAI 账户
        <ExternalLink size={16} aria-hidden="true" />
      </Link>
    </article>
  )
}
