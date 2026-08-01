import { useCallback, useEffect, useMemo, useState } from 'react'
import { ShieldAlert } from 'lucide-react'
import { openCommerceApi } from './openCommerceApi'
import type {
  OpenCommerceAppActivityHealth,
  OpenCommerceAppBlock,
  OpenCommerceAppBlockReason,
} from './openCommerceTypes'
import { commerceStyles } from './openCommerceStyles'
import styles from './OpenCommercePanel.module.css'

interface Props {
  projectId: string
  merchantId: string
  appActivityHealth: OpenCommerceAppActivityHealth[]
  suggestedAppIds: string[]
  canEdit: boolean
  onChanged: () => Promise<void>
}

const reasonOptions: Array<{ value: OpenCommerceAppBlockReason; label: string }> = [
  { value: 'abusive_traffic', label: '异常高频调用' },
  { value: 'policy_violation', label: '违反商户规则' },
  { value: 'security_incident', label: '安全事件' },
  { value: 'merchant_request', label: '商户主动终止' },
  { value: 'other', label: '其他' },
]

export default function OpenCommerceAppBlockManager({
  projectId,
  merchantId,
  appActivityHealth,
  suggestedAppIds,
  canEdit,
  onChanged,
}: Props) {
  const [blocks, setBlocks] = useState<OpenCommerceAppBlock[]>([])
  const [requesterAppId, setRequesterAppId] = useState('')
  const [reasonCode, setReasonCode] = useState<OpenCommerceAppBlockReason>('abusive_traffic')
  const [reasonNote, setReasonNote] = useState('')
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    const next = await openCommerceApi.listAppBlocks(projectId)
    setBlocks(next)
  }, [projectId])

  useEffect(() => {
    refresh().catch((error) => setMessage(errorMessage(error)))
  }, [refresh])

  const merchantBlocks = useMemo(
    () => blocks.filter((block) => block.merchant_id === merchantId),
    [blocks, merchantId],
  )
  const appSuggestions = useMemo(
    () => Array.from(new Set([
      ...suggestedAppIds,
      ...appActivityHealth.map((item) => item.requester_app_id),
      ...merchantBlocks.map((block) => block.requester_app_id),
    ])).filter((appId) => !['pc-web', 'mcp-client'].includes(appId)),
    [appActivityHealth, merchantBlocks, suggestedAppIds],
  )
  const attentionCount = appActivityHealth.filter((item) => item.status === 'attention').length

  async function blockApp(event: React.FormEvent) {
    event.preventDefault()
    setBusy('block')
    setMessage('')
    try {
      const outcome = await openCommerceApi.blockApp(projectId, {
        merchant_id: merchantId,
        requester_app_id: requesterAppId.trim(),
        reason_code: reasonCode,
        reason_note: reasonNote.trim(),
      })
      setRequesterAppId('')
      setReasonNote('')
      setMessage(`已封禁；撤销 ${outcome.revoked_grants} 项授权，取消 ${outcome.canceled_authorization_requests} 项待审批申请。`)
      await refresh()
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  async function unblockApp(block: OpenCommerceAppBlock) {
    setBusy(block.id)
    setMessage('')
    try {
      await openCommerceApi.unblockApp(projectId, block.id)
      setMessage('封禁已解除；旧授权没有恢复，该 App 需要重新申请授权。')
      await refresh()
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
        <strong>App 安全控制</strong>
        <span>{attentionCount} 个需关注 · {merchantBlocks.filter((block) => block.status === 'active').length} 个已封禁</span>
      </header>

      {appActivityHealth.map((health) => (
        <div key={health.requester_app_id} className={styles.capabilityRow}>
          <span>
            <strong>{health.requester_app_id}</strong>
            <small>
              近 24 小时 {health.total_invocations_24h} 次 · 成功 {health.succeeded_invocations_24h} · 失败 {health.failed_invocations_24h}
            </small>
            {health.attention_codes.length > 0 && (
              <small>{health.attention_codes.map(attentionLabel).join(' · ')}</small>
            )}
            {(health.rate_limited_invocations_24h > 0
              || health.grant_budget_rejections_24h > 0
              || health.recovered_invocations_24h > 0) && (
              <small>
                限流 {health.rate_limited_invocations_24h} · 授权预算拒绝 {health.grant_budget_rejections_24h} · 中断恢复 {health.recovered_invocations_24h}
              </small>
            )}
          </span>
          <span>
            <small>{formatTime(health.last_invoked_at)}</small>
            {health.status === 'attention' && (
              <button
                type="button"
                title="填入紧急封禁表单"
                onClick={() => setRequesterAppId(health.requester_app_id)}
                disabled={!canEdit}
              >
                <ShieldAlert size={16} aria-hidden="true" />
                处置
              </button>
            )}
          </span>
        </div>
      ))}
      {appActivityHealth.length === 0 && <p className={styles.empty}>近 24 小时暂无外部 App 调用。</p>}

      {merchantBlocks.map((block) => (
        <div key={block.id} className={styles.capabilityRow}>
          <span>
            <strong>{block.requester_app_id}</strong>
            <small>{reasonLabel(block.reason_code)} · {block.reason_note || '未填写补充说明'}</small>
          </span>
          <span>
            <small>{block.status === 'active' ? `封禁于 ${formatTime(block.blocked_at)}` : `解除于 ${formatTime(block.unblocked_at)}`}</small>
            {block.status === 'active' && (
              <button type="button" onClick={() => unblockApp(block)} disabled={!canEdit || busy === block.id}>
                {busy === block.id ? '解除中…' : '解除封禁'}
              </button>
            )}
          </span>
        </div>
      ))}
      {merchantBlocks.length === 0 && <p className={styles.empty}>暂无 App 封禁记录。</p>}

      <form className={styles.formCard} onSubmit={blockApp}>
        <header><strong>紧急封禁 App</strong><small>同时撤销授权并取消待审批申请</small></header>
        <label>开发者 App ID
          <input
            list="open-commerce-app-block-suggestions"
            value={requesterAppId}
            onChange={(event) => setRequesterAppId(event.target.value)}
            placeholder="consumer.partner-app"
            disabled={!canEdit}
            required
          />
          <datalist id="open-commerce-app-block-suggestions">
            {appSuggestions.map((appId) => <option key={appId} value={appId} />)}
          </datalist>
        </label>
        <label>原因
          <select value={reasonCode} onChange={(event) => setReasonCode(event.target.value as OpenCommerceAppBlockReason)} disabled={!canEdit}>
            {reasonOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
          </select>
        </label>
        <label>补充说明
          <textarea maxLength={500} value={reasonNote} onChange={(event) => setReasonNote(event.target.value)} disabled={!canEdit} />
        </label>
        <button type="submit" disabled={!canEdit || busy === 'block'}>{busy === 'block' ? '封禁中…' : '封禁并撤销授权'}</button>
      </form>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function reasonLabel(reason: OpenCommerceAppBlockReason) {
  return reasonOptions.find((option) => option.value === reason)?.label ?? reason
}

function attentionLabel(code: OpenCommerceAppActivityHealth['attention_codes'][number]) {
  const labels = {
    recovered_invocation: '存在中断恢复',
    repeated_failures: '失败次数较多',
    rate_limit_pressure: '多次触发配额',
    grant_budget_pressure: '触达授权总预算',
  }
  return labels[code]
}

function formatTime(value?: string) {
  return value ? new Date(value).toLocaleString('zh-CN') : '未知时间'
}

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return '操作失败，请稍后重试'
}
