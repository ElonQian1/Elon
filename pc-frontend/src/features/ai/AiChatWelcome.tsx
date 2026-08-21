import type { AiWebChatBackend } from '../user-browser/useAiWebChatBackend'
import styles from './AiChatPage.module.css'

interface AiChatWelcomeProps {
  chatMode: boolean
  identityReady: boolean
  onlineNodeId: string
  onlineNodeName: string
  sending: boolean
  web: AiWebChatBackend
  onLogin: () => void
}

export default function AiChatWelcome({
  chatMode,
  identityReady,
  onlineNodeId,
  onlineNodeName,
  sending,
  web,
  onLogin,
}: AiChatWelcomeProps) {
  return (
    <div className={styles.welcome}>
      <h2>你好，我是一龙 AI</h2>
      <p>{!identityReady
        ? '正在建立本机访客会话或读取账号身份…'
        : chatMode
          ? `${web.provider?.displayName || '官方网页 AI'} 是当前消息来源；访客可用时直接聊天，登录只用于历史、项目和增强能力。`
          : onlineNodeId
            ? `本机「${onlineNodeName}」已就绪，直接输入需求或命令。`
            : '随时可以开始对话，我会记住我们聊过的内容。'}</p>
      {!identityReady && (
        <div className={styles.loginPrompt}>
          <button className={styles.startBtn} type="button" onClick={onLogin}>登录账号</button>
          <span>也可以稍候使用本机访客身份；登录后还可同步项目、好友和电脑节点。</span>
        </div>
      )}
      {identityReady && chatMode && !web.canCompose
        && !web.controller.newConversationRecoveryActive
        && !['official_loading', 'adapter_waiting', 'context_restoring'].includes(web.userState.phase) && (
        <div className={styles.loginPrompt}>
          <button
            className={styles.startBtn}
            type="button"
            onClick={() => void web.controller.openOfficial()}
            disabled={!web.ready || sending}
          >
            {web.userState.phase === 'login_required' ? '登录 / 打开官方页' : '显示官方页处理'}
          </button>
          <span>{web.userState.detail}</span>
        </div>
      )}
    </div>
  )
}
