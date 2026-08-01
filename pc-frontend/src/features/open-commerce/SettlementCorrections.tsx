import { useCallback, useEffect, useMemo, useState } from 'react'
import { CheckCircle2, RefreshCw, RotateCcw } from 'lucide-react'
import { taskEconomyApi } from './taskEconomyApi'
import type {
  SettlementCorrectionDetail,
  SettlementDisputeDetail,
  SettlementReceipt,
} from './taskEconomyTypes'
import { errorText, formatMicros } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

export default function SettlementCorrections({
  projectId,
  canEdit,
  selectedReceipt,
  selectedDispute,
  refreshToken,
  onChanged,
}: {
  projectId: string
  canEdit: boolean
  selectedReceipt: SettlementReceipt
  selectedDispute: SettlementDisputeDetail
  refreshToken: number
  onChanged: () => void
}) {
  const [items, setItems] = useState<SettlementCorrectionDetail[]>([])
  const [computeYuan, setComputeYuan] = useState(microsToInput(selectedReceipt.compute_amount_micros))
  const [providerYuan, setProviderYuan] = useState(microsToInput(selectedReceipt.provider_amount_micros))
  const [summary, setSummary] = useState('')
  const [evidenceRef, setEvidenceRef] = useState('')
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setMessage('')
    try {
      setItems(await taskEconomyApi.settlementCorrections(projectId, selectedReceipt.id))
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId, selectedReceipt.id])

  useEffect(() => {
    refresh()
  }, [refresh, refreshToken])

  useEffect(() => {
    setComputeYuan(microsToInput(selectedReceipt.compute_amount_micros))
    setProviderYuan(microsToInput(selectedReceipt.provider_amount_micros))
    setSummary('')
    setEvidenceRef('')
  }, [
    selectedDispute.dispute.id,
    selectedReceipt.compute_amount_micros,
    selectedReceipt.provider_amount_micros,
  ])

  const disputeItems = useMemo(
    () => items.filter((item) => item.correction.dispute_id === selectedDispute.dispute.id),
    [items, selectedDispute.dispute.id],
  )
  const hasActive = disputeItems.some((item) => item.correction.status !== 'canceled')
  const canCreate =
    canEdit && selectedDispute.dispute.status === 'accepted' && !hasActive && busy === ''

  async function createCorrection(event: React.FormEvent) {
    event.preventDefault()
    const compute = yuanToMicros(computeYuan)
    const provider = yuanToMicros(providerYuan)
    if (compute === null || provider === null) {
      setMessage('金额必须是大于等于零的有效数字。')
      return
    }
    setBusy('create')
    setMessage('')
    try {
      await taskEconomyApi.createSettlementCorrection(projectId, selectedDispute.dispute.id, {
        corrected_compute_amount_micros: compute,
        corrected_provider_amount_micros: provider,
        summary: summary.trim(),
        evidence_ref: evidenceRef.trim() || undefined,
      })
      setSummary('')
      setEvidenceRef('')
      await refresh()
      onChanged()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy('')
    }
  }

  async function finalize(item: SettlementCorrectionDetail) {
    setBusy(item.correction.id)
    setMessage('')
    try {
      await taskEconomyApi.finalizeSettlementCorrection(projectId, item.correction.id)
      await refresh()
      onChanged()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy('')
    }
  }

  return (
    <div style={commerceStyles.list}>
      <header style={commerceStyles.itemHeader}>
        <div>
          <strong>追加式纠正</strong>
          <small>先建 Matter 核查，人工验收后原子追加冲销与替换凭证</small>
        </div>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新纠正流程">
          <RefreshCw size={14} />
        </button>
      </header>
      {selectedDispute.dispute.status !== 'accepted' && (
        <p className={base.empty}>只有审核为 accepted 的争议可以进入纠正流程。</p>
      )}
      {selectedDispute.dispute.status === 'accepted' && (
        <form style={commerceStyles.grid} onSubmit={createCorrection}>
          <label>
            纠正后计算金额（元）
            <input type="number" min="0" step="0.000001" value={computeYuan} onChange={(event) => setComputeYuan(event.target.value)} disabled={!canCreate} required />
          </label>
          <label>
            纠正后节点金额（元）
            <input type="number" min="0" step="0.000001" value={providerYuan} onChange={(event) => setProviderYuan(event.target.value)} disabled={!canCreate} required />
          </label>
          <label>
            纠正说明
            <textarea value={summary} onChange={(event) => setSummary(event.target.value)} minLength={8} maxLength={1000} disabled={!canCreate} required />
          </label>
          <label>
            证据引用（可选）
            <input value={evidenceRef} onChange={(event) => setEvidenceRef(event.target.value)} maxLength={512} disabled={!canCreate} placeholder="Artifact 或审计记录 ID" />
          </label>
          <button style={actionStyle('primary', !canCreate)} type="submit" disabled={!canCreate}>
            <RotateCcw size={14} />{busy === 'create' ? '创建中' : '创建纠正 Matter'}
          </button>
        </form>
      )}
      <div style={commerceStyles.list}>
        {disputeItems.map((item) => {
          const correction = item.correction
          const matterAccepted = correction.matter_status === 'done' && correction.matter_final_decision === 'accepted'
          return (
            <article className={base.formCard} style={listItemStyle()} key={correction.id}>
              <header style={commerceStyles.itemHeader}>
                <strong>{formatMicros(correction.corrected_compute_amount_micros)} · 节点 {formatMicros(correction.corrected_provider_amount_micros)}</strong>
                <span style={badgeStyle(correction.status === 'canceled' ? 'warn' : 'neutral')}>{correction.status}</span>
              </header>
              <p style={commerceStyles.itemText}>{correction.summary}</p>
              <small style={commerceStyles.itemMeta}>Matter {correction.correction_matter_id} · {correction.matter_status}{correction.matter_final_decision ? ` / ${correction.matter_final_decision}` : ''}</small>
              {correction.status === 'posted' && (
                <code style={commerceStyles.itemMeta}>冲销 {correction.reversal_receipt_id} · 替换 {correction.replacement_receipt_id}</code>
              )}
              {correction.status === 'matter_pending' && (
                <button style={actionStyle('secondary', !matterAccepted)} type="button" onClick={() => finalize(item)} disabled={!canEdit || !matterAccepted || busy !== ''}>
                  <CheckCircle2 size={14} />{busy === correction.id ? '过账中' : '重试纠正过账'}
                </button>
              )}
              {item.events.map((event) => (
                <small style={commerceStyles.itemMeta} key={event.id}>{event.action} · {event.created_at}</small>
              ))}
            </article>
          )
        })}
      </div>
      <div style={commerceStyles.message}>纠正只改变链外影子账本的追加视图，不修改真实余额、节点提现、退款或链上资产。</div>
      {message && <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{message}</div>}
    </div>
  )
}

function microsToInput(value: number) {
  return (value / 1_000_000).toFixed(6).replace(/\.?0+$/, '') || '0'
}

function yuanToMicros(value: string) {
  const number = Number(value)
  if (!Number.isFinite(number) || number < 0) return null
  return Math.round(number * 1_000_000)
}
