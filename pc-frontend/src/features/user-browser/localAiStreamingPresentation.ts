import type { LocalAiVisibleMessage } from './localAiBrowserApi'

export interface LocalAiStreamingTarget {
  messageId: string
  synthetic: boolean
}

export function localAiStreamingTarget(
  messages: LocalAiVisibleMessage[],
  snapshotStreaming: boolean,
): LocalAiStreamingTarget | null {
  let latestUserIndex = -1
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (messages[index].role === 'user') {
      latestUserIndex = index
      break
    }
  }
  for (let index = messages.length - 1; index > latestUserIndex; index -= 1) {
    const message = messages[index]
    if (message.role === 'assistant' && message.state === 'streaming') {
      return { messageId: message.id, synthetic: false }
    }
  }
  if (!snapshotStreaming) return null

  let candidate: LocalAiVisibleMessage | undefined
  for (let index = messages.length - 1; index > latestUserIndex; index -= 1) {
    if (messages[index].role === 'assistant') {
      candidate = messages[index]
      break
    }
  }
  return candidate
    ? { messageId: candidate.id, synthetic: false }
    : { messageId: 'snapshot-progress', synthetic: true }
}

export function localAiStreamingStatus({
  officialStatus,
  pendingSlow,
  providerName,
}: {
  officialStatus?: string
  pendingSlow: boolean
  providerName?: string
}) {
  const status = String(officialStatus ?? '').trim()
  if (status) return status
  const name = providerName || '网页 AI'
  return pendingSlow
    ? `${name} 已发送 · 回答同步较慢，可继续等待或打开官方页确认`
    : `${name} 正在回答…`
}

export function isLocalAiSearchProgress(value: string) {
  return /^(?:正在(?:搜索|查询|浏览)|searching|browsing|looking up)\b/i.test(value.trim())
    || /^(?:正在(?:搜索|查询|浏览))/.test(value.trim())
}
