import { v4 as uuidv4 } from 'uuid'
import type { UiTunerCodexContextPack } from './contextPack'

const MEMORY_KEY = 'elon.uiTuner.moduleMemory.v1'
const SESSIONS_KEY = 'elon.uiTuner.projectSessions.v1'

export interface UiTunerModuleMemory {
  version: 1
  module: 'ui-tuner'
  updatedAt: string
  stableSummary: string
  acceptedDecisions: string[]
  openQuestions: string[]
  preferredStandards: string[]
}

export interface UiTunerProjectSessionRecord {
  id: string
  conversationId: string
  title: string
  projectId: string
  channelId: string
  createdAt: string
  updatedAt: string
  sourceSessionId?: string
  sourceSummary?: string
  selectedElementName?: string
  taskId?: string
  status?: string
}

interface BuildTaskContentInput {
  pack: UiTunerCodexContextPack
  intent: string
  memory: UiTunerModuleMemory
  session: UiTunerProjectSessionRecord
  mode: 'continue' | 'fork'
}

export function readUiTunerModuleMemory(): UiTunerModuleMemory {
  const fallback = createDefaultModuleMemory()
  if (typeof window === 'undefined') return fallback
  try {
    const parsed = JSON.parse(window.localStorage.getItem(MEMORY_KEY) || '') as UiTunerModuleMemory
    if (parsed?.version === 1 && parsed.module === 'ui-tuner') return normalizeMemory(parsed)
  } catch {
    // Ignore corrupt local draft memory.
  }
  return fallback
}

export function writeUiTunerModuleMemory(memory: UiTunerModuleMemory) {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(MEMORY_KEY, JSON.stringify(normalizeMemory(memory)))
}

export function rememberUiTunerIntent(memory: UiTunerModuleMemory, intent: string, elementName: string) {
  const cleanIntent = intent.trim()
  const nextSummary = cleanIntent
    ? `最近目标：围绕 ${elementName || '当前 APK 节点'}，${cleanIntent}`
    : memory.stableSummary
  return normalizeMemory({
    ...memory,
    updatedAt: new Date().toISOString(),
    stableSummary: nextSummary,
    openQuestions: uniqueCompact([
      ...memory.openQuestions,
      'Codex 完成后需要把源码改动、UI 标准配置和真机验收结果回写到模块记忆。',
    ]),
  })
}

export function readUiTunerProjectSessions(): UiTunerProjectSessionRecord[] {
  if (typeof window === 'undefined') return []
  try {
    const parsed = JSON.parse(window.localStorage.getItem(SESSIONS_KEY) || '') as UiTunerProjectSessionRecord[]
    if (Array.isArray(parsed)) return parsed.filter((item) => item?.conversationId).slice(0, 24)
  } catch {
    // Ignore corrupt local session index.
  }
  return []
}

export function saveUiTunerProjectSession(record: UiTunerProjectSessionRecord) {
  if (typeof window === 'undefined') return
  const sessions = readUiTunerProjectSessions()
  const next = [
    record,
    ...sessions.filter((item) => item.id !== record.id && item.conversationId !== record.conversationId),
  ].slice(0, 24)
  window.localStorage.setItem(SESSIONS_KEY, JSON.stringify(next))
}

export function createUiTunerProjectSession(input: {
  projectId: string
  channelId: string
  elementName: string
  source?: UiTunerProjectSessionRecord | null
  memory: UiTunerModuleMemory
}): UiTunerProjectSessionRecord {
  const now = new Date().toISOString()
  const shortId = uuidv4()
  const element = input.elementName.trim() || '页面'
  return {
    id: shortId,
    conversationId: `ui-tuner-${shortId}`,
    title: `微调画布 · ${element}`.slice(0, 42),
    projectId: input.projectId,
    channelId: input.channelId,
    createdAt: now,
    updatedAt: now,
    sourceSessionId: input.source?.id,
    sourceSummary: input.source?.sourceSummary || input.memory.stableSummary,
    selectedElementName: element,
    status: 'draft',
  }
}

export function updateUiTunerProjectSession(
  record: UiTunerProjectSessionRecord,
  patch: Partial<UiTunerProjectSessionRecord>,
): UiTunerProjectSessionRecord {
  return {
    ...record,
    ...patch,
    updatedAt: new Date().toISOString(),
  }
}

export function buildUiTunerProjectTaskContent({
  pack,
  intent,
  memory,
  session,
  mode,
}: BuildTaskContentInput) {
  const selected = pack.selectedElement
  const target = selected
    ? `${selected.name} (${selected.kind}, ${selected.rect.x},${selected.rect.y} ${selected.rect.width}x${selected.rect.height})`
    : pack.screen.canvasName
  const sourceSummary = session.sourceSummary || memory.stableSummary
  return [
    '# 微调画布专属 Codex CLI 会话',
    '',
    `会话模式：${mode === 'fork' ? '从最新稳定摘要分叉' : '继续当前 ui-tuner 会话'}`,
    `目标元素：${target}`,
    `用户意图：${intent.trim() || '继续优化微调画布和 APK UI 标准闭环。'}`,
    '',
    '## 固定工作边界',
    '- 这是 elon 自项目内的项目 AI 开发任务，不是普通聊天。',
    '- 优先读取并修改 `/pc/ui-tuner` 相关源码、Android 源码映射和 `.elon/ui-*` 标准配置。',
    '- 当当前功能无法满足需求时，可以升级微调画布自身源码，再用本任务验证。',
    '- 不要丢失用户关于 UI 标准、过滤规则、源码回写和真机验收的长期目标。',
    '',
    '## 本模块长期记忆',
    `稳定摘要：${memory.stableSummary}`,
    `来源摘要：${sourceSummary}`,
    `已接受决策：${memory.acceptedDecisions.join('；') || '暂无'}`,
    `偏好标准：${memory.preferredStandards.join('；') || '暂无'}`,
    `未决问题：${memory.openQuestions.join('；') || '暂无'}`,
    '',
    '## 必读源码范围',
    '- pc-frontend/src/features/ui-tuner/',
    '- pc-frontend/src/features/conversation/useProjectStore.ts',
    '- pc-frontend/src/features/dev/SidecarTerminalPanel.tsx',
    '- server/src/project_space/channel_ai.rs',
    '- Android layout/values 中与 context pack 的 resourceId/source 文件相关的源码',
    '',
    '## 完成定义',
    '- 能说明用户点中的 XML 节点如何映射到源码。',
    '- 能把可复用设计沉淀到 JSON 标准配置，而不是 Markdown。',
    '- 若需要改功能，直接修改项目源码并运行验证。',
    '- 输出构建/真机重采集/页面验证结果，并更新 ui-tuner 模块记忆建议。',
    '',
    '## Context pack JSON',
    '```json',
    JSON.stringify(pack, null, 2),
    '```',
  ].join('\n')
}

function createDefaultModuleMemory(): UiTunerModuleMemory {
  return {
    version: 1,
    module: 'ui-tuner',
    updatedAt: new Date().toISOString(),
    stableSummary: '微调画布的长期目标是：真机截图/XML 采集、清晰过滤、点击元素、项目 Codex CLI 读源码并改源码、沉淀 UI 标准配置、重新真机验收。',
    acceptedDecisions: [
      'Codex 会话必须归属于自项目项目会话，而不是普通个人聊天。',
      '每个任务都携带当前选中 APK 元素的 context pack。',
      '可复用 UI 标准必须保存为 JSON 配置。',
    ],
    openQuestions: [
      '低置信度 XML 节点需要用户确认后才能写入全局标准。',
    ],
    preferredStandards: [
      '优先保存 tokens、components、screens 三层标准。',
      '产品模式默认隐藏结构容器、重复边界和非目标包节点。',
    ],
  }
}

function normalizeMemory(memory: UiTunerModuleMemory): UiTunerModuleMemory {
  return {
    version: 1,
    module: 'ui-tuner',
    updatedAt: memory.updatedAt || new Date().toISOString(),
    stableSummary: memory.stableSummary || createDefaultModuleMemory().stableSummary,
    acceptedDecisions: uniqueCompact(memory.acceptedDecisions),
    openQuestions: uniqueCompact(memory.openQuestions),
    preferredStandards: uniqueCompact(memory.preferredStandards),
  }
}

function uniqueCompact(values: string[] = []) {
  return Array.from(new Set(values.map((value) => value.trim()).filter(Boolean))).slice(0, 12)
}
