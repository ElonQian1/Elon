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
export const RUNTIME_ROUTE_DEFAULT_VERSION_KEY = 'elon_pc_project_runtime_route_default_v2'
const LEGACY_PLATFORM_DEFAULT_VERSION = 'platform-ai-default-20260630'
const LEGACY_PROJECT_LOCAL_DEFAULT_VERSION = 'project-local-codex-default-20260702'
export const RUNTIME_ROUTE_DEFAULT_VERSION = 'auto-default-20260703'
export const DEFAULT_RUNTIME_ROUTE: RuntimeRoute = 'auto'
export const PROJECT_RUNTIME_ROUTE_DEFAULT_VERSION = 'project-auto-default-20260703'
export const DEFAULT_PROJECT_RUNTIME_ROUTE: RuntimeRoute = 'auto'

export const FIRST_STAGE_RUNTIME_ROUTES: RuntimeRouteOption[] = [
  {
    value: 'auto',
    code: '自动',
    shortLabel: '自动选择',
    title: '自动选择',
    subtitle: '一龙按当前项目选择合适的 AI',
    description: '优先使用项目电脑上已经准备好的 AI；不可用时自动切到平台 AI，减少用户手动判断。',
  },
  {
    value: 'route_a',
    code: '本机AI',
    shortLabel: '本机AI',
    title: '本机AI + 一龙 CLI',
    subtitle: '使用这台电脑上已经登录的 Codex / Copilot',
    description: '适合已经在自己电脑上装好并登录 AI 工具的用户；项目文件和执行过程都在自己的电脑上完成。',
    configHref: '/node?route=route_a',
    configLabel: '配置本机AI',
  },
  {
    value: 'route_b',
    code: '我的Key',
    shortLabel: '我的Key',
    title: '本机 API key + 一龙 CLI',
    subtitle: '使用自己的模型 key，仍然在本机完成项目操作',
    description: '适合想用自己 API key 控制模型成本或模型来源的用户；一龙负责在这台电脑上执行项目开发动作。',
    configHref: '/node?route=route_b',
    configLabel: '配置本机 API key',
  },
  {
    value: 'route_c',
    code: '平台AI',
    shortLabel: '平台AI',
    title: '平台AI + 一龙 CLI',
    subtitle: '不用自己准备 key，由一龙平台提供模型',
    description: '适合不想配置模型 key 的用户；一龙平台提供 AI，项目开发动作仍在项目绑定的电脑上完成。',
  },
]

export const SECOND_STAGE_RUNTIME_ROUTES: RuntimeRouteOption[] = [
  {
    value: 'route_c2',
    code: '远程AI',
    shortLabel: '远程AI',
    title: '其他用户 PC 节点 + 一龙 CLI',
    subtitle: '把项目交给可用的远程电脑执行',
    description: '适合自己电脑不方便运行时使用；目标电脑需要在线、有项目容量，并已准备好可用的一龙 AI 能力。',
    configHref: '/node?route=route_c2',
    configLabel: '配置远程一龙 AI 节点',
  },
  {
    value: 'route_c3',
    code: '远程Codex',
    shortLabel: '远程Codex',
    title: '其他用户 PC 节点 + Codex / Claude',
    subtitle: '使用远程电脑上已经登录的 Codex、Claude 或 Copilot',
    description: '适合使用别人电脑上的专业 AI 工具执行项目；目标电脑需要在线，并且对应 AI 工具已经登录可用。',
    configHref: '/node?route=route_c3',
    configLabel: '配置远程 Codex / Claude 节点',
  },
]

export const RUNTIME_ROUTE_OPTIONS: RuntimeRouteOption[] = [
  ...FIRST_STAGE_RUNTIME_ROUTES,
  ...SECOND_STAGE_RUNTIME_ROUTES,
]

export const ACTIVE_RUNTIME_ROUTE_GROUPS: RuntimeRouteGroup[] = [
  {
    title: '自己的电脑',
    description: '适合用自己的电脑开发，或让一龙平台提供 AI。',
    options: FIRST_STAGE_RUNTIME_ROUTES,
  },
  {
    title: '其他用户的电脑',
    description: '适合使用节点大厅里在线、可授权、可计费的远程电脑。',
    options: SECOND_STAGE_RUNTIME_ROUTES,
  },
]

export function normalizeRuntimeRoute(
  value: unknown,
  fallback: RuntimeRoute = DEFAULT_RUNTIME_ROUTE,
): RuntimeRoute {
  return RUNTIME_ROUTE_OPTIONS.some((item) => item.value === value)
    ? value as RuntimeRoute
    : fallback
}

function getStorageValue(storage: Storage, key: string): string | null {
  try {
    return storage.getItem(key)
  } catch {
    return null
  }
}

function isLegacyImplicitDefault(stored: string | null, defaultVersion: string | null): boolean {
  return (stored === 'route_c' && defaultVersion === LEGACY_PLATFORM_DEFAULT_VERSION)
    || (stored === 'route_a' && defaultVersion === LEGACY_PROJECT_LOCAL_DEFAULT_VERSION)
}

export function initialRuntimeRouteFromStorage(storage?: Storage | null): RuntimeRoute {
  if (!storage) return DEFAULT_RUNTIME_ROUTE
  const stored = getStorageValue(storage, RUNTIME_ROUTE_STORAGE_KEY)
  const defaultVersion = getStorageValue(storage, RUNTIME_ROUTE_DEFAULT_VERSION_KEY)
  if (!stored || isLegacyImplicitDefault(stored, defaultVersion)) return DEFAULT_RUNTIME_ROUTE
  return normalizeRuntimeRoute(stored)
}

export function initialProjectRuntimeRouteFromStorage(storage?: Storage | null): RuntimeRoute {
  if (!storage) return DEFAULT_PROJECT_RUNTIME_ROUTE
  const stored = getStorageValue(storage, RUNTIME_ROUTE_STORAGE_KEY)
  const defaultVersion = getStorageValue(storage, RUNTIME_ROUTE_DEFAULT_VERSION_KEY)
  if (!stored || isLegacyImplicitDefault(stored, defaultVersion)) return DEFAULT_PROJECT_RUNTIME_ROUTE
  return normalizeRuntimeRoute(stored, DEFAULT_PROJECT_RUNTIME_ROUTE)
}

export function persistRuntimeRouteSelection(storage: Storage | null | undefined, value: RuntimeRoute): void {
  try {
    storage?.setItem(RUNTIME_ROUTE_STORAGE_KEY, value)
    storage?.setItem(RUNTIME_ROUTE_DEFAULT_VERSION_KEY, RUNTIME_ROUTE_DEFAULT_VERSION)
  } catch {
    // Ignore blocked storage; the selected route still works for the current session.
  }
}

export function persistProjectRuntimeRouteSelection(storage: Storage | null | undefined, value: RuntimeRoute): void {
  try {
    storage?.setItem(RUNTIME_ROUTE_STORAGE_KEY, value)
    storage?.setItem(RUNTIME_ROUTE_DEFAULT_VERSION_KEY, PROJECT_RUNTIME_ROUTE_DEFAULT_VERSION)
  } catch {
    // Ignore blocked storage; the selected value still works for the current session.
  }
}

export function runtimeRouteOption(value: RuntimeRoute): RuntimeRouteOption {
  return RUNTIME_ROUTE_OPTIONS.find((item) => item.value === value)
    ?? RUNTIME_ROUTE_OPTIONS.find((item) => item.value === DEFAULT_RUNTIME_ROUTE)
    ?? RUNTIME_ROUTE_OPTIONS[0]
}

export function runtimeRouteDescription(value: RuntimeRoute): string {
  return runtimeRouteOption(value).description
}
