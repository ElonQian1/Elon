import { clean } from '../../lib/utils'
import type { TaskTone, ToolEvent } from './types'

export type ProcessCardKind = 'command' | 'file' | 'test' | 'search' | 'tool'

export interface ProcessCardChip {
  label: string
  tone?: TaskTone
}

export interface ProcessCard {
  kind: ProcessCardKind
  tone: TaskTone
  title: string
  subtitle: string
  bodyLabel: string
  body: string
  chips: ProcessCardChip[]
  monospace: boolean
  truncated: boolean
}

const BODY_LIMIT = 2400

export function processCardFromToolEvent(event: ToolEvent): ProcessCard | null {
  const type = clean(event.type)
  if (type !== 'tool_call' && type !== 'tool_result') return null

  const tool = clean(event.tool ?? 'tool')
  const isResult = type === 'tool_result'
  const failed = isResult && clean(event.status ?? '').toLowerCase() === 'error'
  const tone: TaskTone = failed ? 'failed' : isResult ? 'done' : 'running'
  if (tool === 'shell') return shellProcessCard(event, tone, isResult)
  if (tool === 'file_change') return fileProcessCard(event, tone, isResult)
  if (tool === 'web_search') return searchProcessCard(event, tone, isResult)
  return genericProcessCard(event, tone, isResult)
}

function shellProcessCard(event: ToolEvent, tone: TaskTone, isResult: boolean): ProcessCard {
  const validation = shellEventLooksLikeValidation(event)
  const command = clean(event.args?.command ?? '')
  const result = clean(event.result ?? '')
  const status = clean(event.status ?? '')
  const body = isResult ? result || '（命令没有输出）' : command || formatJson(event.args)
  return {
    kind: validation ? 'test' : 'command',
    tone,
    title: validation ? (isResult ? '测试/构建完成' : '运行测试/构建') : (isResult ? '命令完成' : '执行命令'),
    subtitle: isResult ? firstLine(result) || firstLine(command) || 'shell' : firstLine(command) || 'shell',
    bodyLabel: isResult ? '输出' : '命令',
    ...limitedBody(body),
    chips: [
      { label: 'shell' },
      validation ? { label: '测试/构建', tone: 'running' } : null,
      status ? { label: status, tone } : null,
      isResult ? { label: formatChars(result.length) } : null,
    ].filter(Boolean) as ProcessCardChip[],
    monospace: true,
  }
}

function fileProcessCard(event: ToolEvent, tone: TaskTone, isResult: boolean): ProcessCard {
  const files = fileTargets(event)
  const diffPreview = clean(event.diff?.preview ?? '')
  const result = clean(event.result ?? '')
  const argsPreview = fileArgsPreview(event)
  const body = diffPreview || result || argsPreview || '（没有文件详情）'
  return {
    kind: 'file',
    tone,
    title: isResult ? '文件修改完成' : '准备修改文件',
    subtitle: files.length ? files.slice(0, 3).join(', ') : 'file_change',
    bodyLabel: diffPreview ? 'Diff 预览' : isResult ? '结果' : '文件',
    ...limitedBody(body),
    chips: [
      { label: 'file_change' },
      files.length ? { label: `${files.length} 个文件`, tone: 'running' } : null,
      event.diff?.truncated ? { label: 'diff 已截断', tone: 'muted' } : null,
    ].filter(Boolean) as ProcessCardChip[],
    monospace: true,
  }
}

function searchProcessCard(event: ToolEvent, tone: TaskTone, isResult: boolean): ProcessCard {
  const query = clean(event.args?.query ?? event.args?.input ?? '')
  const result = clean(event.result ?? '')
  const body = isResult ? result || '（搜索没有返回摘要）' : query || formatJson(event.args)
  return {
    kind: 'search',
    tone,
    title: isResult ? '搜索完成' : '搜索网络',
    subtitle: firstLine(query || result) || 'web_search',
    bodyLabel: isResult ? '结果' : '查询',
    ...limitedBody(body),
    chips: [{ label: 'web_search' }, isResult ? { label: formatChars(result.length) } : null].filter(Boolean) as ProcessCardChip[],
    monospace: false,
  }
}

function genericProcessCard(event: ToolEvent, tone: TaskTone, isResult: boolean): ProcessCard {
  const tool = clean(event.tool ?? 'tool')
  const result = clean(event.result ?? '')
  const body = isResult ? result || '（工具没有输出）' : formatJson(event.args) || '（没有参数）'
  return {
    kind: 'tool',
    tone,
    title: `${isResult ? '完成' : '调用'} ${tool}`,
    subtitle: firstLine(body) || tool,
    bodyLabel: isResult ? '输出' : '参数',
    ...limitedBody(body),
    chips: [{ label: tool }, isResult ? { label: formatChars(result.length) } : null].filter(Boolean) as ProcessCardChip[],
    monospace: true,
  }
}

function shellEventLooksLikeValidation(event: ToolEvent): boolean {
  const command = clean(event.args?.command ?? '').toLowerCase()
  const result = clean(event.result ?? '').toLowerCase()
  return (
    /\b(cargo|npm|pnpm|yarn|bun|pytest|gradle|go|mvn|ruff|eslint|tsc)\b/.test(command)
    && /\b(test|check|build|clippy|lint|typecheck|assemble|verify)\b/.test(command)
  ) || result.includes('test result:')
    || result.includes('finished `test`')
    || result.includes('finished `dev`')
    || result.includes('npm run build')
    || result.includes('vite')
    || result.includes('cargo check')
    || result.includes('cargo test')
    || result.includes('build successful')
}

function fileTargets(event: ToolEvent): string[] {
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

function fileArgsPreview(event: ToolEvent): string {
  const files = fileTargets(event)
  if (files.length) return files.join('\n')
  return formatJson(event.args)
}

function limitedBody(value: string): Pick<ProcessCard, 'body' | 'truncated'> {
  const text = clean(value)
  if (text.length <= BODY_LIMIT) return { body: text, truncated: false }
  return { body: `${text.slice(0, BODY_LIMIT)}\n...`, truncated: true }
}

function firstLine(value: string): string {
  const line = clean(value).split(/\r?\n/).find(Boolean) ?? ''
  return line.length > 110 ? `${line.slice(0, 110)}...` : line
}

function formatJson(value: unknown): string {
  if (!value) return ''
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return clean(value)
  }
}

function formatChars(count: number): string {
  if (!Number.isFinite(count) || count <= 0) return '无输出'
  if (count < 1000) return `${count} 字`
  return `${Math.round(count / 100) / 10}k 字`
}
