import type { LocalAiWebSessionState } from './localAiBrowserApi'
import {
  isLocalAiComposerControlsSnapshot,
  isLocalAiFeatureNavigationSnapshot,
  type LocalAiComposerControlsSnapshot,
  type LocalAiComposerOption,
  type LocalAiFeatureNavigationItem,
  type LocalAiFeatureNavigationSnapshot,
} from './localAiBrowserProtocol'

export const LOCAL_AI_INTERACTION_PRESET_PREFIX = 'preset:'
export const LOCAL_AI_STABLE_INTERACTION_FRESH_MS = 6 * 60 * 60 * 1_000

const CHATGPT_MODELS: LocalAiComposerOption[] = [
  option('preset:chatgpt:model:advanced', '高级', 'model', true),
  option('preset:chatgpt:model:auto', '自动', 'model'),
]

const CHATGPT_TOOLS: LocalAiComposerOption[] = [
  option('preset:chatgpt:tool:image-generation', '创建图片', 'image_generation'),
  option('preset:chatgpt:tool:web-search', '网页搜索', 'web_search'),
]

const CHATGPT_FEATURES: LocalAiFeatureNavigationItem[] = [
  { id: 'preset:chatgpt:feature:images', label: '图像', kind: 'images', selected: false },
]

export function localAiBuiltInComposerOptions(
  providerId: string | undefined,
  section: LocalAiComposerControlsSnapshot['section'],
): LocalAiComposerOption[] {
  if (providerId !== 'chatgpt') return []
  return cloneOptions(section === 'model' ? CHATGPT_MODELS : CHATGPT_TOOLS)
}

export function localAiBuiltInFeatures(providerId: string | undefined): LocalAiFeatureNavigationItem[] {
  return providerId === 'chatgpt' ? CHATGPT_FEATURES.map((item) => ({ ...item })) : []
}

export function localAiComposerOptionsOrPreset(
  providerId: string | undefined,
  section: LocalAiComposerControlsSnapshot['section'],
  observed: readonly LocalAiComposerOption[],
): LocalAiComposerOption[] {
  return observed.length ? observed.map((item) => ({ ...item })) : localAiBuiltInComposerOptions(providerId, section)
}

export function localAiFeaturesOrPreset(
  providerId: string | undefined,
  observed: readonly LocalAiFeatureNavigationItem[],
): LocalAiFeatureNavigationItem[] {
  return observed.length ? observed.map((item) => ({ ...item })) : localAiBuiltInFeatures(providerId)
}

export function isLocalAiInteractionPreset(id: string): boolean {
  return id.startsWith(LOCAL_AI_INTERACTION_PRESET_PREFIX)
}

export function localAiStableInteractionNeedsRefresh(
  values: readonly { id: string }[],
  updatedAt: number,
  nowMs = Date.now(),
): boolean {
  return values.length === 0
    || values.some((value) => isLocalAiInteractionPreset(value.id))
    || updatedAt <= 0
    || nowMs - updatedAt < 0
    || nowMs - updatedAt >= LOCAL_AI_STABLE_INTERACTION_FRESH_MS
}

export function resolveLocalAiComposerPreset(
  preset: LocalAiComposerOption,
  live: readonly LocalAiComposerOption[],
): LocalAiComposerOption | null {
  if (!isLocalAiInteractionPreset(preset.id)) return live.find((item) => item.id === preset.id) ?? null
  const semantic = compact(preset.semantic)
  const label = semantic === 'model' ? compactModelLabel(preset.label) : compact(preset.label)
  return live.find((item) => !isLocalAiInteractionPreset(item.id) && (
    (semantic !== 'model' && semantic && compact(item.semantic) === semantic)
    || (semantic === 'model' ? compactModelLabel(item.label) : compact(item.label)) === label
  )) ?? null
}

export function resolveLocalAiFeaturePreset(
  preset: LocalAiFeatureNavigationItem,
  live: readonly LocalAiFeatureNavigationItem[],
): LocalAiFeatureNavigationItem | null {
  if (!isLocalAiInteractionPreset(preset.id)) return live.find((item) => item.id === preset.id) ?? null
  const kind = compact(preset.kind)
  const label = compact(preset.label)
  return live.find((item) => !isLocalAiInteractionPreset(item.id) && (
    (kind && compact(item.kind) === kind)
    || compact(item.label) === label
  )) ?? null
}

export function localAiComposerSnapshotFromState(
  state: LocalAiWebSessionState | null | undefined,
  section: LocalAiComposerControlsSnapshot['section'],
): LocalAiComposerControlsSnapshot | null {
  const grouped = state?.composerEvents?.[section]
  if (isLocalAiComposerControlsSnapshot(grouped) && grouped.section === section) return grouped
  if (isLocalAiComposerControlsSnapshot(state?.composerEvent) && state.composerEvent.section === section) {
    return state.composerEvent
  }
  return null
}

export function localAiFeatureSnapshotFromState(
  state: LocalAiWebSessionState | null | undefined,
): LocalAiFeatureNavigationSnapshot | null {
  return isLocalAiFeatureNavigationSnapshot(state?.featureEvent) ? state.featureEvent : null
}

function option(
  id: string,
  label: string,
  semantic: string,
  opensSubmenu = false,
): LocalAiComposerOption {
  return { id, label, selected: false, kind: semantic === 'model' ? 'model' : 'tool', semantic, opensSubmenu }
}

function cloneOptions(values: readonly LocalAiComposerOption[]): LocalAiComposerOption[] {
  return values.map((item) => ({ ...item }))
}

function compact(value: string): string {
  return value.normalize('NFKC').toLocaleLowerCase().replace(/[\p{P}\p{S}\s_]+/gu, '')
}

function compactModelLabel(value: string): string {
  const normalized = value.normalize('NFKC').toLocaleLowerCase()
  const matches = [...normalized.matchAll(
    /极高|重度|中度|标准|轻度|高|中|低|自动|快速|思考|extra\s*high|high|medium|low|auto|fast|thinking/gu,
  )]
  const token = matches[matches.length - 1]?.[0]?.replace(/\s+/g, '')
  if (token === '轻度' || token === 'low') return '低'
  if (['标准', '中度', 'medium'].includes(token ?? '')) return '中'
  if (['重度', 'high'].includes(token ?? '')) return '高'
  if (token === '极高' || token === 'extrahigh') return '极高'
  if (token === 'auto' || token === '自动') return '自动'
  if (token === 'fast' || token === '快速') return '快速'
  if (token === 'thinking' || token === '思考') return '思考'
  return compact(value)
}
