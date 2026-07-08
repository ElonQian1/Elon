import { clean } from '../../lib/utils'
import { toolEventSummary } from './devTaskUtils'
import { processCardFromToolEvent } from './taskProcessCardModel'
import type { TimelineItem } from './taskTimelineModel'
import type { ToolEvent } from './types'

export function mergeFileChangeResult(items: TimelineItem[], item: TimelineItem): boolean {
  const event = item.event
  if (event?.type !== 'tool_result' || clean(event.tool ?? '') !== 'file_change') return false
  const resultFiles = fileChangeTargets(event)
  for (let index = items.length - 1; index >= 0 && index >= items.length - 6; index--) {
    const previous = items[index]
    const previousEvent = previous.event
    if (previousEvent?.type !== 'tool_call' || clean(previousEvent.tool ?? '') !== 'file_change') continue
    if (!fileChangeEventsMatch(previousEvent, event, resultFiles)) continue
    const mergedEvent = mergeFileChangeEvents(previousEvent, event)
    items[index] = {
      ...previous,
      tone: item.tone,
      title: '文件修改',
      detail: toolEventSummary(mergedEvent, 140) || item.detail || previous.detail,
      meta: item.meta,
      metaTitle: item.metaTitle,
      event: mergedEvent,
      process: processCardFromToolEvent(mergedEvent) ?? item.process ?? previous.process,
      compact: item.compact,
    }
    return true
  }
  return false
}

function mergeFileChangeEvents(callEvent: ToolEvent, resultEvent: ToolEvent): ToolEvent {
  const args = {
    ...(callEvent.args ?? {}),
    ...(resultEvent.args ?? {}),
  }
  return {
    ...resultEvent,
    id: clean(resultEvent.id ?? '') || callEvent.id,
    args,
    diff: mergeFileChangeDiff(callEvent.diff, resultEvent.diff),
  }
}

function mergeFileChangeDiff(callDiff: ToolEvent['diff'], resultDiff: ToolEvent['diff']): ToolEvent['diff'] {
  if (!callDiff && !resultDiff) return undefined
  const files = Array.from(new Set([...(callDiff?.files ?? []), ...(resultDiff?.files ?? [])].map(clean).filter(Boolean)))
  return {
    preview: clean(resultDiff?.preview ?? '') || clean(callDiff?.preview ?? '') || undefined,
    files: files.length ? files : undefined,
    truncated: Boolean(callDiff?.truncated || resultDiff?.truncated) || undefined,
  }
}

function fileChangeEventsMatch(callEvent: ToolEvent, resultEvent: ToolEvent, resultFiles: string[]): boolean {
  const callId = clean(callEvent.id ?? '')
  const resultId = clean(resultEvent.id ?? '')
  if (callId && resultId && callId === resultId) return true
  const callFiles = fileChangeTargets(callEvent)
  if (!callFiles.length || !resultFiles.length) return true
  const resultFileSet = new Set(resultFiles)
  return callFiles.some((file) => resultFileSet.has(file))
}

function fileChangeTargets(event: ToolEvent): string[] {
  const files = new Set<string>()
  const add = (value: unknown) => {
    const text = clean(value)
    if (text) files.add(text)
  }
  add(event.args?.path)
  add(event.args?.file)
  if (Array.isArray(event.args?.changes)) {
    for (const change of event.args.changes) {
      if (change && typeof change === 'object') {
        const record = change as Record<string, unknown>
        add(record.path ?? record.file)
      }
    }
  }
  if (Array.isArray(event.diff?.files)) {
    for (const file of event.diff.files) add(file)
  }
  return Array.from(files)
}
