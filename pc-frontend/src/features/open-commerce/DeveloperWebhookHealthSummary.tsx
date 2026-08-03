import { useEffect, useState } from 'react'
import { Activity, CircleAlert, CircleCheck, PauseCircle } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  DeveloperWebhookEnvironmentHealth,
  DeveloperWebhookHealthSummary as HealthSummary,
} from './openCommerceClientTypes'
import { badgeStyle, commerceStyles } from './openCommerceStyles'

export default function DeveloperWebhookHealthSummary({
  projectId,
  appRecordId,
  refreshSignal,
}: {
  projectId: string
  appRecordId: string
  refreshSignal: string
}) {
  const [health, setHealth] = useState<HealthSummary | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    let active = true
    if (!appRecordId) {
      setHealth(null)
      return () => { active = false }
    }
    openCommerceClientApi.getDeveloperWebhookHealth(projectId, appRecordId)
      .then((response) => {
        if (active) {
          setHealth(response)
          setError('')
        }
      })
      .catch(() => {
        if (active) setError('运行健康暂时不可用')
      })
    return () => { active = false }
  }, [appRecordId, projectId, refreshSignal])

  if (!health) {
    return error ? <small style={commerceStyles.itemMeta}>{error}</small> : null
  }
  const productionConfigured = health.environments.some(
    (item) => item.environment === 'production' && item.subscription_count > 0,
  )
  const productionStatusLabel = !productionConfigured
    ? '未配置生产'
    : health.production_ready
      ? '生产已就绪'
      : blockerLabel(health.production_blocker_code)
  return (
    <section style={summaryStyle} aria-label="Webhook 运行健康">
      <header style={commerceStyles.itemHeader}>
        <strong style={commerceStyles.itemTitle}>投递健康</strong>
        <span style={badgeStyle(!productionConfigured || health.production_ready ? 'neutral' : 'warn')}>
          {productionStatusLabel}
        </span>
      </header>
      <div style={environmentGridStyle}>
        {health.environments.map((item) => (
          <div style={environmentRowStyle} key={item.environment}>
            <span style={statusIconStyle}>{statusIcon(item.status)}</span>
            <div>
              <strong style={commerceStyles.itemTitle}>
                {item.environment === 'production' ? '生产环境' : '测试环境'}
              </strong>
              <p style={commerceStyles.itemText}>
                订阅 {item.active_subscription_count}/{item.subscription_count} · 待发{' '}
                {item.pending_delivery_count + item.delivering_delivery_count} · 重试{' '}
                {item.retry_delivery_count} · 死信 {item.dead_delivery_count}
              </p>
              {(item.latest_error_code || item.oldest_queued_at) && (
                <small style={commerceStyles.itemMeta}>
                  {item.latest_error_code ?? `最早排队 ${formatTime(item.oldest_queued_at)}`}
                </small>
              )}
            </div>
          </div>
        ))}
      </div>
    </section>
  )
}

function statusIcon(status: DeveloperWebhookEnvironmentHealth['status']) {
  if (status === 'action_required' || status === 'attention') return <CircleAlert size={15} />
  if (status === 'healthy') return <CircleCheck size={15} />
  if (status === 'processing') return <Activity size={15} />
  return <PauseCircle size={15} />
}

function blockerLabel(code?: string) {
  if (code === 'production_webhooks_disabled') return '生产通知未开启'
  if (code === 'production_credentials_disabled') return '生产凭据未开启'
  if (code === 'production_credential_unavailable') return '生产资格未就绪'
  return '生产未就绪'
}

function formatTime(value?: string) {
  return value ? new Date(value).toLocaleString() : '未知'
}

const summaryStyle: React.CSSProperties = {
  display: 'grid',
  gap: 8,
  paddingBottom: 10,
  borderBottom: '1px solid var(--line)',
}

const environmentGridStyle: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fit, minmax(min(240px, 100%), 1fr))',
  gap: 8,
}

const environmentRowStyle: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '24px minmax(0, 1fr)',
  alignItems: 'start',
  minHeight: 58,
  padding: 8,
  border: '1px solid var(--line)',
  borderRadius: 6,
}

const statusIconStyle: React.CSSProperties = {
  display: 'grid',
  placeItems: 'center',
  width: 22,
  height: 22,
  color: '#a9ded2',
}
