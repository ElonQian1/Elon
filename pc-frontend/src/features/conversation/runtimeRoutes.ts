export type RuntimeRoute = 'auto' | 'route_a' | 'route_b' | 'route_c'

export interface RuntimeRouteOption {
  value: RuntimeRoute
  label: string
  description: string
}

export const RUNTIME_ROUTE_STORAGE_KEY = 'elon_pc_project_runtime_route'

export const RUNTIME_ROUTE_OPTIONS: RuntimeRouteOption[] = [
  { value: 'auto', label: 'Auto', description: '按节点就绪状态自动选择' },
  { value: 'route_a', label: 'A CLI', description: '本机 Codex/Copilot/Claude/Gemini CLI' },
  { value: 'route_b', label: 'B API', description: '自带 API key + 本机 harness' },
  { value: 'route_c', label: 'C.1 云端', description: '服务器 API key + PC harness' },
]

export function normalizeRuntimeRoute(value: unknown): RuntimeRoute {
  return RUNTIME_ROUTE_OPTIONS.some((item) => item.value === value)
    ? value as RuntimeRoute
    : 'auto'
}

export function runtimeRouteDescription(value: RuntimeRoute): string {
  return RUNTIME_ROUTE_OPTIONS.find((item) => item.value === value)?.description ?? RUNTIME_ROUTE_OPTIONS[0].description
}
