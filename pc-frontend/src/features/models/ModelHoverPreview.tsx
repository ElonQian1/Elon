import { Check } from 'lucide-react'
import { providerGroupTitle } from './modelUtils'
import type { AgentOption } from './types'
import styles from './ModelPicker.module.css'

interface Props {
  option: AgentOption | null
  selected: boolean
  saving: boolean
  routeTitle?: string
  onSelect: (option: AgentOption) => void
}

const EFFORT_ROWS = [
  { key: 'low', label: 'Low', note: '快速响应' },
  { key: 'medium', label: 'Medium', note: '平衡速度和推理' },
  { key: 'high', label: 'High', note: '更深推理' },
  { key: 'max', label: 'Max', note: '最高推理预算' },
]

function normalize(value: string): string {
  return value.trim().toLowerCase().replace(/\s+/g, '_')
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
  return option.reasoningEffort || (option.agentName ? '服务端默认' : '自动')
}

export function ModelHoverPreview({
  option,
  selected,
  saving,
  routeTitle,
  onSelect,
}: Props) {
  if (!option) {
    return (
      <aside className={styles.previewPane} aria-label="模型详情">
        <div className={styles.previewEmpty}>
          <strong>暂无模型</strong>
          <span>当前来源没有返回可选模型。</span>
        </div>
      </aside>
    )
  }

  const provider = providerGroupTitle(option.provider)
  const activeEffort = normalize(option.reasoningEffort)
  const meta = [
    routeTitle ? ['AI 来源', routeTitle] : null,
    ['运行方式', provider],
    ['模型 ID', modelIdentity(option)],
    ['最大上下文', contextLabel(option)],
    ['推理强度', effortLabel(option)],
    option.verbosity ? ['输出细节', option.verbosity] : null,
    option.reasoningSummary ? ['推理摘要', option.reasoningSummary] : null,
  ].filter(Boolean) as [string, string][]

  return (
    <aside className={styles.previewPane} aria-label="模型详情">
      <div className={styles.previewHeader}>
        <span className={styles.previewProvider}>{provider}</span>
        <strong>{option.label}</strong>
        {option.subtitle && <p>{option.subtitle}</p>}
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
          {EFFORT_ROWS.map((row) => {
            const active = activeEffort === row.key
            return (
              <div className={active ? styles.effortRowActive : styles.effortRow} key={row.key}>
                <span>{active ? '✓' : ''}</span>
                <strong>{row.label}</strong>
                <em>{row.note}</em>
              </div>
            )
          })}
          {!activeEffort && (
            <div className={styles.effortRowActive}>
              <span>✓</span>
              <strong>Auto</strong>
              <em>跟随服务端策略</em>
            </div>
          )}
        </div>
      </section>

      <button
        className={styles.previewSelectBtn}
        type="button"
        disabled={selected || saving}
        onClick={() => onSelect(option)}
      >
        <Check size={15} strokeWidth={2.3} aria-hidden="true" />
        {selected ? '当前模型' : '选择此模型'}
      </button>
    </aside>
  )
}
