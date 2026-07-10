import type { UiTunerDocument, UiTunerElement } from './types'
import {
  analyzeRepeatComponents,
  type UiTunerRepeatAnalysis,
  type UiTunerRepeatGroup,
} from './runtime/repeatComponents'

export type UiTunerViewMode = 'product' | 'layout' | 'source' | 'debug'

export interface UiTunerFilterState {
  mode: UiTunerViewMode
  query: string
  onlyTargetPackage: boolean
  onlySourceMapped: boolean
  onlyInteractive: boolean
  showStructural: boolean
  showHidden: boolean
  minSize: number
}

export interface UiTunerElementAnalysis {
  id: string
  depth: number
  role: string
  groupKey: string
  groupLabel: string
  isRuntime: boolean
  isInteractive: boolean
  isSourceMapped: boolean
  isTargetPackage: boolean
  isStructural: boolean
  isDuplicateBounds: boolean
  repeatGroupId?: string
  repeatCount: number
  isRepeatRepresentative: boolean
  isVisible: boolean
  isLocked: boolean
  appearance: 'solid' | 'ghost' | 'outline'
  hiddenReasons: string[]
}

export interface UiTunerFilteredElement {
  element: UiTunerElement
  analysis: UiTunerElementAnalysis
}

export interface UiTunerLayerGroup {
  key: string
  label: string
  items: UiTunerFilteredElement[]
}

export interface UiTunerFilterResult {
  visible: UiTunerFilteredElement[]
  groups: UiTunerLayerGroup[]
  analysisById: Record<string, UiTunerElementAnalysis>
  totalCount: number
  hiddenCount: number
  structuralCount: number
  duplicateCount: number
  sourceMappedCount: number
  repeatedInstanceCount: number
  repeatGroups: UiTunerRepeatGroup[]
}

export const DEFAULT_UI_TUNER_FILTER: UiTunerFilterState = {
  mode: 'product',
  query: '',
  onlyTargetPackage: true,
  onlySourceMapped: false,
  onlyInteractive: false,
  showStructural: false,
  showHidden: false,
  minSize: 8,
}

export function filterUiTunerElements(
  document: UiTunerDocument,
  filter: UiTunerFilterState,
): UiTunerFilterResult {
  const duplicateWinners = pickDuplicateBoundsWinners(document.elements)
  const repeatAnalysis = analyzeRepeatComponents(document.elements)
  const analysisById: Record<string, UiTunerElementAnalysis> = {}
  const visible: UiTunerFilteredElement[] = []
  let structuralCount = 0
  let duplicateCount = 0
  let sourceMappedCount = 0

  for (const element of document.elements) {
    const analysis = analyzeElement(document, element, duplicateWinners, repeatAnalysis, filter)
    analysisById[element.id] = analysis
    if (analysis.isStructural) structuralCount += 1
    if (analysis.isDuplicateBounds) duplicateCount += 1
    if (analysis.isSourceMapped) sourceMappedCount += 1
    if (analysis.hiddenReasons.length === 0) {
      visible.push({ element, analysis })
    }
  }

  return {
    visible,
    groups: groupElements(visible),
    analysisById,
    totalCount: document.elements.length,
    hiddenCount: document.elements.length - visible.length,
    structuralCount,
    duplicateCount,
    sourceMappedCount,
    repeatedInstanceCount: repeatAnalysis.repeatedInstanceCount,
    repeatGroups: repeatAnalysis.groups,
  }
}

function analyzeElement(
  document: UiTunerDocument,
  element: UiTunerElement,
  duplicateWinners: Map<string, string>,
  repeatAnalysis: UiTunerRepeatAnalysis,
  filter: UiTunerFilterState,
): UiTunerElementAnalysis {
  const hiddenReasons: string[] = []
  const className = element.runtime?.className ?? ''
  const isRuntime = Boolean(element.runtime)
  const isInteractive = element.kind === 'button' || /button|tab|checkbox|switch|edittext|input/i.test(className)
  const isSourceMapped = Boolean(element.source?.file)
  const isTargetPackage = !document.runtimeSnapshot?.packageName
    || !element.runtime?.packageName
    || element.runtime.packageName === document.runtimeSnapshot.packageName
  const isStructural = isStructuralElement(element)
  const isVisible = element.visibility !== 'hidden'
  const isLocked = element.visibility === 'locked'
  const depth = element.runtime?.indexPath.length ?? 0
  const duplicateWinner = duplicateWinners.get(boundsKey(element))
  const isDuplicateBounds = Boolean(duplicateWinner && duplicateWinner !== element.id)
  const repeatGroup = repeatAnalysis.groupByElementId[element.id]
  const isRepeatRepresentative = !repeatGroup || repeatGroup.representativeId === element.id
  const role = inferRole(element)
  const groupLabel = inferGroupLabel(element, role)
  const query = filter.query.trim().toLowerCase()

  if (!filter.showHidden && !isVisible) hiddenReasons.push('已隐藏')
  if (filter.onlyTargetPackage && !isTargetPackage) hiddenReasons.push('非目标包')
  if (filter.onlySourceMapped && !isSourceMapped) hiddenReasons.push('无源码映射')
  if (filter.onlyInteractive && !isInteractive) hiddenReasons.push('非可交互')
  if (Math.min(element.width, element.height) < filter.minSize) hiddenReasons.push('尺寸过小')
  if (query && !matchesQuery(element, role, query)) hiddenReasons.push('搜索不匹配')

  if (filter.mode !== 'debug' && isDuplicateBounds) hiddenReasons.push('同边界重复')
  if (filter.mode === 'product' && repeatGroup && !isRepeatRepresentative) {
    hiddenReasons.push(`同组件实例（共 ${repeatGroup.count} 个）`)
  }
  if (filter.mode === 'product' && isStructural && !filter.showStructural && !isSourceMapped) {
    hiddenReasons.push('结构容器')
  }
  if (filter.mode === 'source' && !isSourceMapped && !element.runtime?.resourceId) {
    hiddenReasons.push('无源码标识')
  }
  if (filter.mode === 'layout' && !filter.showStructural && isStructural && !isSourceMapped) {
    hiddenReasons.push('结构容器')
  }

  return {
    id: element.id,
    depth,
    role,
    groupKey: groupLabel,
    groupLabel,
    isRuntime,
    isInteractive,
    isSourceMapped,
    isTargetPackage,
    isStructural,
    isDuplicateBounds,
    repeatGroupId: repeatGroup?.id,
    repeatCount: repeatGroup?.count ?? 1,
    isRepeatRepresentative,
    isVisible,
    isLocked,
    appearance: resolveAppearance(filter.mode, isStructural, isSourceMapped, isVisible),
    hiddenReasons,
  }
}

function isStructuralElement(element: UiTunerElement) {
  if (!element.runtime) return false
  const className = (element.runtime.className ?? '').toLowerCase()
  if (element.kind !== 'card') return false
  if (element.text && element.text !== element.name) return false
  return /layout|viewgroup|recyclerview|scrollview|fragment|container|frame/.test(className)
}

function resolveAppearance(
  mode: UiTunerViewMode,
  isStructural: boolean,
  isSourceMapped: boolean,
  isVisible: boolean,
): UiTunerElementAnalysis['appearance'] {
  if (!isVisible) return 'ghost'
  if (mode === 'debug') return 'solid'
  if (isStructural) return isSourceMapped ? 'outline' : 'ghost'
  return 'solid'
}

function pickDuplicateBoundsWinners(elements: UiTunerElement[]) {
  const winners = new Map<string, UiTunerElement>()
  for (const element of elements) {
    const key = boundsKey(element)
    const current = winners.get(key)
    if (!current || scoreElement(element) > scoreElement(current)) {
      winners.set(key, element)
    }
  }
  return new Map(Array.from(winners.entries()).map(([key, element]) => [key, element.id]))
}

function scoreElement(element: UiTunerElement) {
  let score = 0
  if (element.source?.file) score += 28
  if (element.runtime?.resourceId) score += 20
  if (element.kind === 'button') score += 18
  if (element.kind === 'text') score += 14
  if (element.text && element.text === element.name) score += 8
  score += element.runtime?.indexPath.length ?? 0
  return score
}

function boundsKey(element: UiTunerElement) {
  return `${element.x}:${element.y}:${element.width}:${element.height}`
}

function matchesQuery(element: UiTunerElement, role: string, query: string) {
  return [
    element.name,
    element.text,
    element.source?.file,
    element.source?.token,
    element.runtime?.resourceId,
    element.runtime?.className,
    role,
  ].some((value) => value?.toLowerCase().includes(query))
}

function inferRole(element: UiTunerElement) {
  const resource = lastResourceName(element.runtime?.resourceId ?? element.source?.token)
  if (resource) return resource
  if (element.source?.token) return element.source.token
  return element.name || element.kind
}

function inferGroupLabel(element: UiTunerElement, role: string) {
  if (element.source?.file) return sourceFileName(element.source.file)
  if (element.runtime?.packageName && element.runtime.resourceId) return element.runtime.packageName
  if (element.runtime?.className) return simplifyClassName(element.runtime.className)
  if (role) return '手动图层'
  return '未分组'
}

function groupElements(items: UiTunerFilteredElement[]): UiTunerLayerGroup[] {
  const groups = new Map<string, UiTunerLayerGroup>()
  for (const item of items) {
    const key = item.analysis.groupKey
    const group = groups.get(key) ?? { key, label: key, items: [] }
    group.items.push(item)
    groups.set(key, group)
  }
  return Array.from(groups.values())
}

function sourceFileName(file: string) {
  return file.split(/[\\/]/).pop() ?? file
}

function simplifyClassName(className: string) {
  return className.split('.').pop() ?? className
}

function lastResourceName(resourceId?: string) {
  if (!resourceId) return ''
  const normalized = resourceId.replace(/.*(?:[:/]id\/|\+id\/)/, '')
  return normalized.split('/').pop() ?? normalized
}
