import type { LocalAiMessageSnapshot } from './localAiBrowserApi'

export interface LocalAiHistoryWindow {
  syncedCount: number
  observedCount: number
  windowStart: number
  complete: boolean
  label: string
}

export function localAiHistoryWindow(snapshot: LocalAiMessageSnapshot | null): LocalAiHistoryWindow {
  const syncedCount = snapshot?.messages.length ?? 0
  const windowStart = nonNegativeInteger(snapshot?.messageWindowStart)
  const observedCount = Math.max(
    nonNegativeInteger(snapshot?.observedMessageCount),
    windowStart + syncedCount,
  )
  const complete = snapshot !== null && windowStart === 0 && syncedCount >= observedCount
  const label = !snapshot
    ? '当前会话历史待同步'
    : complete
      ? `当前会话已完整同步 · ${syncedCount} 条`
      : `当前显示最近窗口 · 已同步 ${syncedCount} / 官网观察 ${observedCount} 条`
  return { syncedCount, observedCount, windowStart, complete, label }
}

function nonNegativeInteger(value: number | undefined): number {
  return Number.isFinite(value) ? Math.max(0, Math.floor(value!)) : 0
}
