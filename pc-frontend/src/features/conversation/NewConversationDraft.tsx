import { Bug, Code2, ListRestart, ScanSearch, Sparkles } from 'lucide-react'
import styles from './NewConversationDraft.module.css'

interface NewConversationDraftProps {
  projectName: string
  channelName: string
  localNodeReady: boolean
  usesLocalNode: boolean
  onChoosePrompt: (prompt: string) => void
}

const STARTERS = [
  {
    icon: Code2,
    title: '开发新功能',
    detail: '从一个清晰的产品目标开始',
    prompt: '请帮我开发一个新功能：',
  },
  {
    icon: Bug,
    title: '修复一个问题',
    detail: '描述现象，或直接粘贴截图',
    prompt: '请帮我定位并修复这个问题：',
  },
  {
    icon: ListRestart,
    title: '继续未完成任务',
    detail: '先检查现状，再接着完成',
    prompt: '请继续处理这个项目中尚未完成的任务。先检查当前进度，再告诉我下一步。',
  },
  {
    icon: ScanSearch,
    title: '分析当前项目',
    detail: '了解结构、状态与主要风险',
    prompt: '请先分析当前项目的结构、状态和主要风险，再给出最值得优先处理的建议。',
  },
] as const

export default function NewConversationDraft({
  projectName,
  channelName,
  localNodeReady,
  usesLocalNode,
  onChoosePrompt,
}: NewConversationDraftProps) {
  const nodeLabel = usesLocalNode
    ? localNodeReady ? '本机节点已连接' : '本机节点未连接'
    : '运行路线已准备'

  return (
    <section className={styles.canvas} aria-labelledby="new-conversation-title">
      <div className={styles.content}>
        <div className={styles.eyebrow}><span />新对话草稿</div>
        <div className={styles.heroIcon}><Sparkles size={30} aria-hidden="true" /></div>
        <h1 id="new-conversation-title">准备开始新的开发</h1>
        <p className={styles.lead}>选择一个起点，或直接在下方描述功能、问题和预期结果。</p>

        <div className={styles.contextRow} aria-label="当前开发上下文">
          <span>{projectName}</span>
          <span>{channelName}</span>
          <span data-tone={localNodeReady || !usesLocalNode ? 'ready' : 'warning'}>{nodeLabel}</span>
        </div>

        <div className={styles.starterHead}>
          <strong>从这里开始</strong>
          <span>选择后仍可继续编辑</span>
        </div>
        <div className={styles.starterGrid}>
          {STARTERS.map(({ icon: Icon, title, detail, prompt }) => (
            <button key={title} type="button" onClick={() => onChoosePrompt(prompt)}>
              <span className={styles.starterIcon}><Icon size={19} aria-hidden="true" /></span>
              <span className={styles.starterCopy}>
                <strong>{title}</strong>
                <small>{detail}</small>
              </span>
              <span className={styles.starterArrow}>→</span>
            </button>
          ))}
        </div>

        <div className={styles.draftNote}>
          <Sparkles size={14} aria-hidden="true" />
          <span><strong>发送第一条消息后才会创建真实会话</strong>，空草稿不会出现在会话记录中。</span>
        </div>
      </div>
    </section>
  )
}
