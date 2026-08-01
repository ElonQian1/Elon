import { useCallback, useEffect, useState } from 'react'
import { GitBranch, RefreshCw } from 'lucide-react'
import { taskEconomyApi } from './taskEconomyApi'
import type { SettlementCorrectionLineage } from './taskEconomyTypes'
import { errorText, formatMicros } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
} from './openCommerceStyles'

export default function SettlementLineage({
  projectId,
  receiptId,
  refreshToken,
}: {
  projectId: string
  receiptId: string | null
  refreshToken: number
}) {
  const [lineage, setLineage] = useState<SettlementCorrectionLineage | null>(null)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setMessage('')
    if (!receiptId) {
      setLineage(null)
      return
    }
    try {
      setLineage(await taskEconomyApi.settlementLineage(projectId, receiptId))
    } catch (error) {
      setLineage(null)
      setMessage(errorText(error))
    }
  }, [projectId, receiptId])

  useEffect(() => {
    refresh()
  }, [refresh, refreshToken])

  return (
    <section className={base.integrationSection}>
      <header>
        <strong>纠正链与当前有效凭证</strong>
        <div style={commerceStyles.headerActions}>
          {lineage && <span style={badgeStyle(lineage.effective_has_blocking_dispute ? 'danger' : 'neutral')}>{lineage.effective_has_blocking_dispute ? '有效凭证有争议' : `${lineage.depth} 次已过账纠正`}</span>}
          <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新纠正链">
            <RefreshCw size={14} />
          </button>
        </div>
      </header>
      <div style={commerceStyles.sectionBody}>
        {!receiptId && <p className={base.empty}>选择一张凭证查看它在纠正链中的位置。</p>}
        {receiptId && !lineage && !message && <p className={base.empty}>正在解析纠正链。</p>}
        {lineage && (
          <div style={commerceStyles.list}>
            <div style={commerceStyles.priorityRow}>
              <GitBranch size={15} />
              <span>{lineage.requested_position}</span>
              <code>{lineage.requested_receipt.id}</code>
            </div>
            <div style={commerceStyles.priorityRow}>
              <span style={commerceStyles.priorityIndex}>根</span>
              <code>{lineage.root_receipt.id}</code>
              <strong>{formatMicros(lineage.root_receipt.compute_amount_micros)}</strong>
            </div>
            {lineage.posted_corrections.map((item, index) => (
              <div style={commerceStyles.priorityRow} key={item.correction.id}>
                <span style={commerceStyles.priorityIndex}>{index + 1}</span>
                <code>{item.correction.id}</code>
                <span>{formatMicros(item.correction.corrected_compute_amount_micros)}</span>
              </div>
            ))}
            <div style={commerceStyles.priorityRow}>
              <span style={commerceStyles.priorityIndex}>效</span>
              <code>{lineage.effective_receipt.id}</code>
              <strong>{formatMicros(lineage.effective_receipt.compute_amount_micros)}</strong>
            </div>
            {lineage.non_posted_corrections.length > 0 && (
              <small style={commerceStyles.itemMeta}>
                另有 {lineage.non_posted_corrections.length} 条待验收或已取消计划，不改变当前有效金额。
              </small>
            )}
            <small style={commerceStyles.itemMeta}>冲销凭证只用于反向会计记录；经营与审计读取应使用“有效凭证”。</small>
          </div>
        )}
        {message && <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{message}</div>}
      </div>
    </section>
  )
}
