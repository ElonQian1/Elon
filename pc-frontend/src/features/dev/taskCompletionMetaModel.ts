import { clean } from '../../lib/utils'
import { usageEventSummary } from './devTaskUtils'
import type { TaskTimelineModel } from './taskTimelineModel'

export interface TaskCompletionMetaModel {
  model: string
  usage: string
}

export function taskCompletionMetaModel(
  timeline: Pick<TaskTimelineModel, 'items'>,
): TaskCompletionMetaModel | null {
  for (let index = timeline.items.length - 1; index >= 0; index--) {
    const event = timeline.items[index].event
    if (event?.type !== 'usage') continue
    const structuredUsage = usageEventSummary(event)
    const messageUsage = clean(event.message ?? '')
    return {
      model: clean(event.model ?? ''),
      usage: structuredUsage === '已记录本轮用量' && messageUsage
        ? messageUsage
        : structuredUsage,
    }
  }
  return null
}
