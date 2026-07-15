import type { ReplayIssue } from './model'

export interface ReplayDomSnapshot {
  frameIndex: number
  visibleBlocks: string[]
  contentHeight: number
}

export function auditReplayDom(root: HTMLElement, frameIndex: number, previous?: ReplayDomSnapshot): {
  issues: ReplayIssue[]
  snapshot: ReplayDomSnapshot
} {
  const issues: ReplayIssue[] = []
  const visibleElements = [...root.querySelectorAll<HTMLElement>('[data-task-visible-content]')]
    .filter((element) => isVisible(element) && !element.querySelector('[data-task-visible-content]'))
  const visibleBlocks = visibleElements.map((element) => normalizeText(element.innerText)).filter(Boolean)
  const taskTurns = [...root.querySelectorAll<HTMLElement>('[data-task-turn]')].filter(isVisible)
  for (const turn of taskTurns) {
    const body = turn.querySelector<HTMLElement>('[data-task-assistant-body]')
    const content = turn.querySelector<HTMLElement>('[data-task-visible-content]')
    if (body && isVisible(body) && !content && normalizeText(body.innerText).replace(/一龙|正在处理|最终回复|任务失败|任务完成/g, '').length < 8) {
      issues.push(domIssue('blank-assistant-turn', 'error', 'AI 区域没有可见内容', '任务头像和状态已经出现，但正文仍为空。', frameIndex))
      break
    }
  }

  const duplicate = firstDuplicate(visibleBlocks)
  if (duplicate) {
    issues.push(domIssue('duplicate-visible-content', 'warning', '可见内容重复', `同一帧重复显示“${clip(duplicate, 48)}”。`, frameIndex))
  }
  if (root.querySelector('details details')) {
    issues.push(domIssue('nested-details', 'warning', '存在嵌套折叠层', '工具详情中再次嵌套 details，阅读层级可能过重。', frameIndex))
  }

  const feed = root.querySelector<HTMLElement>('[data-replay-feed]')
  const composer = root.querySelector<HTMLElement>('[data-replay-composer]')
  if (feed && composer && rectanglesOverlap(feed.getBoundingClientRect(), composer.getBoundingClientRect())) {
    issues.push(domIssue('composer-overlap', 'error', '输入框遮挡对话', '对话内容与底部输入框发生重叠。', frameIndex))
  }

  const contentHeight = feed?.scrollHeight ?? root.scrollHeight
  if (previous && frameIndex >= previous.frameIndex && previous.visibleBlocks.length > visibleBlocks.length + 1) {
    issues.push(domIssue('forward-content-disappeared', 'warning', '前进后公开内容减少', `上一帧有 ${previous.visibleBlocks.length} 个内容块，本帧只剩 ${visibleBlocks.length} 个。`, frameIndex))
  }
  return { issues, snapshot: { frameIndex, visibleBlocks, contentHeight } }
}

function isVisible(element: HTMLElement): boolean {
  const style = window.getComputedStyle(element)
  return style.display !== 'none' && style.visibility !== 'hidden' && element.getClientRects().length > 0
}

function normalizeText(value: string): string {
  return value.replace(/\s+/g, ' ').trim()
}

function firstDuplicate(values: string[]): string {
  const seen: string[] = []
  for (const value of values) {
    if (value.length < 12) continue
    const duplicate = seen.find((previous) => previous === value
      || (Math.min(previous.length, value.length) >= 24 && (previous.includes(value) || value.includes(previous))))
    if (duplicate) return value.length <= duplicate.length ? value : duplicate
    seen.push(value)
  }
  return ''
}

function rectanglesOverlap(left: DOMRect, right: DOMRect): boolean {
  return left.left < right.right && left.right > right.left && left.top < right.bottom && left.bottom > right.top
}

function clip(value: string, length: number): string {
  return value.length > length ? `${value.slice(0, length)}…` : value
}

function domIssue(id: string, severity: ReplayIssue['severity'], title: string, detail: string, frameIndex: number): ReplayIssue {
  return { id: `${id}-${frameIndex}`, severity, title, detail, frameIndex }
}
