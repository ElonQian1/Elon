import { clean } from '../../lib/utils'
import type { UiTunerCodexContextPack } from './contextPack'
import type { UiTunerProjectSessionRecord } from './projectSessions'

export type UiTunerConversationMode = 'continue' | 'fork'

export function uiTunerElementLabel(pack: UiTunerCodexContextPack): string {
  const selected = pack.selectedElement
  if (!selected) return pack.screen.canvasName || '当前画布'
  const resource = clean(pack.runtimeBinding.resourceId)
  return resource ? `${selected.name} · ${resource}` : selected.name
}

export function uiTunerConversationSeed(pack: UiTunerCodexContextPack, intent: string): string {
  const cleanIntent = intent.trim()
  if (cleanIntent) return cleanIntent
  if (pack.headlessDesign) {
    return `请读取 ${pack.headlessDesign.platform.toUpperCase()} 端 ${pack.headlessDesign.route} 的后台 designSession，定位当前选区对应源码，完成修改并重新捕获 UI 树和截图哈希。`
  }
  return `请基于当前选中的 ${uiTunerElementLabel(pack)}，分析它的 XML 节点、源码映射和可复用 UI 标准，并给出可执行修改方案。`
}

export function uiTunerSessionLabel(session: UiTunerProjectSessionRecord | null): string {
  if (!session) return '新建 ui-tuner 会话'
  const element = clean(session.selectedElementName)
  return element ? `${session.title} · ${element}` : session.title
}

export function uiTunerSendDisabledReason(input: {
  hasProject: boolean
  hasChannel: boolean
  canStart: boolean
  localNodeStatusText: string
  workspacePath: string
}): string {
  if (!input.hasProject) return '请先选择自项目'
  if (!input.hasChannel) return '当前项目缺少 AI 开发频道'
  if (!input.workspacePath) return '自项目缺少本机工作区路径'
  if (!input.canStart) return input.localNodeStatusText || '项目 Codex 会话未就绪'
  return ''
}
