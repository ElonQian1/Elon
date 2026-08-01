import { useMemo, useState } from 'react'
import { openCommerceApi } from './openCommerceApi'
import type {
  OpenCommerceMerchantDetail,
  OpenCommerceRateLimitPolicy,
  OpenCommerceRateLimitUsage,
} from './openCommerceTypes'
import { commerceStyles } from './openCommerceStyles'
import styles from './OpenCommercePanel.module.css'

interface Props {
  projectId: string
  merchant: OpenCommerceMerchantDetail
  policies: OpenCommerceRateLimitPolicy[]
  usage: OpenCommerceRateLimitUsage[]
  canEdit: boolean
  onChanged: () => Promise<void>
}

export default function OpenCommerceRateLimitManager({
  projectId,
  merchant,
  policies,
  usage,
  canEdit,
  onChanged,
}: Props) {
  const capabilities = useMemo(
    () => merchant.capabilities.filter((capability) => capability.status === 'active'),
    [merchant.capabilities],
  )
  const merchantPolicies = useMemo(
    () => policies.filter((policy) => policy.merchant_id === merchant.merchant.id),
    [merchant.merchant.id, policies],
  )
  const [capabilityKey, setCapabilityKey] = useState('')
  const [requesterAppId, setRequesterAppId] = useState('')
  const [windowSeconds, setWindowSeconds] = useState('60')
  const [maxRequests, setMaxRequests] = useState('60')
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')

  async function savePolicy(event: React.FormEvent) {
    event.preventDefault()
    setBusy('save')
    setMessage('')
    try {
      if (!capabilityKey) throw new Error('请选择商业能力')
      const windowValue = Number.parseInt(windowSeconds, 10)
      const maxValue = Number.parseInt(maxRequests, 10)
      if (!Number.isFinite(windowValue) || windowValue < 1 || windowValue > 86_400) {
        throw new Error('时间窗必须在 1 到 86400 秒之间')
      }
      if (!Number.isFinite(maxValue) || maxValue < 1 || maxValue > 1_000_000) {
        throw new Error('调用上限必须在 1 到 1000000 次之间')
      }
      await openCommerceApi.upsertRateLimit(projectId, {
        merchant_id: merchant.merchant.id,
        capability_key: capabilityKey,
        requester_app_id: requesterAppId.trim() || undefined,
        window_seconds: windowValue,
        max_requests: maxValue,
        enabled: true,
      })
      setMessage('调用配额已保存。相同能力和 App 的策略会原位更新。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  async function togglePolicy(policy: OpenCommerceRateLimitPolicy) {
    setBusy(policy.id)
    setMessage('')
    try {
      await openCommerceApi.setRateLimitEnabled(
        projectId,
        policy.id,
        policy.status !== 'active',
      )
      setMessage(policy.status === 'active' ? '调用配额已停用。' : '调用配额已启用。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  return (
    <section className={styles.capabilityList}>
      <header>
        <strong>外部调用配额</strong>
        <span>{merchantPolicies.filter((policy) => policy.status === 'active').length} 条生效</span>
      </header>
      <p>按能力和 App 限制固定时间窗内的新调用。幂等重放不重复计数，项目编辑者调试不占额度。</p>

      {merchantPolicies.map((policy) => {
        const current = usage.find((entry) => entry.policy_id === policy.id)
        return (
          <div key={policy.id} className={styles.capabilityRow}>
            <span>
              <strong>{policy.capability_key}</strong>
              <small>
                {policy.requester_app_id ?? '全部 App'} · 每 {policy.window_seconds} 秒最多 {policy.max_requests} 次
              </small>
            </span>
            <span>
              <small>本窗已接收 {current?.accepted_requests ?? 0} 次 · {current?.active_subjects ?? 0} 个主体</small>
              <button
                type="button"
                onClick={() => togglePolicy(policy)}
                disabled={!canEdit || busy === policy.id}
              >
                {policy.status === 'active' ? '停用' : '启用'}
              </button>
            </span>
          </div>
        )
      })}
      {merchantPolicies.length === 0 && <p className={styles.empty}>尚未设置配额；默认沿用现有调用行为。</p>}

      <form className={styles.formCard} onSubmit={savePolicy}>
        <header><strong>设置调用配额</strong><small>留空 App 表示每个调用方分别限流</small></header>
        <label>商业能力<select value={capabilityKey} onChange={(event) => setCapabilityKey(event.target.value)} disabled={!canEdit} required>
          <option value="">请选择</option>
          {capabilities.map((capability) => <option key={capability.id} value={capability.capability_key}>{capability.display_name} · {capability.capability_key}</option>)}
        </select></label>
        <label>指定 App（可选）<input value={requesterAppId} onChange={(event) => setRequesterAppId(event.target.value)} placeholder="consumer.my-app" disabled={!canEdit} /></label>
        <div className={styles.twoColumns}>
          <label>时间窗（秒）<input type="number" min="1" max="86400" value={windowSeconds} onChange={(event) => setWindowSeconds(event.target.value)} disabled={!canEdit} /></label>
          <label>最多调用（次）<input type="number" min="1" max="1000000" value={maxRequests} onChange={(event) => setMaxRequests(event.target.value)} disabled={!canEdit} /></label>
        </div>
        <button type="submit" disabled={!canEdit || busy === 'save'}>{busy === 'save' ? '保存中…' : '保存配额'}</button>
      </form>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return '操作失败，请稍后重试'
}
