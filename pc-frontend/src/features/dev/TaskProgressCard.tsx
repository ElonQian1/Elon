import { ChevronDown, ChevronRight, StopCircle } from 'lucide-react'
import type { TaskTimelineModel } from './taskTimelineModel'
import type { TaskTone } from './types'
import styles from './TaskProgressCard.module.css'

interface TaskProgressCardProps {
  status: {
    tone: TaskTone
    label: string
  }
  timeline: TaskTimelineModel
  progressCount: number
  processSummary: string
  collapsed: boolean
  canCancel: boolean
  onToggle: () => void
  onCancel: () => void
}

export default function TaskProgressCard({
  status,
  timeline,
  progressCount,
  processSummary,
  collapsed,
  canCancel,
  onToggle,
  onCancel,
}: TaskProgressCardProps) {
  const stage = readableStage(timeline.stage.label)
  const detail = readableText(timeline.stage.detail)
  const meta = readableMeta(timeline.stage.meta)
  const summary = compactProcessSummary(processSummary, progressCount)
  const hasDetails = progressCount > 0

  return (
    <section className={styles.card} data-tone={status.tone} aria-live="polite">
      <div className={styles.header}>
        <span className={styles.statusDot} aria-hidden="true" />
        <div className={styles.headerText}>
          <span>{status.label}</span>
          <strong>{headlineForTone(status.tone)}</strong>
        </div>
        {canCancel && (
          <button type="button" className={styles.cancelButton} onClick={onCancel}>
            <StopCircle size={14} />
            <span>停止</span>
          </button>
        )}
      </div>

      <div className={styles.current}>
        <div className={styles.currentHead}>
          <strong>{stage}</strong>
          {meta && <em>{meta}</em>}
        </div>
        {detail && <p>{detail}</p>}
      </div>

      {hasDetails && (
        <button
          type="button"
          className={styles.detailButton}
          onClick={onToggle}
          aria-expanded={!collapsed}
        >
          <span>{collapsed ? `查看过程 · ${summary}` : `收起过程 · ${summary}`}</span>
          {collapsed ? <ChevronRight size={15} /> : <ChevronDown size={15} />}
        </button>
      )}
    </section>
  )
}

function headlineForTone(tone: TaskTone): string {
  if (tone === 'done') return '任务已完成'
  if (tone === 'failed') return '任务遇到问题'
  if (tone === 'canceled') return '任务已停止'
  if (tone === 'approval') return '等待你的确认'
  return 'AI 正在处理'
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
  return meta
}

function compactProcessSummary(summary: string, progressCount: number): string {
  const fallback = `${progressCount} 项`
  const parts = summary
    .split(' · ')
    .map((part) => readableText(part))
    .filter((part) => part && !looksTechnical(part) && !part.startsWith('当前：'))
  return parts.slice(0, 2).join(' · ') || fallback
}

function looksTechnical(value: string): boolean {
  return /^[a-f0-9]{6,}$/i.test(value)
    || /^(usr|node|pc|agent|task|tsk)_/i.test(value)
    || /\b[0-9a-f]{8}-[0-9a-f-]{13,}\b/i.test(value)
}
