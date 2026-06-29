export type RuntimeRoute = 'auto' | 'route_a' | 'route_b' | 'route_c'

export interface RuntimeRouteOption {
  value: RuntimeRoute
  code: string
  shortLabel: string
  title: string
  subtitle: string
  description: string
}

export interface RuntimeRouteGroup {
  title: string
  description: string
  options: RuntimeRouteOption[]
}

export interface FutureRuntimeRouteOption {
  key: string
  code: string
  title: string
  subtitle: string
  description: string
  stage: string
}

export const RUNTIME_ROUTE_STORAGE_KEY = 'elon_pc_project_runtime_route'

export const RUNTIME_ROUTE_OPTIONS: RuntimeRouteOption[] = [
  {
    value: 'auto',
    code: 'Auto',
    shortLabel: '自动路线',
    title: '自动选择',
    subtitle: '按当前 PC 节点能力选择',
    description: '优先使用本机 CLI；不可用时按本机 API Runtime、服务器 API key Runtime 兜底。',
  },
  {
    value: 'route_a',
    code: 'Route A',
    shortLabel: '本机 CLI',
    title: '本机 CLI',
    subtitle: 'Copilot / Codex 在这台电脑上执行',
    description: '使用本机已登录的 Copilot、Codex、Claude 或 Gemini CLI，模型调用、文件读写和命令都发生在本机。',
  },
  {
    value: 'route_b',
    code: 'Route B',
    shortLabel: '自带 API key',
    title: '自带 API key + 本机 harness',
    subtitle: '模型 key 属于用户，工具循环在本机',
    description: '用用户自己配置的 OpenAI-compatible API key 调模型，本机 PC harness 负责读写文件、运行命令和审批。',
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

export const ACTIVE_RUNTIME_ROUTE_GROUPS: RuntimeRouteGroup[] = [
  {
    title: '当前可用',
    description: '这几条路线已经接入项目 AI 任务。',
    options: RUNTIME_ROUTE_OPTIONS,
  },
]

export const FUTURE_RUNTIME_ROUTES: FutureRuntimeRouteOption[] = [
  {
    key: 'route_c2',
    code: 'Route C.2',
    title: '远程别人 PC 节点 + API key + PC harness',
    subtitle: '下一阶段：把项目派到可授权的远程 PC 节点执行',
    description: '需要先完成节点授权、项目隔离、密钥归属、计费结算和审计边界。',
    stage: '第二阶段',
  },
  {
    key: 'route_c3',
    code: 'Route C.3',
    title: '远程别人 PC 节点 CLI + PC harness',
    subtitle: '下一阶段后半：使用远程节点上的 Copilot/Codex CLI',
    description: '需要在 C.2 的远程节点安全边界上，再补远程 CLI 登录状态、命令审批、会话隔离和收益结算。',
    stage: '第二阶段',
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
