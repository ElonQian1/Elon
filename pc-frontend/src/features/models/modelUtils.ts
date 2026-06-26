/** 纯业务逻辑，对应旧 pc_app_models.js 中的 helper 函数 */
import { clean } from '../../lib/utils'
import type { AgentOption, AgentConfigResponse, RawAgentItem } from './types'

export function providerGroupTitle(provider: string): string {
  const p = clean(provider).toLowerCase()
  if (!p || p === 'default') return '默认'
  if (p === 'codex') return 'Codex CLI'
  if (p === 'copilot') return 'GitHub Copilot'
  if (p === 'openai') return 'OpenAI'
  if (p === 'anthropic' || p === 'claude') return 'Claude'
  if (p === 'google' || p === 'gemini') return 'Gemini'
  if (p === 'api') return 'API 模型'
  return p.toUpperCase()
}

export function friendlyModelName(value: string): string {
  const lower = clean(value).toLowerCase()
  if (lower === 'gpt-5.5') return 'GPT-5.5'
  if (lower === 'gpt-5.4') return 'GPT-5.4'
  if (lower === 'gpt-5.4-mini') return 'GPT-5.4 mini'
  if (lower === 'gpt-5.3-codex-spark') return 'GPT-5.3 Codex Spark'
  if (lower === 'gpt-5.3-codex') return 'GPT-5.3 Codex'
  if (lower === 'gpt-5.2') return 'GPT-5.2'
  if (lower === 'gpt-5') return 'GPT-5'
  return clean(value)
}

export function shortButtonLabel(label: string): string {
  const v = clean(label).replace(/^服务器默认$/, '默认')
  if (!v || v === 'AI') return 'AI'
  if (v.includes('GPT-5.5')) return 'GPT-5.5'
  if (v.includes('GPT-5.4 mini')) return '5.4 mini'
  if (v.includes('GPT-5.4')) return 'GPT-5.4'
  if (v.includes('GPT-5.3')) return 'GPT-5.3'
  if (v.includes('Claude')) return 'Claude'
  if (v.includes('Gemini')) return 'Gemini'
  if (v.includes('Copilot')) return 'Copilot'
  if (v.includes('Codex')) return 'Codex'
  return v.length > 8 ? v.slice(0, 8) : v
}

function codexSubtitle(
  modelId: string,
  reasoningEffort: string,
  reasoningSummary: string,
  verbosity: string,
): string {
  const parts: string[] = []
  if (modelId && modelId.toLowerCase() !== 'default') parts.push(`模型 ${friendlyModelName(modelId)}`)
  if (reasoningEffort) parts.push(`推理 ${reasoningEffort}`)
  if (verbosity) parts.push(`输出 ${verbosity}`)
  if (reasoningSummary) parts.push(`摘要 ${reasoningSummary}`)
  return parts.join(' · ')
}

function withCodexRunMeta(
  label: string,
  provider: string,
  reasoningEffort: string,
  verbosity: string,
): string {
  if (provider !== 'codex') return label
  const parts = [label || 'Codex 默认']
  if (reasoningEffort) parts.push(`推理 ${reasoningEffort}`)
  if (verbosity) parts.push(`输出 ${verbosity}`)
  return parts.join(' · ')
}

function normalizeAgentOption(item: RawAgentItem): AgentOption | null {
  const agentName = clean(item.name)
  if (!agentName) return null
  const provider = clean(item.provider ?? item.backend ?? 'api').toLowerCase()
  const modelId = clean(item.model)
  const displayModel = clean(item.display_model)
  const rawLabel = clean(item.label)
  const reasoningEffort = clean(item.reasoning_effort)
  const reasoningSummary = clean(item.reasoning_summary)
  const verbosity = clean(item.verbosity)
  const baseLabel = displayModel || (rawLabel
    ? rawLabel
    : modelId && modelId.toLowerCase() !== 'default'
      ? friendlyModelName(modelId)
      : providerGroupTitle(provider))
  const label = displayModel
    ? displayModel
    : withCodexRunMeta(baseLabel, provider, reasoningEffort, verbosity)
  return {
    label,
    agentName,
    provider,
    backend: clean(item.backend),
    modelId,
    reasoningEffort,
    reasoningSummary,
    verbosity,
    subtitle:
      codexSubtitle(modelId, reasoningEffort, reasoningSummary, verbosity) ||
      clean(item.api_base) ||
      providerGroupTitle(provider),
  }
}

export function buildOptions(data: AgentConfigResponse): AgentOption[] {
  const options: AgentOption[] = []
  if (!data.codex_cli_only) {
    options.push({
      label: '服务器默认',
      agentName: '',
      provider: 'default',
      backend: 'default',
      modelId: '',
      reasoningEffort: '',
      reasoningSummary: '',
      verbosity: '',
      subtitle: '使用服务器当前默认模型',
    })
  }
  const agents = Array.isArray(data.available_agents) ? data.available_agents : []
  for (const item of agents) {
    const opt = normalizeAgentOption(item)
    if (opt) options.push(opt)
  }
  return options
}

export function resolveSelection(
  data: AgentConfigResponse,
  options: AgentOption[],
  cachedAgent: string,
): { selectedAgent: string; label: string } {
  const configured = clean(data.config?.use_agent)
  const hasCustomConfig = !!(clean(data.config?.api_base) || clean(data.config?.model))
  const byAgent = (name: string) => options.find((o) => o.agentName === clean(name)) ?? null

  let selectedAgent = ''
  if (byAgent(configured)) selectedAgent = clean(configured)
  else if (byAgent(cachedAgent)) selectedAgent = clean(cachedAgent)
  else if (!data.codex_cli_only && byAgent(clean(data.default_agent))) selectedAgent = clean(data.default_agent)
  else if (!data.codex_cli_only && options[0]) selectedAgent = options[0].agentName

  const selectedOption = byAgent(selectedAgent)
  const label =
    hasCustomConfig && !selectedAgent
      ? '自定义模型'
      : selectedOption?.label ?? '服务器默认'

  return { selectedAgent, label }
}
