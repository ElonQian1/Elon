import type { UiTunerElement } from '../types'

export interface UiTunerRepeatGroup {
  id: string
  fingerprint: string
  label: string
  representativeId: string
  memberIds: string[]
  count: number
  componentKey?: string
  sourceFile?: string
  confidence: 'high' | 'medium'
}

export interface UiTunerRepeatAnalysis {
  groups: UiTunerRepeatGroup[]
  groupByElementId: Record<string, UiTunerRepeatGroup>
  repeatedInstanceCount: number
}

export function analyzeRepeatComponents(elements: UiTunerElement[]): UiTunerRepeatAnalysis {
  const buckets = new Map<string, UiTunerElement[]>()
  for (const element of elements) {
    const fingerprint = repeatFingerprint(element)
    if (!fingerprint) continue
    const bucket = buckets.get(fingerprint) ?? []
    bucket.push(element)
    buckets.set(fingerprint, bucket)
  }

  const groups = Array.from(buckets.entries())
    .filter(([, members]) => members.length >= 2 && spansMultiplePositions(members))
    .map(([fingerprint, members]) => createGroup(fingerprint, members))
    .sort((left, right) => right.count - left.count)
  const groupByElementId: Record<string, UiTunerRepeatGroup> = {}
  for (const group of groups) {
    for (const memberId of group.memberIds) groupByElementId[memberId] = group
  }
  return {
    groups,
    groupByElementId,
    repeatedInstanceCount: groups.reduce((total, group) => total + group.count - 1, 0),
  }
}

function createGroup(fingerprint: string, members: UiTunerElement[]): UiTunerRepeatGroup {
  const representative = [...members].sort((left, right) => score(right) - score(left))[0]
  const componentKey = representative.source?.componentKey
  const sourceFile = representative.source?.file
  return {
    id: `repeat-${stableHash(fingerprint)}`,
    fingerprint,
    label: repeatLabel(representative, componentKey),
    representativeId: representative.id,
    memberIds: members.map((member) => member.id),
    count: members.length,
    componentKey,
    sourceFile,
    confidence: componentKey || representative.runtime?.resourceId ? 'high' : 'medium',
  }
}

function repeatFingerprint(element: UiTunerElement): string | null {
  if (!element.runtime) return null
  const componentKey = element.source?.componentKey
  if (componentKey && element.source?.scope === 'repeated_component') {
    return `component:${componentKey}:${element.kind}:${resourceName(element.runtime.resourceId)}`
  }
  const normalizedPath = element.runtime.xpath.replace(/\[\d+\]/g, '[*]')
  const resource = resourceName(element.runtime.resourceId)
  const className = element.runtime.className?.split('.').pop() ?? ''
  if (!resource && !className) return null
  return [
    'runtime',
    normalizedPath,
    className,
    resource,
    element.kind,
    sizeBucket(element.width),
    sizeBucket(element.height),
  ].join(':')
}

function spansMultiplePositions(elements: UiTunerElement[]) {
  const positions = new Set(elements.map((element) => `${element.x}:${element.y}`))
  return positions.size > 1
}

function score(element: UiTunerElement) {
  let value = 0
  if (element.source?.componentKey) value += 40
  if (element.source?.file) value += 30
  if (element.runtime?.resourceId) value += 20
  if (element.kind === 'button') value += 10
  if (element.text) value += 4
  return value
}

function repeatLabel(element: UiTunerElement, componentKey?: string) {
  if (componentKey) return componentKey.replace(/^(layout|compose):/, '')
  return resourceName(element.runtime?.resourceId) || element.name || '重复组件'
}

function resourceName(resourceId?: string) {
  if (!resourceId) return ''
  return resourceId.replace(/.*(?:[:/]id\/|\+id\/)/, '').split('/').pop() ?? resourceId
}

function sizeBucket(value: number) {
  return Math.max(1, Math.round(value / 8))
}

function stableHash(value: string) {
  let hash = 2166136261
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index)
    hash = Math.imul(hash, 16777619)
  }
  return (hash >>> 0).toString(36)
}

export function selectedRepeatGroup(
  analysis: UiTunerRepeatAnalysis,
  elementId?: string | null,
) {
  return elementId ? analysis.groupByElementId[elementId] ?? null : null
}
