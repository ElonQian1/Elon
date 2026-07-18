export const LOCAL_TASK_PLACEHOLDER_TITLE = '本机离线任务'
export const LOCAL_TASK_FALLBACK_TITLE = '本机 Codex 任务'

const MAX_TITLE_CHARS = 34
const SECTION_STOPS = ['桌面监督分析结论', '用户可见目标', '实施要求', '非目标']

export function readableTaskTitle(input: unknown, fallback = LOCAL_TASK_FALLBACK_TITLE): string {
  const prompt = typeof input === 'string' ? input : ''
  const scoped = originalRequirementBody(userRequestBody(prompt))
  const cleaned = cleanLines(scoped)
  const preferred = preferredGoalClause(cleaned) ?? cleaned
  const candidate = normalizeCandidate(preferred)
  const stableFallback = fallback.trim() || LOCAL_TASK_FALLBACK_TITLE
  if (!candidate || isMachineLine(candidate)) return truncateTitle(stableFallback)
  return truncateTitle(candidate)
}

function userRequestBody(prompt: string): string {
  const open = prompt.indexOf('<user-request>')
  if (open >= 0) {
    const body = prompt.slice(open + '<user-request>'.length)
    const close = body.indexOf('</user-request>')
    return close >= 0 ? body.slice(0, close) : body
  }
  const executorClose = prompt.indexOf('</elon-pc-executor>')
  return executorClose >= 0
    ? prompt.slice(executorClose + '</elon-pc-executor>'.length)
    : prompt
}

function originalRequirementBody(prompt: string): string {
  const marker = prompt.indexOf('用户原始需求')
  if (marker < 0) return prompt
  const body = prompt.slice(marker + '用户原始需求'.length).replace(/^[\s:：]+/, '')
  const stops = SECTION_STOPS
    .map((section) => body.indexOf(section))
    .filter((index) => index >= 0)
  return stops.length ? body.slice(0, Math.min(...stops)) : body
}

function cleanLines(input: string): string {
  return input
    .split(/\r?\n/)
    .map((line) => line.replace(/codex:\/\/[A-Za-z0-9_./:?=&%#-]+/gi, ' ').trim())
    .filter((line) => line && !isMachineLine(line))
    .map(stripTitleEdges)
    .filter(Boolean)
    .join(' ')
}

function stripTitleEdges(input: string): string {
  return input
    .trim()
    .replace(/^\d+[.、)]\s*/, '')
    .replace(/^[\s“”‘’"'`#*\-•<>]+|[\s“”‘’"'`#*\-•<>]+$/g, '')
    .trim()
}

function isMachineLine(input: string): boolean {
  const value = input.trim()
  const lower = value.toLowerCase()
  return lower.startsWith('<elon-pc-executor')
    || lower.startsWith('</elon-pc-executor')
    || lower.startsWith('<user-request')
    || lower.startsWith('</user-request')
    || lower.startsWith('supervision_contract=')
    || value.startsWith('你是由一龙 PC 本机节点启动的执行者')
    || value.startsWith('直接在当前项目完成任务')
    || value.startsWith('读取并遵守项目 AGENTS.md')
    || value.startsWith('桌面监督者会独立检查')
    || value.startsWith('非阻塞的平台改进先记录')
    || value.startsWith('最终回复分别说明')
    || value.startsWith('桌面监督分析结论')
    || value.startsWith('用户可见目标')
    || value.startsWith('实施要求')
    || value.startsWith('非目标')
    || value.startsWith('节点更新后自动恢复原任务')
    || value.startsWith('请恢复原任务')
    || value.startsWith('请继续完成上述任务')
    || value.startsWith('继续完成原任务并运行统一收尾')
}

function preferredGoalClause(input: string): string | null {
  for (const cue of ['用户希望的是', '用户希望的就是', '希望的是']) {
    const start = input.indexOf(cue)
    if (start >= 0) {
      return firstSentence(input.slice(start + cue.length).replace(/^[\s,，:：]+/, ''))
    }
  }
  return null
}

function normalizeCandidate(input: string): string {
  let value = stripTitleEdges(firstSentence(input).replace(/\s+/g, ' '))
  for (const prefix of ['请你', '请', '麻烦你', '麻烦', '帮我', '希望你', '需要你']) {
    if (value.startsWith(prefix)) {
      value = stripTitleEdges(value.slice(prefix.length))
      break
    }
  }
  if (value.startsWith('有') && Array.from(value).length > 2) value = value.slice(1)
  return value.replace(/^[\s,，;；:：]+|[\s,，;；:：]+$/g, '')
}

function firstSentence(input: string): string {
  const end = input.search(/[。！？!?]/)
  return end >= 0 ? input.slice(0, end) : input
}

function truncateTitle(input: string): string {
  const chars = Array.from(input)
  return chars.length <= MAX_TITLE_CHARS
    ? input
    : `${chars.slice(0, MAX_TITLE_CHARS - 1).join('')}…`
}
