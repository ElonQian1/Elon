import { useCallback, useEffect, useMemo, useState } from 'react'
import { AlertTriangle, Check, RefreshCw, Undo2, X } from 'lucide-react'
import { taskEconomyApi } from './taskEconomyApi'
import type {
  SettlementDisputeDetail,
  SettlementDisputeReason,
  SettlementReceipt,
} from './taskEconomyTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

const reasonLabels: Record<SettlementDisputeReason, string> = {
  amount: '计量金额',
  provider_allocation: '节点分配',
  policy: '结算策略',
  source_evidence: '来源证据',
  other: '其他',
}

export default function SettlementDisputes({
  projectId,
  canEdit,
  selectedReceipt,
  onChanged,
}: {
  projectId: string
  canEdit: boolean
  selectedReceipt: SettlementReceipt | null
  onChanged: () => void
}) {
  const [cases, setCases] = useState<SettlementDisputeDetail[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [reasonCode, setReasonCode] = useState<SettlementDisputeReason>('amount')
  const [summary, setSummary] = useState('')
  const [evidenceRef, setEvidenceRef] = useState('')
  const [resolutionNote, setResolutionNote] = useState('')
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')

  const receiptId = selectedReceipt?.id ?? ''
  const refresh = useCallback(async () => {
    setMessage('')
    if (!receiptId) {
      setCases([])
      setSelectedId(null)
      return
    }
    try {
      const next = await taskEconomyApi.settlementDisputes(projectId, receiptId)
      setCases(next)
      setSelectedId((current) =>
        current && next.some((item) => item.dispute.id === current)
          ? current
          : (next[0]?.dispute.id ?? null),
      )
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId, receiptId])

  useEffect(() => {
    refresh()
  }, [refresh])

  const selected = useMemo(
    () => cases.find((item) => item.dispute.id === selectedId) ?? null,
    [cases, selectedId],
  )
  const hasBlockingCase = cases.some((item) => item.blocks_projection)
  const canOpen =
    canEdit && selectedReceipt?.status === 'reconciled' && !hasBlockingCase && busy === ''

  async function openCase(event: React.FormEvent) {
    event.preventDefault()
    if (!selectedReceipt) return
    setBusy('open')
    setMessage('')
    try {
      const opened = await taskEconomyApi.openSettlementDispute(projectId, selectedReceipt.id, {
        reason_code: reasonCode,
        summary: summary.trim(),
        evidence_ref: evidenceRef.trim() || undefined,
      })
      setSummary('')
      setEvidenceRef('')
      await refresh()
      setSelectedId(opened.dispute.id)
      onChanged()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy('')
    }
  }

  async function transition(action: 'accept' | 'reject' | 'withdraw') {
    if (!selected) return
    setBusy(action)
    setMessage('')
    try {
      if (action === 'withdraw') {
        await taskEconomyApi.withdrawSettlementDispute(
          projectId,
          selected.dispute.id,
          resolutionNote.trim(),
        )
      } else {
        await taskEconomyApi.resolveSettlementDispute(
          projectId,
          selected.dispute.id,
          action,
          resolutionNote.trim(),
        )
      }
      setResolutionNote('')
      await refresh()
      onChanged()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy('')
    }
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <strong>影子结算争议</strong>
        <div style={commerceStyles.headerActions}>
          {hasBlockingCase && <span style={badgeStyle('danger')}>投影已阻断</span>}
          <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新争议">
            <RefreshCw size={14} />
          </button>
        </div>
      </header>
      <div style={commerceStyles.sectionBody}>
        {!selectedReceipt && <p className={base.empty}>选择一张凭证后查看或提出争议。</p>}
        {selectedReceipt && (
          <div style={commerceStyles.grid}>
            <form className={base.formCard} onSubmit={openCase}>
              <header>
                <strong>提出争议</strong>
                <small>只追加案件和证据，不自动改账或退款</small>
              </header>
              <label>
                原因
                <select
                  value={reasonCode}
                  onChange={(event) => setReasonCode(event.target.value as SettlementDisputeReason)}
                  disabled={!canOpen}
                >
                  {Object.entries(reasonLabels).map(([value, label]) => (
                    <option key={value} value={value}>{label}</option>
                  ))}
                </select>
              </label>
              <label>
                摘要
                <textarea
                  value={summary}
                  onChange={(event) => setSummary(event.target.value)}
                  minLength={8}
                  maxLength={500}
                  disabled={!canOpen}
                  required
                />
              </label>
              <label>
                证据引用（可选）
                <input
                  value={evidenceRef}
                  onChange={(event) => setEvidenceRef(event.target.value)}
                  maxLength={512}
                  placeholder="Matter、Artifact 或审计记录 ID"
                  disabled={!canOpen}
                />
              </label>
              <button style={actionStyle('primary', !canOpen)} type="submit" disabled={!canOpen}>
                <AlertTriangle size={14} />{busy === 'open' ? '提交中' : '提交争议'}
              </button>
            </form>

            <div style={{ ...commerceStyles.list, ...commerceStyles.scrollArea }}>
              {cases.map((item) => (
                <button
                  className={base.formCard}
                  key={item.dispute.id}
                  style={listItemStyle(item.dispute.id === selectedId)}
                  type="button"
                  onClick={() => setSelectedId(item.dispute.id)}
                >
                  <header style={commerceStyles.itemHeader}>
                    <strong>{reasonLabels[item.dispute.reason_code]}</strong>
                    <span style={badgeStyle(item.blocks_projection ? 'danger' : 'neutral')}>
                      {item.dispute.status}
                    </span>
                  </header>
                  <p style={commerceStyles.itemText}>{item.dispute.summary}</p>
                  <code style={commerceStyles.itemMeta}>{item.dispute.id}</code>
                </button>
              ))}
              {cases.length === 0 && <p className={base.empty}>该凭证没有争议记录。</p>}
            </div>
          </div>
        )}

        {selected && (
          <div style={commerceStyles.list}>
            {selected.events.map((event) => (
              <div style={commerceStyles.priorityRow} key={event.id}>
                <span style={commerceStyles.priorityIndex}>{event.action.slice(0, 1).toUpperCase()}</span>
                <span>{event.action}{event.note ? ` · ${event.note}` : ''}</span>
                <code>{event.created_at}</code>
              </div>
            ))}
            {selected.dispute.status === 'open' && (
              <div className={base.formCard}>
                <label>
                  处理说明
                  <textarea
                    value={resolutionNote}
                    onChange={(event) => setResolutionNote(event.target.value)}
                    minLength={4}
                    maxLength={1000}
                    disabled={!canEdit || busy !== ''}
                  />
                </label>
                <div style={commerceStyles.headerActions}>
                  <button style={actionStyle('primary')} type="button" onClick={() => transition('accept')} disabled={!canEdit || busy !== '' || resolutionNote.trim().length < 4}>
                    <Check size={14} />接受
                  </button>
                  <button style={actionStyle('secondary')} type="button" onClick={() => transition('reject')} disabled={!canEdit || busy !== '' || resolutionNote.trim().length < 4}>
                    <X size={14} />驳回
                  </button>
                  <button style={actionStyle('secondary')} type="button" onClick={() => transition('withdraw')} disabled={!canEdit || busy !== '' || resolutionNote.trim().length < 4}>
                    <Undo2 size={14} />撤回
                  </button>
                </div>
              </div>
            )}
          </div>
        )}
        {message && <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{message}</div>}
      </div>
    </section>
  )
}
