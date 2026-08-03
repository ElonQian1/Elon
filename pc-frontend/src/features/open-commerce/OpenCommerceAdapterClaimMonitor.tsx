import { useCallback, useEffect, useState } from 'react'
import { CheckCircle2, Clock3, RefreshCw, RotateCcw, TimerOff, TriangleAlert } from 'lucide-react'
import { openCommerceApi } from './openCommerceApi'
import type {
  OpenCommerceAdapterHandoffClaim,
  OpenCommerceIntegration,
} from './openCommerceTypes'
import { actionStyle, badgeStyle } from './openCommerceStyles'
import { errorText } from './openCommerceUi'
import styles from './OpenCommerceAdapterClaimMonitor.module.css'

type Props = {
  projectId: string
  integrations: OpenCommerceIntegration[]
}

export default function OpenCommerceAdapterClaimMonitor({ projectId, integrations }: Props) {
  const [claims, setClaims] = useState<OpenCommerceAdapterHandoffClaim[]>([])
  const [busy, setBusy] = useState(false)
  const [resuming, setResuming] = useState('')
  const [error, setError] = useState('')

  const refresh = useCallback(async () => {
    if (!projectId) return
    setBusy(true)
    setError('')
    try {
      const result = await openCommerceApi.listAdapterHandoffClaims(projectId, 30)
      setClaims(result.claims)
    } catch (requestError) {
      setError(errorText(requestError))
    } finally {
      setBusy(false)
    }
  }, [projectId])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const resume = async (claim: OpenCommerceAdapterHandoffClaim) => {
    const confirmed = globalThis.confirm(
      `确认将调用 ${shortId(claim.invocation_id)} 重新排队？新的接入器会再次处理该业务结果。`,
    )
    if (!confirmed) return
    setResuming(claim.id)
    setError('')
    try {
      await openCommerceApi.resumeAdapterHandoffClaim(projectId, claim.id)
      await refresh()
    } catch (requestError) {
      setError(errorText(requestError))
    } finally {
      setResuming('')
    }
  }

  return (
    <section className={styles.monitor}>
      <header>
        <span>
          <strong><Clock3 size={14} />接入器任务租约</strong>
          <small>最近 30 次领取；同一业务调用同时只允许一个活动租约。</small>
        </span>
        <button
          style={actionStyle('icon', busy)}
          type="button"
          onClick={refresh}
          disabled={busy}
          title="刷新任务租约"
        >
          <RefreshCw size={14} />
        </button>
      </header>
      <div className={styles.claims}>
        {claims.map((claim) => {
          const integration = integrations.find((item) => item.id === claim.integration_id)
          const state = claimState(claim)
          const Icon = state === 'completed'
            ? CheckCircle2
            : state === 'attention_required'
              ? TriangleAlert
            : state === 'released' || state === 'retry_wait' || state === 'retry_ready'
              ? RotateCcw
              : state === 'expired'
                ? TimerOff
                : Clock3
          return (
            <article key={claim.id}>
              <Icon size={14} />
              <span>
                <strong>{integration?.display_name ?? claim.integration_id}</strong>
                <small>
                  调用 {shortId(claim.invocation_id)} · 第 {claim.attempt_no} 次
                  {state === 'released' && claim.release_reason_code
                    ? ` · ${releaseReasonLabel(claim.release_reason_code)}`
                    : ''}
                  {state === 'retry_wait' && claim.retry_not_before
                    ? ` · ${formatTime(claim.retry_not_before)} 后重试`
                    : ''}
                </small>
              </span>
              <span style={badgeStyle(state === 'active' || state === 'retry_wait' ? 'warn' : state === 'completed' || state === 'released' || state === 'retry_ready' ? 'neutral' : 'danger')}>
                {state === 'active'
                  ? '处理中'
                  : state === 'completed'
                    ? '已回执'
                    : state === 'released'
                      ? '已释放'
                      : state === 'retry_wait'
                        ? '退避中'
                        : state === 'retry_ready'
                          ? '可重试'
                          : state === 'attention_required'
                            ? '需人工处理'
                          : '已超时'}
              </span>
              {state === 'attention_required' && (
                <button
                  style={actionStyle('secondary', resuming === claim.id)}
                  type="button"
                  onClick={() => void resume(claim)}
                  disabled={Boolean(resuming)}
                >
                  <RotateCcw size={13} />重新排队
                </button>
              )}
              <time>
                {state === 'active'
                  ? `本次到期 ${formatTime(claim.lease_expires_at)} · 最长 ${formatTime(claim.lease_deadline_at)}`
                  : formatTime(claim.updated_at)}
              </time>
            </article>
          )
        })}
        {claims.length === 0 && !busy && <p>暂无接入器任务租约。</p>}
      </div>
      {error && <p className={styles.error}>{error}</p>}
    </section>
  )
}

function claimState(claim: OpenCommerceAdapterHandoffClaim) {
  if (claim.retry_suspended_at && !claim.retry_resumed_at) {
    return 'attention_required' as const
  }
  if (claim.completion_status === 'rejected') {
    return claim.retry_not_before && new Date(claim.retry_not_before).getTime() > Date.now()
      ? 'retry_wait' as const
      : 'retry_ready' as const
  }
  if (claim.status === 'active' && new Date(claim.lease_expires_at).getTime() <= Date.now()) {
    return 'expired' as const
  }
  return claim.status
}

function shortId(value: string) {
  return value.length > 16 ? `${value.slice(0, 7)}…${value.slice(-5)}` : value
}

function formatTime(value: string) {
  return new Date(value).toLocaleString('zh-CN', { hour12: false })
}

function releaseReasonLabel(value: string) {
  const labels: Record<string, string> = {
    adapter_shutdown: '接入器停机',
    capacity_pressure: '容量不足',
    transient_failure: '临时故障',
    manual_release: '主动释放',
  }
  return labels[value] ?? value
}
