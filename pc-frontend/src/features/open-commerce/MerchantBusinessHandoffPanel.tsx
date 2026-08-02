import { useCallback, useEffect, useMemo, useState } from 'react'
import { Check, RefreshCw, X } from 'lucide-react'
import { openCommerceApi } from './openCommerceApi'
import type {
  MerchantBusinessEvidenceSummary,
  OpenCommerceBusinessHandoffReceiptList,
  OpenCommerceBusinessHandoffStatus,
  OpenCommerceBusinessHandoffTarget,
  OpenCommerceIntegration,
} from './openCommerceTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle } from './openCommerceStyles'
import styles from './MerchantBusinessHandoffPanel.module.css'

type Props = {
  projectId: string
  merchantId: string
  evidence: MerchantBusinessEvidenceSummary | null
  integrations: OpenCommerceIntegration[]
  canEdit: boolean
  onClose: () => void
  onRecorded: () => void
}

const outcomeOptions: Array<{ value: OpenCommerceBusinessHandoffStatus; label: string }> = [
  { value: 'applied', label: '已进入系统' },
  { value: 'ignored', label: '无需处理' },
  { value: 'rejected', label: '处理失败' },
]

const reasonOptions = {
  ignored: [
    ['already_present', '目标记录已存在'],
    ['duplicate', '重复业务证据'],
    ['not_applicable', '无需进入该系统'],
  ],
  rejected: [
    ['adapter_failed', '接入器处理失败'],
    ['invalid_data', '业务数据不完整'],
    ['permission_denied', '外部系统拒绝访问'],
  ],
}

export default function MerchantBusinessHandoffPanel({
  projectId,
  merchantId,
  evidence,
  integrations,
  canEdit,
  onClose,
  onRecorded,
}: Props) {
  const [list, setList] = useState<OpenCommerceBusinessHandoffReceiptList | null>(null)
  const [integrationId, setIntegrationId] = useState('')
  const [status, setStatus] = useState<OpenCommerceBusinessHandoffStatus>('applied')
  const [target, setTarget] = useState<OpenCommerceBusinessHandoffTarget>('erp')
  const [targetReference, setTargetReference] = useState('')
  const [reasonCode, setReasonCode] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [attempt, setAttempt] = useState(newAttempt)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null)

  const availableIntegrations = useMemo(
    () => integrations.filter((item) => item.merchant_id === merchantId && item.status !== 'disabled'),
    [integrations, merchantId],
  )

  const refresh = useCallback(async () => {
    if (!projectId || !merchantId) return
    setBusy(true)
    try {
      setList(await openCommerceApi.listBusinessHandoffReceipts(projectId, merchantId))
    } catch (error) {
      setMessage({ text: errorText(error), error: true })
    } finally {
      setBusy(false)
    }
  }, [merchantId, projectId])

  useEffect(() => {
    void refresh()
  }, [refresh])

  useEffect(() => {
    if (!evidence) return
    setIntegrationId(availableIntegrations[0]?.id ?? '')
    setStatus(evidence.receipt_state === 'valid' ? 'applied' : 'rejected')
    setTarget('erp')
    setTargetReference(evidence.business_receipt?.reference_id ?? '')
    setReasonCode(evidence.receipt_state === 'valid' ? '' : 'invalid_data')
    setConfirmed(false)
    setAttempt(newAttempt())
    setMessage(null)
  }, [availableIntegrations, evidence])

  async function submit() {
    if (!evidence?.result_sha256 || !integrationId || !confirmed) return
    setBusy(true)
    setMessage(null)
    try {
      await openCommerceApi.recordBusinessHandoffReceipt(projectId, {
        merchant_id: merchantId,
        invocation_id: evidence.invocation_id,
        integration_id: integrationId,
        receipt_key: attempt.receiptKey,
        status,
        target_domain: target,
        evidence_result_sha256: evidence.result_sha256,
        target_reference: status === 'applied' ? targetReference.trim() : undefined,
        error_code: status === 'applied' ? undefined : reasonCode,
        confirmed_by_user: true,
        completed_at: attempt.completedAt,
      })
      setMessage({ text: '衔接结果已记录', error: false })
      onRecorded()
      onClose()
      await refresh()
    } catch (error) {
      setMessage({ text: errorText(error), error: true })
    } finally {
      setBusy(false)
    }
  }

  const submitDisabled = busy
    || !canEdit
    || !evidence?.result_sha256
    || !integrationId
    || !confirmed
    || (status === 'applied' ? !targetReference.trim() : !reasonCode)

  return (
    <section className={base.integrationSection}>
      <header>
        <span>
          <strong>ERP / CRM 衔接记录</strong>
          <small>记录接入器的实际处理结果，不创建平台订单，不代表真实资金流转。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新衔接记录">
          <RefreshCw size={14} />
        </button>
      </header>
      <div className={styles.body}>
        {evidence && canEdit && (
          <div className={styles.recorder}>
            <header>
              <span>
                <strong>{evidence.capability_key}</strong>
                <small>{evidence.business_receipt?.reference_id ?? `调用 ${evidence.invocation_id.slice(0, 12)}…`}</small>
              </span>
              <button style={actionStyle('icon', busy)} type="button" onClick={onClose} disabled={busy} title="关闭记录表单">
                <X size={14} />
              </button>
            </header>
            <div className={styles.formGrid}>
              <label className={styles.field}>
                接入器
                <select value={integrationId} onChange={(event) => setIntegrationId(event.target.value)} disabled={busy}>
                  <option value="">选择商户接入器</option>
                  {availableIntegrations.map((item) => <option key={item.id} value={item.id}>{item.display_name}</option>)}
                </select>
              </label>
              <label className={styles.field}>
                处理结果
                <span className={styles.segmented}>
                  {outcomeOptions.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      aria-pressed={status === option.value}
                      disabled={busy || (option.value === 'applied' && evidence.receipt_state !== 'valid')}
                      onClick={() => {
                        setStatus(option.value)
                        setReasonCode(option.value === 'ignored' ? 'already_present' : option.value === 'rejected' ? 'adapter_failed' : '')
                      }}
                    >
                      {option.label}
                    </button>
                  ))}
                </span>
              </label>
              <label className={styles.field}>
                目标系统
                <span className={`${styles.segmented} ${styles.two}`}>
                  {(['erp', 'crm'] as const).map((value) => (
                    <button key={value} type="button" aria-pressed={target === value} disabled={busy} onClick={() => setTarget(value)}>
                      {value.toUpperCase()}
                    </button>
                  ))}
                </span>
              </label>
              {status === 'applied' ? (
                <label className={styles.field}>
                  目标记录号
                  <input value={targetReference} onChange={(event) => setTargetReference(event.target.value)} disabled={busy} maxLength={160} />
                </label>
              ) : (
                <label className={styles.field}>
                  处理原因
                  <select value={reasonCode} onChange={(event) => setReasonCode(event.target.value)} disabled={busy}>
                    <option value="">选择处理原因</option>
                    {reasonOptions[status].map(([value, label]) => <option key={value} value={value}>{label}</option>)}
                  </select>
                </label>
              )}
            </div>
            <label className={styles.confirmation}>
              <input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} disabled={busy} />
              我已核对该接入器的真实处理结果
            </label>
            <div className={styles.actions}>
              <button style={actionStyle('primary', submitDisabled)} type="button" onClick={submit} disabled={submitDisabled}>
                <Check size={14} />保存衔接结果
              </button>
            </div>
          </div>
        )}

        {availableIntegrations.length === 0 && canEdit && (
          <p className={styles.boundary}>当前商户没有可用接入器，请先在“数据接入”中完成登记。</p>
        )}
        <div className={styles.receiptList}>
          {(list?.receipts ?? []).map((receipt) => {
            const integration = integrations.find((item) => item.id === receipt.integration_id)
            return (
              <article className={styles.receipt} key={receipt.id}>
                <header>
                  <strong>{integration?.display_name ?? receipt.target_domain.toUpperCase()}</strong>
                  <span style={badgeStyle(receiptTone(receipt.status))}>{receiptLabel(receipt.status)}</span>
                </header>
                <p>{receipt.error_code ? reasonLabel(receipt.error_code) : `目标摘要 ${receipt.target_reference_sha256?.slice(0, 12)}…`}</p>
                <small className={styles.receiptMeta}>
                  {authorityLabel(receipt.assertion_authority, receipt.adapter_credential_version)} · 未扣真实资金 · {new Date(receipt.completed_at).toLocaleString('zh-CN')}
                </small>
              </article>
            )
          })}
        </div>
        {(list?.receipts.length ?? 0) === 0 && <p className={base.empty}>暂无 ERP / CRM 衔接记录。</p>}
        {message && <p className={`${styles.message} ${message.error ? styles.error : styles.success}`}>{message.text}</p>}
      </div>
    </section>
  )
}

function newAttempt() {
  const random = globalThis.crypto?.randomUUID?.() ?? Math.random().toString(36).slice(2)
  return {
    receiptKey: `pc-handoff-${random}`,
    completedAt: new Date().toISOString(),
  }
}

function receiptLabel(status: OpenCommerceBusinessHandoffStatus) {
  return status === 'applied' ? '已进入系统' : status === 'ignored' ? '无需处理' : '处理失败'
}

function receiptTone(status: OpenCommerceBusinessHandoffStatus): 'neutral' | 'warn' | 'danger' {
  return status === 'applied' ? 'neutral' : status === 'ignored' ? 'warn' : 'danger'
}

function reasonLabel(code: string) {
  const labels = Object.fromEntries([...reasonOptions.ignored, ...reasonOptions.rejected])
  return labels[code] ?? code
}

function authorityLabel(
  authority: 'project_editor_asserted' | 'adapter_token_authenticated',
  credentialVersion?: number,
) {
  return authority === 'adapter_token_authenticated'
    ? `接入器鉴权${credentialVersion ? ` v${credentialVersion}` : ''}`
    : '人工确认'
}
