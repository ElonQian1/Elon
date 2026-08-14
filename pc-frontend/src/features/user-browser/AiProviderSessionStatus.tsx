import { AlertTriangle, CircleCheck, LoaderCircle, MonitorUp } from 'lucide-react'
import type { LocalAiUserState } from './localAiUserState'
import styles from './AiProviderSessionStatus.module.css'

export default function AiProviderSessionStatus({
  state,
  compact = false,
}: {
  state: LocalAiUserState
  compact?: boolean
}) {
  const Icon = state.tone === 'ready'
    ? CircleCheck
    : state.tone === 'loading'
      ? LoaderCircle
      : state.tone === 'muted'
        ? MonitorUp
        : AlertTriangle

  return (
    <section className={styles.status} data-tone={state.tone} data-compact={compact} aria-label="官方 AI 会话与能力状态">
      <Icon className={state.tone === 'loading' ? styles.spin : ''} size={compact ? 15 : 19} />
      <div className={styles.copy}>
        <header><strong>{state.title}</strong><em>{state.badge}</em></header>
        <p>{state.detail}</p>
        {state.features.length > 0 && (
          <ul aria-label="当前原生能力">
            {state.features.map((feature) => (
              <li key={feature.id} data-active={feature.active}>
                <span aria-hidden="true" />{feature.label}
              </li>
            ))}
          </ul>
        )}
        {state.fallbackRecommended && <small>可显示官方窗口继续使用；原生 UI 会在能力恢复后自动连接。</small>}
      </div>
    </section>
  )
}
