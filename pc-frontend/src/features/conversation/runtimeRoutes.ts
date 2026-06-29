export type RuntimeRoute = 'auto' | 'route_a' | 'route_b' | 'route_c' | 'route_c2' | 'route_c3'

export interface RuntimeRouteOption {
  value: RuntimeRoute
  code: string
  shortLabel: string
  title: string
  subtitle: string
  description: string
  configHref?: string
  configLabel?: string
}

export interface RuntimeRouteGroup {
  title: string
  description: string
  options: RuntimeRouteOption[]
}

export const RUNTIME_ROUTE_STORAGE_KEY = 'elon_pc_project_runtime_route'

export const FIRST_STAGE_RUNTIME_ROUTES: RuntimeRouteOption[] = [
  {
    value: 'auto',
    code: 'Auto',
    shortLabel: '自动路线',
    title: '自动选择',
    subtitle: '按项目绑定 PC 节点能力选择',
    description: '优先使用项目节点 CLI；不可用时按 API Runtime、服务器 API key Runtime 兜底。',
  },
  {
    value: 'route_a',
    code: 'Route A',
    shortLabel: '本机 CLI',
    title: '本机 CLI',
    subtitle: 'Copilot / Codex 在这台电脑上执行',
    description: '使用本机已登录的 Copilot、Codex、Claude 或 Gemini CLI，模型调用、文件读写和命令都发生在本机。',
    configHref: '/node?route=route_a',
    configLabel: '配置本机 CLI',
  },
  {
    value: 'route_b',
    code: 'Route B',
    shortLabel: '自带 API key',
    title: '自带 API key + 本机 harness',
    subtitle: '模型 key 属于用户，工具循环在本机',
    description: '用用户自己配置的 OpenAI-compatible API key 调模型，本机 PC harness 负责读写文件、运行命令和审批。',
    configHref: '/node?route=route_b',
    configLabel: '配置 API Runtime',
  },
  {
    value: 'route_c',
    code: 'Route C.1',
    shortLabel: '平台 API key',
    title: '服务器 API key + 本机 PC harness',
    subtitle: '平台出模型，本机执行工具',
    description: '模型调用走一龙服务器 API key；本机 PC harness 仍负责项目文件、命令执行、审批和审计。',
  },
]

export const SECOND_STAGE_RUNTIME_ROUTES: RuntimeRouteOption[] = [
  {
    value: 'route_c2',
    code: 'Route C.2',
    shortLabel: '远程 API',
    title: '远程 PC 节点 API Runtime',
    subtitle: '服务器分配远程 PC 节点，模型 key 在该节点 Runtime 中使用',
    description: '项目工作区、工具执行、审批和审计都在远程 PC harness 上运行；要求目标节点已配置 API Runtime。',
    configHref: '/node?route=route_c2',
    configLabel: '配置远程 API 节点',
  },
  {
    value: 'route_c3',
    code: 'Route C.3',
    shortLabel: '远程 CLI',
    title: '远程 PC 节点 CLI',
    subtitle: '服务器分配远程 PC 节点，使用该节点上的 Copilot / Codex CLI',
    description: '项目工作区和 CLI 会话都在远程 PC 节点上隔离执行；要求目标节点 CLI 登录和探测通过。',
    configHref: '/node?route=route_c3',
    configLabel: '配置远程 CLI 节点',
  },
]

export const RUNTIME_ROUTE_OPTIONS: RuntimeRouteOption[] = [
  ...FIRST_STAGE_RUNTIME_ROUTES,
  ...SECOND_STAGE_RUNTIME_ROUTES,
]

export const ACTIVE_RUNTIME_ROUTE_GROUPS: RuntimeRouteGroup[] = [
  {
    title: '第一阶段：本机 / 平台兜底',
    description: '适合项目工作区在自己的 PC 节点上，或让服务器模型兜底。',
    options: FIRST_STAGE_RUNTIME_ROUTES,
  },
  {
    title: '第二阶段：远程 PC 节点',
    description: '适合使用服务器节点大厅里可授权、可计费、可审计的别人 PC 节点。',
    options: SECOND_STAGE_RUNTIME_ROUTES,
  },
]

export function normalizeRuntimeRoute(value: unknown): RuntimeRoute {
  return RUNTIME_ROUTE_OPTIONS.some((item) => item.value === value)
    ? value as RuntimeRoute
    : 'auto'
}

export function runtimeRouteOption(value: RuntimeRoute): RuntimeRouteOption {
  return RUNTIME_ROUTE_OPTIONS.find((item) => item.value === value) ?? RUNTIME_ROUTE_OPTIONS[0]
}

export function runtimeRouteDescription(value: RuntimeRoute): string {
  return runtimeRouteOption(value).description
}
