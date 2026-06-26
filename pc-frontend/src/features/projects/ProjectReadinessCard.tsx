/**
 * P2.3：项目任务进度面板（开发者就绪状态 + 下一步引导）
 * 对应旧 pc_app_project_readiness.js 的 renderMemberPanel / buildReadiness
 */
import styles from './ProjectReadinessCard.module.css'

interface WorkspaceHealth {
  workspace_exists?: boolean
  git_initialized?: boolean
  git_remote?: string
  node_online?: boolean
  node_id?: string
  disk_free_bytes?: number
  issues?: string[]
  cli_ready?: boolean
  ai_cli_routes?: string[]
  runtime_route?: string
  workspace_provision_ready?: boolean
  dev_env_ready?: boolean
}

interface Channel {
  id: string
  name: string
  kind?: string
}

interface Props {
  health: WorkspaceHealth | null
  loading: boolean
  channels: Channel[]
  onOpenChannel: (channelId: string) => void
  onRefresh: () => void
}

interface Check {
  label: string
  ok: boolean
  hint?: string
}

function buildReadiness(h: WorkspaceHealth, channels: Channel[]) {
  const devChannel = channels.find((c) => c.kind === 'ai_development')
  const checks: Check[] = [
    { label: '工作区目录', ok: !!h.workspace_exists, hint: h.workspace_exists ? '已创建' : '节点上还未创建工作区目录' },
    { label: 'Git 初始化', ok: !!h.git_initialized, hint: h.git_initialized ? '已初始化' : '工作区尚未 git init' },
    { label: '节点在线', ok: !!h.node_online, hint: h.node_online ? `${h.node_id ?? ''}` : '执行节点离线' },
    { label: 'AI Agent 就绪', ok: !!h.cli_ready, hint: h.cli_ready ? '可接受 AI 开发任务' : '安装并配置 AI CLI 后就绪' },
    { label: '开发环境', ok: !!h.dev_env_ready, hint: h.dev_env_ready ? '工具链就绪' : '需要安装开发环境（JDK/Node/Git）' },
  ]

  const doneCount = checks.filter((c) => c.ok).length
  const total = checks.length
  const allOk = doneCount === total
  const tone = allOk ? 'ok' : doneCount >= 3 ? 'warn' : 'bad'

  let nextStep = ''
  if (!h.node_online) nextStep = '启动或注册一个在线执行节点'
  else if (!h.workspace_exists) nextStep = '创建项目工作区'
  else if (!h.git_initialized) nextStep = '初始化 Git 仓库'
  else if (!h.cli_ready) nextStep = '在节点上安装并配置 AI CLI'
  else if (!h.dev_env_ready) nextStep = '在节点上安装开发环境'
  else if (!devChannel) nextStep = '创建一个 AI 开发频道'
  else nextStep = '直接在 AI 开发频道描述需求，开始开发！'

  return { checks, tone, doneCount, total, nextStep, devChannel, allOk }
}

export default function ProjectReadinessCard({ health, loading, channels, onOpenChannel, onRefresh }: Props) {
  if (loading) return <div className={styles.loading}>检查开发就绪状态…</div>
  if (!health) return null

  const { checks, tone, doneCount, total, nextStep, devChannel, allOk } = buildReadiness(health, channels)

  return (
    <div className={[styles.card, styles[tone]].join(' ')}>
      <div className={styles.header}>
        <div>
          <div className={styles.eyebrow}>开发者就绪状态</div>
          <strong className={styles.title}>
            {allOk ? '✅ 已就绪' : `⚙ 配置中 (${doneCount}/${total})`}
          </strong>
        </div>
        <button className={styles.refreshBtn} onClick={onRefresh} type="button" title="刷新">↺</button>
      </div>

      {/* 进度条 */}
      <div className={styles.progressBar}>
        <div className={styles.progressFill} style={{ width: `${(doneCount / total) * 100}%` }} />
      </div>

      {/* 检查清单 */}
      <div className={styles.checklist}>
        {checks.map((c) => (
          <div key={c.label} className={[styles.checkItem, c.ok ? styles.checkOk : styles.checkFail].join(' ')}>
            <span className={styles.checkIcon}>{c.ok ? '✓' : '○'}</span>
            <span className={styles.checkLabel}>{c.label}</span>
            {c.hint && <span className={styles.checkHint}>{c.hint}</span>}
          </div>
        ))}
      </div>

      {/* 下一步 */}
      <div className={styles.nextStep}>
        <span className={styles.nextLabel}>下一步</span>
        <strong className={styles.nextText}>{nextStep}</strong>
      </div>

      {/* 快捷动作 */}
      {devChannel && (
        <div className={styles.actions}>
          <button
            className={styles.actionPrimary}
            onClick={() => onOpenChannel(devChannel.id)}
            type="button"
          >
            🛠 打开 AI 开发频道
          </button>
        </div>
      )}

      {/* 问题列表 */}
      {(health.issues ?? []).length > 0 && (
        <div className={styles.issues}>
          {(health.issues ?? []).map((iss, i) => <div key={i}>⚠ {iss}</div>)}
        </div>
      )}
    </div>
  )
}
