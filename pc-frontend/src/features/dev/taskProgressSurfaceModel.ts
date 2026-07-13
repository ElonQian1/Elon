import type { TimelineItem, TimelineItemKind } from './taskTimelineModel'
import type { TaskTone } from './types'

export interface ProgressSurfaceItem {
  surfaceType?: 'text' | 'commands' | 'artifact'
  id: string
  title?: string
  detail?: string
  meta?: string
  kind?: TimelineItemKind
  items?: TimelineItem[]
  tone?: TaskTone
}

export function withRunningProgressFallback(items: ProgressSurfaceItem[]): ProgressSurfaceItem[] {
  if (items.length > 0) return items
  return [{
    surfaceType: 'text',
    id: 'running-progress-pending',
    title: '任务已发送',
    detail: '正在连接执行环境并等待第一条进展。收到节点或 AI 输出后会自动更新。',
    tone: 'queued',
  }]
}
