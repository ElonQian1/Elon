import { clean } from '../../lib/utils'
import { runtimeRouteOption } from '../conversation/runtimeRoutes'
import type { RuntimeRoute } from '../conversation/runtimeRoutes'
import { shortButtonLabel } from './modelUtils'
import type { AgentOption } from './types'

const CLI_PROVIDERS = new Set(['codex', 'copilot', 'github', 'claude', 'gemini'])

export interface RouteModelButtonCopy {
  source: string
  detail: string
  title: string
}

export interface RouteModelEmptyState {
  title: string
  body: string
  actionHref?: string
  actionLabel?: string
}

export function isCliAgentOption(option: AgentOption): boolean {
  const backend = clean(option.backend).toLowerCase()
  if (backend === 'cli') return true
  if (backend === 'api' || backend === 'default') return false
  return CLI_PROVIDERS.has(clean(option.provider).toLowerCase())
}

function isPlatformAgentOption(option: AgentOption): boolean {
  const backend = clean(option.backend).toLowerCase()
  return !clean(option.agentName) || backend === 'api' || backend === 'default'
}

export function optionMatchesRuntimeRoute(option: AgentOption, route: RuntimeRoute): boolean {
  if (route === 'auto') return true
  if (route === 'route_a' || route === 'route_c3') return isCliAgentOption(option)
  if (route === 'route_c') return isPlatformAgentOption(option)
  return false
}

export function filterOptionsForRuntimeRoute(
  options: AgentOption[],
  route: RuntimeRoute,
): AgentOption[] {
  return options.filter((option) => optionMatchesRuntimeRoute(option, route))
}

export function selectedAgentForRuntimeRoute(
  selectedAgent: string,
  options: AgentOption[],
  route: RuntimeRoute,
): string {
  const normalized = clean(selectedAgent)
  if (!normalized) return ''
  if (route === 'auto') return normalized
  const option = options.find((item) => item.agentName === normalized)
  return option && optionMatchesRuntimeRoute(option, route) ? normalized : ''
}

function selectedOptionForRuntimeRoute(
  selectedAgent: string,
  options: AgentOption[],
  route: RuntimeRoute,
): AgentOption | null {
  const normalized = clean(selectedAgent)
  if (!normalized) return null
  const option = options.find((item) => item.agentName === normalized) ?? null
  return option && optionMatchesRuntimeRoute(option, route) ? option : null
}

export function routeModelEmptyState(route: RuntimeRoute): RouteModelEmptyState {
  const routeOption = runtimeRouteOption(route)
  if (route === 'route_b') {
    return {
      title: '模型在我的Key配置里填写',
      body: '这里不会再列平台模型。保存 API 地址、API key 和模型名后，项目会用你的 key 跑一龙 CLI 能力。',
      actionHref: routeOption.configHref,
      actionLabel: routeOption.configLabel ?? '配置我的Key',
    }
  }
  if (route === 'route_c2') {
    return {
      title: '模型由远程节点决定',
      body: '其他用户 PC 节点会使用它自己准备好的 API key 和模型配置。请先选择一个已就绪的远程节点。',
      actionHref: routeOption.configHref,
      actionLabel: routeOption.configLabel ?? '选择远程节点',
    }
  }
  if (route === 'route_a') {
    return {
      title: '还没有可用的本机AI',
      body: '这台电脑需要安装并登录 Codex、Copilot、Claude 或 Gemini，检测通过后才会出现可选 CLI。',
      actionHref: routeOption.configHref,
      actionLabel: routeOption.configLabel ?? '配置本机AI',
    }
  }
  if (route === 'route_c3') {
    return {
      title: '还没有可用的远程 Codex / Claude',
      body: '请在节点大厅选择已登录 Codex、Claude、Copilot 或 Gemini 的远程 PC 节点。',
      actionHref: routeOption.configHref,
      actionLabel: routeOption.configLabel ?? '选择远程节点',
    }
  }
  if (route === 'route_c') {
    return {
      title: '平台AI暂无可选模型',
      body: '平台默认模型未返回可选项。可以先切回自动选择，或稍后刷新模型列表。',
    }
  }
  return {
    title: '暂无可选模型',
    body: '当前账号还没有返回可用模型。请刷新，或检查服务器 agent 与 PC 节点配置。',
  }
}

export function routeModelButtonCopy(
  route: RuntimeRoute,
  selectedLabel: string,
  options: AgentOption[],
  selectedAgent: string,
): RouteModelButtonCopy {
  const source = runtimeRouteOption(route)
  const matchingOptions = filterOptionsForRuntimeRoute(options, route)
  const selectedOption = selectedOptionForRuntimeRoute(selectedAgent, options, route)

  let detail = ''
  if (route === 'route_b') detail = '配置模型'
  else if (route === 'route_c2') detail = '远程节点决定'
  else if (selectedOption) detail = shortButtonLabel(selectedOption.label)
  else if (route === 'auto') detail = shortButtonLabel(selectedLabel || '自动模型')
  else if (matchingOptions.length > 0) detail = route === 'route_c' ? '平台默认' : '自动选择'
  else if (route === 'route_a') detail = '去配置'
  else if (route === 'route_c3') detail = '选远程节点'
  else detail = '暂无模型'

  const cleanDetail = shortButtonLabel(detail)
  return {
    source: source.shortLabel,
    detail: cleanDetail,
    title: `AI来源：${source.title}；模型：${detail}`,
  }
}
