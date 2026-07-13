import type { TaskTone } from './types'

export interface TaskTerminalActionModel {
  visible: boolean
  label: string
  requiresNode: boolean
}

const NO_ACTION: TaskTerminalActionModel = {
  visible: false,
  label: '',
  requiresNode: false,
}

export function taskTerminalActionModel(
  tone: TaskTone,
  content: string,
  reason = '',
): TaskTerminalActionModel {
  if (tone !== 'failed' && tone !== 'canceled') return NO_ACTION
  const text = `${content} ${reason}`
  const requiresNode = /(PC\s*节点|Win\s*端|本机节点|本机\s*AI|PC\s*CLI|Codex\s*CLI|通信中断|连接中断|节点.*(?:离线|断开)|重连)/i.test(text)
  if (tone === 'canceled') {
    return { visible: true, label: '继续任务', requiresNode }
  }
  if (/(回复未完成|没有返回.*(?:最终|收尾)回复|final_reply_missing|继续生成)/i.test(text)) {
    return { visible: true, label: '继续生成回复', requiresNode }
  }
  return { visible: true, label: '重试任务', requiresNode }
}
