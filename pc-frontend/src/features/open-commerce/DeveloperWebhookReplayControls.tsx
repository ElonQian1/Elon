import { useState } from 'react'
import { History } from 'lucide-react'
import type { DeveloperWebhookHistoryReplayResult } from './openCommerceClientTypes'
import { actionStyle, commerceStyles } from './openCommerceStyles'

export default function DeveloperWebhookReplayControls({
  disabled,
  onReplay,
  onMessage,
}: {
  disabled: boolean
  onReplay: (
    afterSequence: number,
    limit: number,
  ) => Promise<DeveloperWebhookHistoryReplayResult | undefined>
  onMessage: (message: string) => void
}) {
  const [afterSequence, setAfterSequence] = useState('0')
  const [limit, setLimit] = useState('50')
  const [submitting, setSubmitting] = useState(false)

  async function replay() {
    const parsedSequence = Number(afterSequence)
    const parsedLimit = Number(limit)
    if (
      !Number.isInteger(parsedSequence)
      || parsedSequence < 0
      || !Number.isInteger(parsedLimit)
      || parsedLimit < 1
      || parsedLimit > 100
    ) {
      onMessage('历史补发序号须为非负整数，单次数量须为 1 到 100。')
      return
    }
    setSubmitting(true)
    try {
      const result = await onReplay(parsedSequence, parsedLimit)
      if (result) setAfterSequence(String(result.processed_through_sequence))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div style={commerceStyles.itemHeader}>
      <input
        type="number"
        min="0"
        style={{ width: 96 }}
        value={afterSequence}
        onChange={(event) => setAfterSequence(event.target.value)}
        disabled={disabled || submitting}
        title="从此终态序号之后补发"
      />
      <input
        type="number"
        min="1"
        max="100"
        style={{ width: 72 }}
        value={limit}
        onChange={(event) => setLimit(event.target.value)}
        disabled={disabled || submitting}
        title="单次补发数量"
      />
      <button
        style={actionStyle('icon', disabled || submitting)}
        type="button"
        onClick={replay}
        disabled={disabled || submitting}
        title="补发历史终态通知"
      >
        <History size={13} />
      </button>
    </div>
  )
}
