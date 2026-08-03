import { useState, type CSSProperties, type FormEvent } from 'react'
import { Archive, Check, RotateCcw, X } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { DeveloperWebhookDelivery } from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import { actionStyle, badgeStyle } from './openCommerceStyles'

export default function DeveloperWebhookDeadLetterActions({
  projectId,
  appRecordId,
  webhookId,
  delivery,
  disabled,
  onChanged,
  onMessage,
}: {
  projectId: string
  appRecordId: string
  webhookId: string
  delivery: DeveloperWebhookDelivery
  disabled: boolean
  onChanged: () => Promise<void>
  onMessage: (message: string) => void
}) {
  const [acknowledging, setAcknowledging] = useState(false)
  const [reason, setReason] = useState('')
  const [busy, setBusy] = useState(false)

  async function retry() {
    setBusy(true)
    try {
      await openCommerceClientApi.retryDeveloperWebhookDelivery(
        projectId,
        appRecordId,
        webhookId,
        delivery.id,
      )
      onMessage('死信已重新进入投递队列。')
      await onChanged()
    } catch (error) {
      onMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function acknowledge(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    try {
      await openCommerceClientApi.acknowledgeDeveloperWebhookDeadLetter(
        projectId,
        appRecordId,
        webhookId,
        delivery.id,
        reason,
      )
      setAcknowledging(false)
      setReason('')
      onMessage('死信已确认处理，原失败记录仍保留。')
      await onChanged()
    } catch (error) {
      onMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  if (delivery.status !== 'dead') return null
  const unavailable = disabled || busy
  return (
    <div style={actionsStyle}>
      {delivery.dead_letter_acknowledged_at && (
        <span style={badgeStyle('neutral')} title={delivery.dead_letter_acknowledgement_reason}>
          已确认
        </span>
      )}
      <button
        style={actionStyle('icon', unavailable)}
        type="button"
        onClick={retry}
        disabled={unavailable}
        title="重新投递死信"
      >
        <RotateCcw size={13} />
      </button>
      {!delivery.dead_letter_acknowledged_at && !acknowledging && (
        <button
          style={actionStyle('icon', unavailable)}
          type="button"
          onClick={() => setAcknowledging(true)}
          disabled={unavailable}
          title="确认不再处理"
        >
          <Archive size={13} />
        </button>
      )}
      {acknowledging && (
        <form style={acknowledgeFormStyle} onSubmit={acknowledge}>
          <input
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            placeholder="处理原因"
            minLength={4}
            maxLength={500}
            disabled={unavailable}
            required
          />
          <button style={actionStyle('icon', unavailable)} disabled={unavailable} title="确认">
            <Check size={13} />
          </button>
          <button
            style={actionStyle('icon', unavailable)}
            type="button"
            onClick={() => {
              setAcknowledging(false)
              setReason('')
            }}
            disabled={unavailable}
            title="取消"
          >
            <X size={13} />
          </button>
        </form>
      )}
    </div>
  )
}

const actionsStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  flexWrap: 'wrap',
  justifyContent: 'flex-end',
  gap: 6,
}

const acknowledgeFormStyle: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'minmax(120px, 220px) 30px 30px',
  alignItems: 'center',
  gap: 5,
}
