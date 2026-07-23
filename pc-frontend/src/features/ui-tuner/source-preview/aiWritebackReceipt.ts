import type { CrossPlatformTarget } from './crossPlatformWritebackReceipt'

export interface AiPlatformWritebackResult {
  status: 'SAVED' | 'FAILED'
  changedFiles: string[]
  sourceRevision: string
  error?: string
}

export interface AiWritebackReceipt {
  schemaVersion: 1
  changedFiles: string[]
  sourceHash: string
  sourceRevisionBefore: string
  sourceRevision: string
  targetPlatforms: CrossPlatformTarget[]
  platformResults: Partial<Record<CrossPlatformTarget, AiPlatformWritebackResult>>
}

const AI_RECEIPT_MARKER = 'ELON_UI_WRITEBACK_RECEIPT_V1'

export function parseAiWritebackReceipt(text: string): AiWritebackReceipt | null {
  const marker = text.lastIndexOf(AI_RECEIPT_MARKER)
  if (marker < 0) return null
  const jsonText = firstJsonObject(text.slice(marker + AI_RECEIPT_MARKER.length))
  if (!jsonText) return null
  try {
    return normalizeAiWritebackReceipt(JSON.parse(jsonText))
  } catch {
    return null
  }
}

export function aiWritebackReceiptInstruction(): string {
  return [
    '最终回复必须包含一行 ELON_UI_WRITEBACK_RECEIPT_V1，紧接一个 JSON 对象。',
    'JSON 必须包含 schemaVersion=1、changedFiles、sourceHash、sourceRevisionBefore、sourceRevision、targetPlatforms、platformResults。',
    'platformResults 仅允许 pwa/apk；每端必须给出 SAVED 或 FAILED、changedFiles、sourceRevision，失败时给 error。',
    'sourceHash/sourceRevision 必须来自修改后的真实文件或工作区；缺少该机器回执时，界面不会显示源码已保存或任务完成。',
    '不要把 build verified 写进 AI 回执；PWA 重载和 APK 真机构建由 UI 工作台独立验收。',
  ].join('\n')
}

function normalizeAiWritebackReceipt(value: unknown): AiWritebackReceipt | null {
  if (!value || typeof value !== 'object') return null
  const input = value as Partial<AiWritebackReceipt>
  if (input.schemaVersion !== 1) return null
  const changedFiles = safeFiles(input.changedFiles)
  const targetPlatforms = safePlatforms(input.targetPlatforms)
  const sourceHash = safeRevision(input.sourceHash)
  const sourceRevisionBefore = safeRevision(input.sourceRevisionBefore)
  const sourceRevision = safeRevision(input.sourceRevision)
  if (!changedFiles.length || !targetPlatforms.length || !sourceHash || !sourceRevisionBefore || !sourceRevision) return null
  const platformResults: Partial<Record<CrossPlatformTarget, AiPlatformWritebackResult>> = {}
  for (const platform of targetPlatforms) {
    const raw = input.platformResults?.[platform]
    const status = raw?.status
    const files = safeFiles(raw?.changedFiles)
    const revision = safeRevision(raw?.sourceRevision)
    if (!raw || !['SAVED', 'FAILED'].includes(String(status)) || !revision) return null
    if (status === 'SAVED' && !files.length) return null
    platformResults[platform] = {
      status: status as AiPlatformWritebackResult['status'],
      changedFiles: files,
      sourceRevision: revision,
      error: safeText(raw.error, 2_000),
    }
  }
  const declaredFiles = new Set(Object.values(platformResults).flatMap((result) => result?.changedFiles ?? []))
  if (changedFiles.some((file) => !declaredFiles.has(file))) return null
  return {
    schemaVersion: 1,
    changedFiles,
    sourceHash,
    sourceRevisionBefore,
    sourceRevision,
    targetPlatforms,
    platformResults,
  }
}

function safePlatforms(value: unknown): CrossPlatformTarget[] {
  if (!Array.isArray(value)) return []
  const result = [...new Set(value.map(String).map((item) => item.toLowerCase()))]
  return result.length && result.every((item) => item === 'pwa' || item === 'apk')
    ? result as CrossPlatformTarget[]
    : []
}

function safeFiles(value: unknown): string[] {
  if (!Array.isArray(value) || value.length > 256) return []
  const result = [...new Set(value.map(String).map((item) => item.trim().replace(/\\/g, '/')))]
  return result.every((item) => (
    item.length > 0
    && item.length <= 1_000
    && !item.startsWith('/')
    && !/^[a-z]:\//i.test(item)
    && item.split('/').every((part) => part && part !== '.' && part !== '..')
  )) ? result.sort() : []
}

function safeRevision(value: unknown): string {
  const revision = String(value ?? '').trim()
  return revision.length > 0 && revision.length <= 256 ? revision : ''
}

function safeText(value: unknown, maxLength: number): string | undefined {
  const text = String(value ?? '').trim().slice(0, maxLength)
  return text || undefined
}

function firstJsonObject(text: string): string | null {
  const start = text.indexOf('{')
  if (start < 0) return null
  let depth = 0
  let quoted = false
  let escaped = false
  for (let index = start; index < text.length; index += 1) {
    const character = text[index]
    if (quoted) {
      if (escaped) escaped = false
      else if (character === '\\') escaped = true
      else if (character === '"') quoted = false
      continue
    }
    if (character === '"') quoted = true
    else if (character === '{') depth += 1
    else if (character === '}' && --depth === 0) return text.slice(start, index + 1)
  }
  return null
}
