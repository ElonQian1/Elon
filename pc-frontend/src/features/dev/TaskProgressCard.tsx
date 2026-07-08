import { ChevronDown, ChevronRight, Monitor, PlayCircle, Settings, StopCircle } from 'lucide-react'
import type { TaskTimelineModel } from './taskTimelineModel'
import { launchWinClientProtocol } from '../node/launchWinClient'
import type { TaskTone } from './types'
import styles from './TaskProgressCard.module.css'

interface TaskProgressCardProps {
  status: {
    tone: TaskTone
    label: string
  }
  displayStatus?: {
    tone: TaskTone
    label: string
  }
  timeline: TaskTimelineModel
  progressCount: number
  processSummary: string
  collapsed: boolean
  canCancel: boolean
  canContinue?: boolean
  compact?: boolean
  lockedOpen?: boolean
  processedDuration?: string
  suppressNarrative?: boolean
  onToggle: () => void
  onCancel: () => void
  onContinue?: () => void
}

export default function TaskProgressCard({
  status,
  displayStatus = status,
  timeline,
  progressCount,
  processSummary,
  collapsed,
  canCancel,
  canContinue = false,
  compact = false,
  lockedOpen = false,
  processedDuration = '',
  suppressNarrative = false,
  onToggle,
  onCancel,
  onContinue,
}: TaskProgressCardProps) {
  const stage = readableStage(timeline.stage.label)
  const detail = readableText(timeline.stage.detail)
  const meta = readableMeta(timeline.stage.meta)
  const headline = headlineForTone(displayStatus.tone)
  const stageRepeatsHeadline = stage === headline || stage === displayStatus.label
  const showStatusLabel = displayStatus.tone !== 'done' && displayStatus.tone !== 'canceled' && displayStatus.tone !== 'approval'
  const summary = compactProcessSummary(processSummary, progressCount)
  const terseSummary = terseProcessSummary(summary)
  const hasDetails = progressCount > 0
  const shouldExplainCurrentStage = shouldShowCurrentStageCopy(timeline.stage.key, timeline.stage.stuck, hasDetails)
  const showCurrent = timeline.stage.key === 'approval'
    ? false
    : shouldExplainCurrentStage && Boolean(detail || meta || !stageRepeatsHeadline)
  const streamMode = !compact && (displayStatus.tone === 'running' || displayStatus.tone === 'queued') && !timeline.stage.stuck
  const streamCopy = shouldExplainCurrentStage
    ? progressNarrative(displayStatus.tone, timeline.stage.key, stage, detail)
    : { kicker: '正在处理', title: '我正在继续处理这轮任务。', detail: '' }
  const stageActions = actionStateForStage(timeline.stage.key, status.tone, canContinue)
  const compactTitle = compactTitleForTone(displayStatus.tone, processedDuration)
  const detailButton = hasDetails ? (
    <button
      type="button"
      className={[
        styles.detailButton,
        streamMode ? styles.streamDetailButton : '',
        suppressNarrative ? styles.summaryOnlyDetailButton : '',
        lockedOpen ? styles.detailButtonLocked : '',
      ].filter(Boolean).join(' ')}
      onClick={onToggle}
      aria-expanded={!collapsed}
      disabled={lockedOpen}
    >
      <span>{lockedOpen ? `审批已展开 · ${terseSummary}` : collapsed ? `过程 · ${terseSummary}` : `收起过程 · ${terseSummary}`}</span>
      {!lockedOpen && (collapsed ? <ChevronRight size={15} /> : <ChevronDown size={15} />)}
    </button>
  ) : null

  if (compact) {
    return (
      <section className={[styles.card, styles.compactCard].join(' ')} data-tone={displayStatus.tone} aria-live="polite">
        <div className={styles.compactRow}>
          <button
            type="button"
            className={styles.compactButton}
            onClick={onToggle}
            aria-expanded={!collapsed}
          >
            <span className={styles.statusDot} aria-hidden="true" />
            <span className={styles.compactTitle}>{compactTitle}</span>
            <span className={styles.compactSummary}>{terseSummary}</span>
            {collapsed ? <ChevronRight size={15} /> : <ChevronDown size={15} />}
          </button>
          <StageActions state={stageActions} onContinue={onContinue} />
        </div>
      </section>
    )
  }

  if (suppressNarrative) {
    if (!detailButton && !canCancel && !stageActions.show) return null
    return (
      <section className={[styles.card, styles.summaryOnlyCard].join(' ')} data-tone={displayStatus.tone} aria-live="polite">
        {(detailButton || canCancel) && (
          <div className={styles.summaryOnlyRow}>
            {detailButton}
            {canCancel && (
              <button type="button" className={styles.cancelButton} onClick={onCancel} aria-label="停止任务" title="停止任务">
                <StopCircle size={14} />
                <span>停止</span>
              </button>
            )}
          </div>
        )}
        <StageActions state={stageActions} onContinue={onContinue} />
      </section>
    )
  }

  return (
    <section className={styles.card} data-tone={displayStatus.tone} data-mode={streamMode ? 'stream' : undefined} aria-live="polite">
      {streamMode ? (
        <div className={styles.streamLine}>
          <span className={styles.statusDot} aria-hidden="true" />
          <div className={styles.streamText}>
            <span className={styles.streamKicker}>{streamCopy.kicker}</span>
            <strong>{streamCopy.title}</strong>
            {streamCopy.detail && <p>{streamCopy.detail}</p>}
          </div>
          {canCancel && (
            <button type="button" className={styles.cancelButton} onClick={onCancel} aria-label="停止任务" title="停止任务">
              <StopCircle size={14} />
              <span>停止</span>
            </button>
          )}
        </div>
      ) : (
        <>
          <div className={styles.header}>
            <span className={styles.statusDot} aria-hidden="true" />
            <div className={styles.headerText}>
              {showStatusLabel && <span>{displayStatus.label}</span>}
              <strong>{headline}</strong>
            </div>
            {canCancel && (
              <button type="button" className={styles.cancelButton} onClick={onCancel} aria-label="停止任务" title="停止任务">
                <StopCircle size={14} />
                <span>停止</span>
              </button>
            )}
          </div>

          {showCurrent && (
            <div className={styles.current}>
              {(!stageRepeatsHeadline || meta) && (
                <div className={styles.currentHead}>
                  {!stageRepeatsHeadline && <strong>{stage}</strong>}
                  {meta && <em>{meta}</em>}
                </div>
              )}
              {detail && <p>{detail}</p>}
            </div>
          )}
        </>
      )}

      <StageActions state={stageActions} onContinue={onContinue} />

      {detailButton}
    </section>
  )
}

interface StageActionState {
  show: boolean
  canContinue: boolean
  canOpenNode: boolean
  continueLabel: string
}

function StageActions({ state, onContinue }: { state: StageActionState; onContinue?: () => void }) {
  if (!state.show) return null
  return (
    <div className={styles.stageActions}>
      {state.canContinue && (
        <button type="button" className={styles.stagePrimaryAction} onClick={onContinue}>
          <PlayCircle size={13} aria-hidden="true" />
          <span>{state.continueLabel}</span>
        </button>
      )}
      {state.canOpenNode && (
        <>
          <button type="button" onClick={launchWinClientProtocol}>
            <Monitor size={13} aria-hidden="true" />
            <span>启动 Win 端</span>
          </button>
          <a href="/pc/node">
            <Settings size={13} aria-hidden="true" />
            <span>节点设置</span>
          </a>
        </>
      )}
    </div>
  )
}

function progressNarrative(
  tone: TaskTone,
  stageKey: string,
  stage: string,
  detail: string,
): { kicker: string; title: string; detail: string } {
  if (tone === 'queued') {
    return { kicker: '准备中', title: '我正在准备处理这轮请求。', detail: '连接到可用节点后会继续。' }
  }
  if (['recovery', 'recovering'].includes(stageKey)) {
    return { kicker: '恢复连接', title: '我正在恢复本轮任务连接。', detail: '先确认本地会话状态，再接上后续步骤。' }
  }
  if (stageKey === 'heartbeat') {
    return { kicker: '等待输出', title: '我已经接到任务，正在等待本机 AI 输出。', detail: '收到公开步骤后会继续更新这里。' }
  }
  if (stageKey === 'dispatch') {
    return { kicker: '连接节点', title: '我正在连接本机节点。', detail: '确认执行环境后会继续处理。' }
  }
  if (stageKey === 'server-update') {
    return { kicker: '临时中断', title: '服务器正在更新升级。', detail: '更新完成后会自动恢复通信。' }
  }
  if (stageKey === 'win-update') {
    return { kicker: '临时中断', title: 'Win 端正在更新升级。', detail: '客户端重启并重连后会继续。' }
  }
  if (stageKey === 'command') {
    return { kicker: '执行命令', title: '我正在等待命令执行结果。', detail: '命令返回后会继续推进下一步。' }
  }
  if (stageKey === 'assistant') {
    return { kicker: '整理回复', title: '我已经收到部分回复，正在等待收尾。', detail: '' }
  }
  return {
    kicker: '正在处理',
    title: friendlyStageTitle(stage),
    detail: friendlyStageDetail(detail),
  }
}

function friendlyStageTitle(stage: string): string {
  if (!stage || looksInternalStatus(stage)) return '我正在继续处理这轮任务。'
  if (/通信.*恢复|恢复.*通信|恢复.*连接/.test(stage)) return '我正在恢复本轮任务连接。'
  if (/等待.*(?:CLI|输出)|公开输出/.test(stage)) return '我正在等待本机 AI 输出。'
  if (/等待.*节点|连接.*节点|PC 节点/.test(stage)) return '我正在连接本机节点。'
  if (/等待.*命令|命令.*结果/.test(stage)) return '我正在等待命令执行结果。'
  if (/^(恢复|等待|执行|处理|连接)/.test(stage)) return `我正在${stage.replace(/^正在/, '')}。`
  return stage
}

function friendlyStageDetail(detail: string): string {
  if (!detail) return ''
  if (looksInternalStatus(detail)) return '正在确认本地会话状态，接上后续步骤。'
  return detail.length > 86 ? `${detail.slice(0, 86)}...` : detail
}

function looksInternalStatus(value: string): boolean {
  return /(journal|wait_or_cancel|恢复合同|运行控制句柄|任务快照|pc_req|sidecar|lease|tsk_)/i.test(value)
}

function shouldShowCurrentStageCopy(stageKey: string, stuck: boolean, hasDetails: boolean): boolean {
  if (!hasDetails) return true
  if (stuck) return true
  return [
    'command',
    'dispatch',
    'empty',
    'heartbeat',
    'recovery',
    'recovery-timeout',
    'recovering',
    'resume-required',
    'server-update',
    'timeout',
    'tool-timeout',
    'win-update',
  ].includes(stageKey)
}

function headlineForTone(tone: TaskTone): string {
  if (tone === 'done') return '任务已完成'
  if (tone === 'failed') return '任务遇到问题'
  if (tone === 'canceled') return '任务已停止'
  if (tone === 'approval') return '等待你的确认'
  return 'AI 正在处理'
}

function compactTitleForTone(tone: TaskTone, duration: string): string {
  if (tone === 'done') return duration ? `已处理 ${duration}` : '已处理'
  if (tone === 'failed') return duration ? `处理失败 ${duration}` : '处理失败'
  if (tone === 'canceled') return duration ? `已停止 ${duration}` : '已停止'
  return '过程'
}

function readableStage(value: string): string {
  const cleaned = readableText(value)
    .replace(/^最后公开步骤[：:]\s*/, '')
    .replace(/^当前[：:]\s*/, '')
    .trim()
  return cleaned || '等待公开过程'
}

function readableText(value: string | undefined): string {
  return String(value ?? '')
    .replace(/\btsk_[a-z0-9_-]+\b/gi, '本轮任务')
    .replace(/[（(][a-f0-9]{8,}[)）]/gi, '')
    .replace(/\s{2,}/g, ' ')
    .trim()
}

function readableMeta(value: string | undefined): string {
  const meta = readableText(value)
  if (!meta) return ''
  if (/^[a-f0-9]{8,}$/i.test(meta)) return ''
  if (/^(usr|node|pc|agent|task|tsk)_/i.test(meta)) return ''
  if (/^(codex|claude|copilot|gemini|gpt|ai)$/i.test(meta)) return ''
  return meta
}

function compactProcessSummary(summary: string, progressCount: number): string {
  const fallback = `${progressCount} 项`
  const parts = summary
    .split(' · ')
    .map((part) => readableText(part))
    .filter((part) => part && !looksTechnical(part) && !looksInternalSummary(part) && !part.startsWith('当前：'))
  const step = parts.find((part) => /[0-9]+\s*(步|项)/.test(part))
  const priority = [
    parts.find((part) => part === '等待审批'),
    parts.find((part) => part === '有文件修改'),
    parts.find((part) => part === '有测试/构建'),
    parts.find((part) => part === '有命令'),
    parts.find((part) => /公开回复/.test(part)),
  ].filter(Boolean)
  const compact = [step, ...priority].filter(Boolean).slice(0, 4)
  return compact.join(' · ') || parts.slice(0, 3).join(' · ') || fallback
}

function actionStateForStage(stageKey: string, tone: TaskTone, canContinue: boolean): StageActionState {
  const continueStages = new Set(['heartbeat', 'resume-required', 'recovery-timeout', 'timeout', 'tool-timeout'])
  const nodeStages = new Set(['heartbeat', 'recovery-timeout', 'timeout', 'tool-timeout'])
  const showContinue = canContinue && (tone === 'failed' || continueStages.has(stageKey))
  const showNode = nodeStages.has(stageKey)
  return {
    show: showContinue || showNode,
    canContinue: showContinue,
    canOpenNode: showNode,
    continueLabel: tone === 'failed' && stageKey === 'finished' ? '重试处理' : '继续处理',
  }
}

function terseProcessSummary(summary: string): string {
  return summary
    .replace(/([0-9]+)\s*步过程/g, '$1 步')
    .replace(/合并\s*([0-9]+)\s*条等待状态/g, '合并 $1 条等待')
}

function looksTechnical(value: string): boolean {
  return /^[a-f0-9]{6,}$/i.test(value)
    || /^(usr|node|pc|agent|task|tsk)_/i.test(value)
    || /\b[0-9a-f]{8}-[0-9a-f-]{13,}\b/i.test(value)
}

function looksInternalSummary(value: string): boolean {
  return /本轮任务|未收到 CLI 输出|卡点|等待节点确认|等待过程/i.test(value)
}
