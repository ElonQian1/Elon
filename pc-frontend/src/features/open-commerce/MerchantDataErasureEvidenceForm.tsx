import { useState } from 'react'
import { FilePlus2 } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { CreateConsumerDataErasureEvidence } from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import { actionStyle, commerceStyles } from './openCommerceStyles'

const initialDraft: CreateConsumerDataErasureEvidence = {
  evidence_kind: 'external_system_receipt',
  external_system: '',
  reference_id: '',
  receipt_sha256: '',
  summary: '',
  merchant_confirmed_unverified: false,
}

export default function MerchantDataErasureEvidenceForm({
  projectId,
  merchantId,
  requestId,
  onCreated,
}: {
  projectId: string
  merchantId: string
  requestId: string
  onCreated: () => Promise<void>
}) {
  const [draft, setDraft] = useState(initialDraft)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  async function submit(event: React.FormEvent) {
    event.preventDefault()
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.createMerchantDataErasureEvidence(
        projectId,
        merchantId,
        requestId,
        draft,
      )
      setDraft(initialDraft)
      setMessage('外部删除证明已追加。')
      await onCreated()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <form style={{ display: 'grid', gap: 7, paddingTop: 7, borderTop: '1px solid var(--line)' }} onSubmit={submit}>
      <label>
        证明类型
        <select value={draft.evidence_kind} onChange={(event) => setDraft((current) => ({ ...current, evidence_kind: event.target.value as CreateConsumerDataErasureEvidence['evidence_kind'] }))}>
          <option value="external_system_receipt">外部系统回执</option>
          <option value="merchant_attestation">商户声明</option>
        </select>
      </label>
      <label>
        外部系统
        <input value={draft.external_system} onChange={(event) => setDraft((current) => ({ ...current, external_system: event.target.value }))} maxLength={80} placeholder="例如：收银系统、会员系统" required />
      </label>
      <label>
        回执编号
        <input value={draft.reference_id} onChange={(event) => setDraft((current) => ({ ...current, reference_id: event.target.value }))} maxLength={160} required />
      </label>
      <label>
        回执 SHA-256
        <input value={draft.receipt_sha256} onChange={(event) => setDraft((current) => ({ ...current, receipt_sha256: event.target.value.trim().toLowerCase() }))} minLength={64} maxLength={64} pattern="[0-9a-f]{64}" required />
      </label>
      <label>
        证明摘要
        <input value={draft.summary} onChange={(event) => setDraft((current) => ({ ...current, summary: event.target.value }))} maxLength={500} required />
      </label>
      <label style={commerceStyles.checkRow}>
        <input type="checkbox" checked={draft.merchant_confirmed_unverified} onChange={(event) => setDraft((current) => ({ ...current, merchant_confirmed_unverified: event.target.checked }))} required />
        确认该证明由商户提供，平台尚未核验
      </label>
      <button style={actionStyle('secondary', busy || !draft.merchant_confirmed_unverified)} type="submit" disabled={busy || !draft.merchant_confirmed_unverified}>
        <FilePlus2 size={13} />追加证明
      </button>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </form>
  )
}
