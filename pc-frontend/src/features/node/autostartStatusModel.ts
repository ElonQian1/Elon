import type { AutostartStatus } from './types'

export function autostartSummaryLabel(status: AutostartStatus | null): string {
  if (!status) return '检测中'
  if (!status.supported) return '不支持'
  if (!status.enabled) return '未开启'
  if (status.source === 'scheduled_task') return '已开启 · 计划任务'
  if (status.source === 'hkcu_run') return '已开启 · 待迁移'
  if (status.source?.startsWith('legacy_')) return '已开启 · 旧版'
  return '已开启'
}
