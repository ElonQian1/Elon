import { X } from 'lucide-react'
import type { OpenCommerceActionConfirmation } from './openCommerceTypes'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles } from './openCommerceStyles'

export interface PendingActionConfirmationSummary {
  confirmation: OpenCommerceActionConfirmation
  merchantLabel: string
  capabilityLabel: string
}

export default function PendingActionConfirmationNotice({
  pending,
  busy,
  retryText,
  onCancel,
}: {
  pending: PendingActionConfirmationSummary
  busy: boolean
  retryText: string
  onCancel: () => void
}) {
  return (
    <section className={base.integrationSection}>
      <header>
        <strong>待处理经营操作</strong>
        <span style={badgeStyle('danger')} data-tone="danger">等待处理</span>
      </header>
      <div style={commerceStyles.sectionBody}>
        <p style={commerceStyles.itemText}>
          {pending.merchantLabel} · {pending.capabilityLabel}
        </p>
        <small style={commerceStyles.itemMeta}>{retryText}</small>
        <button
          style={actionStyle('secondary', busy)}
          type="button"
          onClick={onCancel}
          disabled={busy}
          title="取消本次经营操作"
        >
          <X size={14} />取消本次动作
        </button>
      </div>
    </section>
  )
}
