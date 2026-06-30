import { clean } from '../../lib/utils'
import { friendlyModelName, providerGroupTitle } from './modelUtils'
import type { AgentOption } from './types'

export interface ModelOptionGroup {
  key: string
  provider: string
  label: string
  subtitle: string
  options: AgentOption[]
  selectedOption: AgentOption | null
  primaryOption: AgentOption
}

const EFFORT_ORDER = new Map([
  ['minimal', 0],
  ['low', 1],
  ['medium', 2],
  ['high', 3],
  ['xhigh', 4],
  ['max', 4],
])

const PRIMARY_EFFORTS = ['high', 'xhigh', 'max', 'medium', 'low', 'minimal', '']

export function groupModelOptions(
  options: AgentOption[],
  selectedAgent: string,
): ModelOptionGroup[] {
  const groups = new Map<
    string,
    {
      key: string
      provider: string
      label: string
      options: AgentOption[]
      selectedOption: AgentOption | null
    }
  >()

  for (const option of options) {
    const key = optionGroupKey(option)
    const existing = groups.get(key)
    if (existing) {
      existing.options.push(option)
      if (option.agentName === selectedAgent) existing.selectedOption = option
      continue
    }

    groups.set(key, {
      key,
      provider: option.provider,
      label: baseModelLabel(option),
      options: [option],
      selectedOption: option.agentName === selectedAgent ? option : null,
    })
  }

  return Array.from(groups.values()).map((group) => {
    const sortedOptions = sortByEffort(group.options)
    const selectedOption =
      sortedOptions.find((option) => option.agentName === selectedAgent) ?? group.selectedOption
    const primaryOption = selectedOption ?? pickPrimaryOption(sortedOptions)

    return {
      key: group.key,
      provider: group.provider,
      label: group.label,
      subtitle: groupSubtitle(sortedOptions, selectedOption, group.provider),
      options: sortedOptions,
      selectedOption,
      primaryOption,
    }
  })
}

export function effortDisplayName(value: string): string {
  const effort = normalizeEffort(value)
  if (effort === 'minimal') return 'Minimal'
  if (effort === 'low') return 'Low'
  if (effort === 'medium') return 'Medium'
  if (effort === 'high') return 'High'
  if (effort === 'xhigh' || effort === 'max') return 'Max'
  if (!effort) return 'Auto'
  return effort.charAt(0).toUpperCase() + effort.slice(1)
}

export function normalizeEffort(value: string): string {
  return clean(value).toLowerCase().replace(/\s+/g, '_')
}

function optionGroupKey(option: AgentOption): string {
  const provider = clean(option.provider).toLowerCase() || 'default'
  const backend = clean(option.backend).toLowerCase() || provider
  const modelId = clean(option.modelId).toLowerCase()

  if (modelId && modelId !== 'default') {
    return `${provider}:${backend}:model:${modelId}:${baseModelLabel(option).toLowerCase()}`
  }

  const fallback = clean(option.agentName) || baseModelLabel(option)
  return `${provider}:${backend}:agent:${fallback.toLowerCase()}`
}

function baseModelLabel(option: AgentOption): string {
  const modelId = clean(option.modelId)
  if (modelId && modelId.toLowerCase() !== 'default') return friendlyModelName(modelId)

  const label = stripRunMeta(option.label)
  if (label) return label

  return providerGroupTitle(option.provider)
}

function stripRunMeta(value: string): string {
  const parts = clean(value)
    .split(' · ')
    .map((part) => part.trim())
    .filter(Boolean)
  const baseParts = parts.filter((part) => !/^(推理|输出|摘要)\s+/i.test(part))
  return baseParts[0] ?? parts[0] ?? ''
}

function groupSubtitle(
  options: AgentOption[],
  selectedOption: AgentOption | null,
  provider: string,
): string {
  if (options.length > 1) {
    const current = selectedOption?.reasoningEffort
      ? `当前 ${effortDisplayName(selectedOption.reasoningEffort)} · `
      : ''
    return `${current}共 ${options.length} 个推理档位`
  }

  const option = options[0]
  const details = [
    option.reasoningEffort ? `推理 ${effortDisplayName(option.reasoningEffort)}` : '',
    option.verbosity ? `输出 ${option.verbosity}` : '',
    option.reasoningSummary ? `摘要 ${option.reasoningSummary}` : '',
  ].filter(Boolean)

  return details.join(' · ') || option.subtitle || providerGroupTitle(provider)
}

function pickPrimaryOption(options: AgentOption[]): AgentOption {
  for (const effort of PRIMARY_EFFORTS) {
    const found = options.find((option) => normalizeEffort(option.reasoningEffort) === effort)
    if (found) return found
  }
  return options[0]
}

function sortByEffort(options: AgentOption[]): AgentOption[] {
  return [...options].sort((a, b) => {
    const effortA = normalizeEffort(a.reasoningEffort)
    const effortB = normalizeEffort(b.reasoningEffort)
    const orderA = EFFORT_ORDER.get(effortA) ?? 99
    const orderB = EFFORT_ORDER.get(effortB) ?? 99
    if (orderA !== orderB) return orderA - orderB
    return a.label.localeCompare(b.label, 'zh-CN')
  })
}
