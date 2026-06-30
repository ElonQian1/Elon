import { Check } from 'lucide-react'
import type { CSSProperties } from 'react'
import { createPortal } from 'react-dom'
import { effortDisplayName, normalizeEffort } from './modelGroups'
import { providerGroupTitle } from './modelUtils'
import type { ModelOptionGroup } from './modelGroups'
import type { AgentOption } from './types'
import styles from './ModelPicker.module.css'

interface Props {
  group: ModelOptionGroup | null
  anchorRect: DOMRect | null
  selectedAgent: string
  saving: boolean
  routeTitle?: string
  onSelect: (option: AgentOption) => void
  onMouseEnter: () => void
  onMouseLeave: () => void
}

const EFFORT_NOTES: Record<string, string> = {
  minimal: '最小推理，最快响应',
  low: '快速响应',
  medium: '平衡速度和推理',
  high: '更深推理',
  xhigh: '最高推理预算',
  max: '最高推理预算',
}

function modelIdentity(option: AgentOption): string {
  return option.modelId || option.agentName || 'default'
}

function contextLabel(option: AgentOption): string {
  const text = `${option.label} ${option.modelId} ${option.subtitle}`.toLowerCase()
  const explicit = text.match(/\b(\d+(?:\.\d+)?)\s*(m|k)\b/)
  if (explicit) return `${explicit[1]}${explicit[2].toUpperCase()}`
  return option.agentName ? '服务端配置' : '服务器默认'
}

function effortLabel(option: AgentOption): string {
  return option.reasoningEffort ? effortDisplayName(option.reasoningEffort) : 'Auto'
}

function effortNote(option: AgentOption): string {
  const effort = normalizeEffort(option.reasoningEffort)
  const details = [
    EFFORT_NOTES[effort] ?? '跟随服务端策略',
    option.verbosity ? `输出 ${option.verbosity}` : '',
    option.reasoningSummary ? `摘要 ${option.reasoningSummary}` : '',
  ].filter(Boolean)
  return details.join(' · ')
}

function floatingStyle(anchorRect: DOMRect): CSSProperties {
  const gap = 10
  const narrow = window.innerWidth <= 720
  const width = Math.min(narrow ? 360 : 330, window.innerWidth - 24)
  const maxHeight = Math.min(narrow ? 360 : 460, window.innerHeight - 24)
  const openLeft = anchorRect.left - width - gap
  const openRight = anchorRect.right + gap
  const left = narrow
    ? Math.max(12, Math.min(anchorRect.left, window.innerWidth - width - 12))
    : openLeft >= 12
      ? openLeft
      : Math.max(12, Math.min(openRight, window.innerWidth - width - 12))
  const preferredTop = narrow ? anchorRect.bottom + 8 : anchorRect.top - 8
  const top = Math.max(12, Math.min(preferredTop, window.innerHeight - maxHeight - 12))

  return {
    left: Math.round(left),
    top: Math.round(top),
    width: Math.round(width),
    maxHeight: Math.round(maxHeight),
  }
}

export function ModelHoverPreview({
  group,
  anchorRect,
  selectedAgent,
  saving,
  routeTitle,
  onSelect,
  onMouseEnter,
  onMouseLeave,
}: Props) {
  if (!group || !anchorRect) return null

  const option = group.selectedOption ?? group.primaryOption
  const provider = providerGroupTitle(option.provider)
  const meta = [
    routeTitle ? ['AI 来源', routeTitle] : null,
    ['运行方式', provider],
    ['模型 ID', modelIdentity(option)],
    ['最大上下文', contextLabel(option)],
    ['推理档位', group.options.length > 1 ? `${group.options.length} 个可选` : effortLabel(option)],
    option.verbosity ? ['输出细节', option.verbosity] : null,
    option.reasoningSummary ? ['推理摘要', option.reasoningSummary] : null,
    option.selectable === false
      ? ['选择状态', option.unavailableReason || '当前只作为探测结果展示']
      : null,
  ].filter(Boolean) as [string, string][]

  return createPortal(
    <aside
      className={styles.effortPopover}
      style={floatingStyle(anchorRect)}
      aria-label="模型档位详情"
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      <div className={styles.previewHeader}>
        <span className={styles.previewProvider}>{provider}</span>
        <strong>{group.label}</strong>
        {group.subtitle && <p>{group.subtitle}</p>}
      </div>

      <dl className={styles.previewMeta}>
        {meta.map(([key, value]) => (
          <div key={key}>
            <dt>{key}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>

      <section className={styles.previewEffort} aria-label="Thinking Effort">
        <h3>Thinking Effort</h3>
        <div className={styles.effortRows}>
          {group.options.map((item) => {
            const active = item.agentName === selectedAgent
            const disabled = saving || active || item.selectable === false
            return (
              <button
                className={active ? styles.effortRowActive : styles.effortRow}
                key={item.agentName || `${group.key}:default`}
                type="button"
                disabled={disabled}
                onClick={() => onSelect(item)}
                aria-pressed={active}
              >
                <span>
                  {active ? <Check size={14} strokeWidth={2.3} aria-hidden="true" /> : ''}
                </span>
                <strong>{effortDisplayName(item.reasoningEffort)}</strong>
                <em>{item.selectable === false ? item.unavailableReason || '探测结果' : effortNote(item)}</em>
              </button>
            )
          })}
        </div>
      </section>
    </aside>,
    document.body,
  )
}
