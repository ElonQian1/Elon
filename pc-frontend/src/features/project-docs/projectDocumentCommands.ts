import type { ProjectDocumentEntry } from './projectDocumentModel'
import {
  createCustomSection,
  customSectionKey,
  SYSTEM_DOCUMENT_SECTIONS,
  type CustomDocumentSection,
  type DocumentKnowledgeMetadata,
  type DocumentSection,
  type DocumentSectionManifest,
} from './projectDocumentSections'

export type SectionSortMode = 'manual' | 'name' | 'count'
export type DocumentSortMode = 'manual' | 'name' | 'path' | 'authority'

export interface ProjectDocumentViewPreferences {
  sectionSort: SectionSortMode
  documentSort: DocumentSortMode
}

export const DEFAULT_DOCUMENT_VIEW_PREFERENCES: ProjectDocumentViewPreferences = {
  sectionSort: 'manual',
  documentSort: 'manual',
}

export function loadDocumentViewPreferences(projectId: string) {
  try { return parseDocumentViewPreferences(window.localStorage.getItem(viewPreferencesKey(projectId))) }
  catch { return DEFAULT_DOCUMENT_VIEW_PREFERENCES }
}

export function saveDocumentViewPreferences(projectId: string, preferences: ProjectDocumentViewPreferences) {
  try { window.localStorage.setItem(viewPreferencesKey(projectId), JSON.stringify(preferences)) }
  catch { /* Optional per-user view settings may be unavailable in private browsing. */ }
}

export function parseDocumentViewPreferences(value: string | null): ProjectDocumentViewPreferences {
  try {
    const parsed = JSON.parse(value ?? '{}') as Partial<ProjectDocumentViewPreferences>
    return {
      sectionSort: ['manual', 'name', 'count'].includes(parsed.sectionSort ?? '')
        ? parsed.sectionSort as SectionSortMode
        : 'manual',
      documentSort: ['manual', 'name', 'path', 'authority'].includes(parsed.documentSort ?? '')
        ? parsed.documentSort as DocumentSortMode
        : 'manual',
    }
  } catch {
    return DEFAULT_DOCUMENT_VIEW_PREFERENCES
  }
}

export function updateSectionDefinition(
  manifest: DocumentSectionManifest,
  sectionKey: string,
  patch: Partial<Pick<CustomDocumentSection, 'label' | 'detail' | 'color' | 'icon' | 'parent_id'>>,
) {
  const id = customId(sectionKey)
  const current = manifest.sections.find((section) => section.id === id)
  if (!current) throw new Error('只有用户自定义知识分区可以修改')
  const parentId = patch.parent_id === undefined ? current.parent_id : customId(patch.parent_id)
  validateParentChange(manifest.sections, id, parentId)
  const sections = manifest.sections.map((section) => section.id === id ? {
    ...section,
    label: patch.label?.trim().slice(0, 40) || section.label,
    detail: patch.detail?.trim().slice(0, 120) ?? section.detail,
    color: /^#[0-9a-f]{6}$/i.test(patch.color ?? '') ? patch.color! : section.color,
    icon: patch.icon?.trim().slice(0, 32) ?? section.icon,
    parent_id: parentId,
  } : section)
  return recordManifestChange(
    normalizeSectionOrders({ ...manifest, sections }),
    parentId !== current.parent_id ? 'section.move_parent' : 'section.update',
    sectionKey,
    parentId !== current.parent_id
      ? `将“${current.label}”移动到${parentId ? `“${manifest.sections.find((item) => item.id === parentId)?.label ?? parentId}”` : '知识树根级'}`
      : `更新分区“${current.label}”`,
  )
}

export function createSectionInManifest(
  manifest: DocumentSectionManifest,
  label: string,
  parentId = '',
  appearance?: { detail?: string; color?: string; icon?: string },
) {
  const created = createCustomSection(label, manifest.sections, parentId)
  const section = {
    ...created,
    detail: appearance?.detail?.trim().slice(0, 120) || created.detail,
    color: /^#[0-9a-f]{6}$/i.test(appearance?.color ?? '') ? appearance!.color! : created.color,
    icon: appearance?.icon?.trim().slice(0, 32) || '',
  }
  validateSectionTree([...manifest.sections, section])
  return {
    key: customSectionKey(section.id),
    manifest: recordManifestChange({ ...manifest, sections: [...manifest.sections, section] },
      'section.create', customSectionKey(section.id), `新建分区“${section.label}”`),
  }
}

export function canCreateSectionUnder(sections: CustomDocumentSection[], parentId: string) {
  if (!parentId) return true
  if (!sections.some((section) => section.id === parentId)) return false
  return depthOf(sections, parentId) < 3
}

export function canMoveSectionToParent(sections: CustomDocumentSection[], id: string, parentId: string) {
  try {
    validateParentChange(sections, id, parentId)
    return true
  } catch {
    return false
  }
}

export function reorderSection(
  manifest: DocumentSectionManifest,
  sectionKey: string,
  direction: 'top' | 'up' | 'down' | 'bottom',
) {
  const id = customId(sectionKey)
  const current = manifest.sections.find((section) => section.id === id)
  if (!current) throw new Error('模板分区不能修改项目共同顺序')
  const siblings = manifest.sections
    .filter((section) => section.parent_id === current.parent_id)
    .sort(sectionOrder)
  const index = siblings.findIndex((section) => section.id === id)
  if (index < 0) return manifest
  const targetIndex = direction === 'top' ? 0
    : direction === 'bottom' ? siblings.length - 1
      : direction === 'up' ? Math.max(0, index - 1)
        : Math.min(siblings.length - 1, index + 1)
  if (targetIndex === index) return manifest
  siblings.splice(index, 1)
  siblings.splice(targetIndex, 0, current)
  const orders = new Map(siblings.map((section, orderIndex) => [section.id, (orderIndex + 1) * 10]))
  return recordManifestChange({
    ...manifest,
    sections: manifest.sections.map((section) => orders.has(section.id)
      ? { ...section, order: orders.get(section.id)! }
      : section),
  }, `section.${direction}`, sectionKey, `调整分区“${current.label}”的项目共同顺序`)
}

export function reorderSectionBefore(
  manifest: DocumentSectionManifest,
  sectionKey: string,
  beforeSectionKey: string,
) {
  const id = customId(sectionKey)
  const beforeId = customId(beforeSectionKey)
  const current = manifest.sections.find((section) => section.id === id)
  const before = manifest.sections.find((section) => section.id === beforeId)
  if (!current || !before || current.parent_id !== before.parent_id || id === beforeId) return manifest
  const siblings = manifest.sections.filter((section) => section.parent_id === current.parent_id).sort(sectionOrder)
  const from = siblings.findIndex((section) => section.id === id)
  const to = siblings.findIndex((section) => section.id === beforeId)
  siblings.splice(from, 1)
  siblings.splice(to > from ? to - 1 : to, 0, current)
  const orders = new Map(siblings.map((section, index) => [section.id, (index + 1) * 10]))
  return recordManifestChange({
    ...manifest,
    sections: manifest.sections.map((section) => orders.has(section.id)
      ? { ...section, order: orders.get(section.id)! }
      : section),
  }, 'section.drag_reorder', sectionKey, `拖动调整分区“${current.label}”的项目共同顺序`)
}

export function mergeSections(manifest: DocumentSectionManifest, sourceKey: string, targetKey: string) {
  const sourceId = customId(sourceKey)
  const targetId = customId(targetKey)
  const source = manifest.sections.find((section) => section.id === sourceId)
  const target = manifest.sections.find((section) => section.id === targetId)
  if (!source || !target || sourceId === targetId) throw new Error('请选择两个不同的自定义分区')
  if (isDescendant(manifest.sections, targetId, sourceId)) throw new Error('不能把父分区合并到自己的子分区')
  const sourceKeyNormalized = customSectionKey(sourceId)
  const targetKeyNormalized = customSectionKey(targetId)
  const assignments = Object.fromEntries(Object.entries(manifest.assignments).map(([path, section]) => (
    [path, section === sourceKeyNormalized ? targetKeyNormalized : section]
  )))
  const sections = manifest.sections
    .filter((section) => section.id !== sourceId)
    .map((section) => section.parent_id === sourceId ? { ...section, parent_id: targetId } : section)
    .map((section) => section.id === targetId && !section.entrypoint && source.entrypoint
      ? { ...section, entrypoint: source.entrypoint }
      : section)
  validateSectionTree(sections)
  return recordManifestChange(normalizeSectionOrders({ ...manifest, sections, assignments }),
    'section.merge', sourceKey, `将“${source.label}”合并到“${target.label}”`)
}

export function removeSectionTree(manifest: DocumentSectionManifest, sectionKey: string) {
  const id = customId(sectionKey)
  const root = manifest.sections.find((section) => section.id === id)
  if (!root) throw new Error('只有用户自定义分区可以删除')
  const removed = new Set([id])
  let changed = true
  while (changed) {
    changed = false
    manifest.sections.forEach((section) => {
      if (removed.has(section.parent_id) && !removed.has(section.id)) {
        removed.add(section.id)
        changed = true
      }
    })
  }
  const assignments = Object.fromEntries(Object.entries(manifest.assignments)
    .filter(([, section]) => !removed.has(customId(section))))
  return recordManifestChange({
    ...manifest,
    sections: manifest.sections.filter((section) => !removed.has(section.id)),
    assignments,
  }, 'section.delete_tree', sectionKey, `删除分区“${root.label}”及其 ${removed.size - 1} 个子分区；Markdown 未删除`)
}

export function assignDocuments(
  manifest: DocumentSectionManifest,
  paths: string[],
  sectionKey: string,
  facet: 'knowledge' | 'governance',
) {
  const field = facet === 'knowledge' ? 'assignments' : 'governance_overrides'
  if (sectionKey) {
    const valid = facet === 'knowledge'
      ? manifest.sections.some((section) => customSectionKey(section.id) === sectionKey)
      : SYSTEM_DOCUMENT_SECTIONS.some((section) => section.key === sectionKey)
    if (!valid) throw new Error('目标分区不存在')
  }
  const assignments = { ...manifest[field] }
  paths.map(normalizePath).filter(Boolean).forEach((path) => {
    if (sectionKey) assignments[path] = sectionKey
    else delete assignments[path]
  })
  const label = sectionKey || '自动分类'
  return recordManifestChange({ ...manifest, [field]: assignments },
    facet === 'knowledge' ? 'document.assign_topic' : 'document.assign_governance',
    paths.join(','), `${paths.length} 份文档调整为“${label}”；真实路径与正文未改变`)
}

export function setRecommendedDocuments(manifest: DocumentSectionManifest, paths: string[], enabled: boolean) {
  const normalized = paths.map(normalizePath).filter(Boolean)
  const selected = new Set(normalized)
  const startHere = enabled
    ? [...manifest.home.start_here, ...normalized]
    : manifest.home.start_here.filter((path) => !selected.has(normalizePath(path)))
  return recordManifestChange({
    ...manifest,
    home: { ...manifest.home, start_here: [...new Set(startHere)].slice(0, 12) },
  }, enabled ? 'document.recommend' : 'document.unrecommend', normalized.join(','),
  `${enabled ? '加入' : '移出'}知识首页推荐阅读：${normalized.length} 份`)
}

export function setKnowledgeEntrypoint(
  manifest: DocumentSectionManifest,
  path: string,
  sectionKey?: string,
) {
  const normalized = normalizePath(path)
  if (!sectionKey) {
    return recordManifestChange({ ...manifest, home: { ...manifest.home, entrypoint: normalized } },
      'document.home_entrypoint', normalized, `将 ${normalized} 设为知识首页入口`)
  }
  const id = customId(sectionKey)
  const section = manifest.sections.find((item) => item.id === id)
  if (!section) throw new Error('模板分区入口由模板维护；可先创建自定义分区')
  return recordManifestChange({
    ...manifest,
    sections: manifest.sections.map((item) => item.id === id ? { ...item, entrypoint: normalized } : item),
  }, 'document.section_entrypoint', normalized, `将 ${normalized} 设为“${section.label}”入口`)
}

export function pinDocuments(manifest: DocumentSectionManifest, paths: string[], pinned: boolean) {
  const normalized = paths.map(normalizePath).filter(Boolean)
  const metadata = { ...manifest.document_metadata }
  normalized.forEach((path) => {
    const current = metadata[path] ?? emptyKnowledgeMetadata()
    metadata[path] = { ...current, pinned }
  })
  return recordManifestChange({ ...manifest, document_metadata: metadata },
    pinned ? 'document.pin' : 'document.unpin', normalized.join(','),
  `${pinned ? '固定' : '取消固定'} ${normalized.length} 份文档`)
}

export function reorderDocument(
  manifest: DocumentSectionManifest,
  orderedPaths: string[],
  path: string,
  direction: 'top' | 'up' | 'down' | 'bottom',
) {
  const ordered = uniquePaths(orderedPaths)
  const normalized = normalizePath(path)
  const index = ordered.indexOf(normalized)
  if (index < 0) throw new Error('当前分区中找不到这篇文档')
  const targetIndex = direction === 'top' ? 0
    : direction === 'bottom' ? ordered.length - 1
      : direction === 'up' ? Math.max(0, index - 1)
        : Math.min(ordered.length - 1, index + 1)
  if (targetIndex === index) return manifest
  ordered.splice(index, 1)
  ordered.splice(targetIndex, 0, normalized)
  return setDocumentOrders(manifest, ordered, 'document.reorder', normalized, '调整文档的项目共同顺序')
}

export function reorderDocumentBefore(
  manifest: DocumentSectionManifest,
  orderedPaths: string[],
  path: string,
  beforePath: string,
) {
  const ordered = uniquePaths(orderedPaths)
  const normalized = normalizePath(path)
  const before = normalizePath(beforePath)
  const from = ordered.indexOf(normalized)
  const to = ordered.indexOf(before)
  if (from < 0 || to < 0 || from === to) return manifest
  ordered.splice(from, 1)
  ordered.splice(to > from ? to - 1 : to, 0, normalized)
  return setDocumentOrders(manifest, ordered, 'document.drag_reorder', normalized, '拖动调整文档的项目共同顺序')
}

export function sortDocuments(
  documents: ProjectDocumentEntry[],
  manifest: DocumentSectionManifest,
  mode: DocumentSortMode,
) {
  const authorityOrder: Record<string, number> = {
    repository_policy: 0, repository_routing: 1, domain_policy: 2,
    normative: 3, approved: 4, operational: 5, project_guidance: 6,
    decision_record: 7, informative: 8, provider_routing: 9, customization: 10,
    evidence: 11, proposal: 12, historical: 13, unknown: 14,
  }
  return [...documents].sort((left, right) => {
    const leftMeta = manifest.document_metadata[normalizePath(left.path)] ?? emptyKnowledgeMetadata()
    const rightMeta = manifest.document_metadata[normalizePath(right.path)] ?? emptyKnowledgeMetadata()
    if (leftMeta.pinned !== rightMeta.pinned) return leftMeta.pinned ? -1 : 1
    if (mode === 'name') return left.title.localeCompare(right.title, 'zh-CN') || left.path.localeCompare(right.path, 'zh-CN')
    if (mode === 'path') return left.path.localeCompare(right.path, 'zh-CN')
    if (mode === 'authority') {
      return (authorityOrder[left.metadata.authority] ?? 8) - (authorityOrder[right.metadata.authority] ?? 8)
        || left.path.localeCompare(right.path, 'zh-CN')
    }
    return leftMeta.order - rightMeta.order || left.path.localeCompare(right.path, 'zh-CN')
  })
}

export function sortHierarchicalSections(
  sections: DocumentSection[],
  mode: SectionSortMode,
  counts: Record<string, number>,
) {
  const keys = new Set(sections.map((section) => section.key))
  const children = new Map<string, DocumentSection[]>()
  sections.forEach((section) => {
    const parent = section.parentId && keys.has(section.parentId) ? section.parentId : ''
    children.set(parent, [...(children.get(parent) ?? []), section])
  })
  const compare = (left: DocumentSection, right: DocumentSection) => mode === 'name'
    ? left.label.localeCompare(right.label, 'zh-CN')
    : mode === 'count'
      ? (counts[right.key] ?? 0) - (counts[left.key] ?? 0) || left.label.localeCompare(right.label, 'zh-CN')
      : (left.order ?? 0) - (right.order ?? 0) || left.label.localeCompare(right.label, 'zh-CN')
  const output: DocumentSection[] = []
  const visit = (parent: string, depth: number) => {
    const siblings = children.get(parent) ?? []
    siblings.sort(compare).forEach((section) => {
      output.push({ ...section, depth })
      visit(section.key, depth + 1)
    })
  }
  visit('', 0)
  return output
}

export function recordManifestChange(
  manifest: DocumentSectionManifest,
  action: string,
  target: string,
  summary: string,
  at = new Date().toISOString(),
) {
  const audit = {
    id: `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`,
    action,
    target: target.slice(0, 500),
    summary: summary.slice(0, 500),
    at,
  }
  return { ...manifest, audit_log: [...manifest.audit_log, audit].slice(-100) }
}

function validateParentChange(sections: CustomDocumentSection[], id: string, parentId: string) {
  if (parentId && !sections.some((section) => section.id === parentId)) throw new Error('父分区不存在')
  if (id === parentId || isDescendant(sections, parentId, id)) throw new Error('不能把分区移动到自己的子分区')
  validateSectionTree(sections.map((section) => section.id === id ? { ...section, parent_id: parentId } : section))
}

function validateSectionTree(sections: CustomDocumentSection[]) {
  const parents = new Map(sections.map((section) => [section.id, section.parent_id]))
  sections.forEach((section) => {
    let cursor = section.id
    let depth = 0
    const visited = new Set<string>()
    while (cursor) {
      if (visited.has(cursor)) throw new Error('知识分区层级不能形成循环')
      visited.add(cursor)
      const parent = parents.get(cursor) ?? ''
      if (parent && !parents.has(parent)) throw new Error('父分区不存在')
      if (parent && ++depth > 3) throw new Error('知识分区最多支持四层')
      cursor = parent
    }
  })
}

function isDescendant(sections: CustomDocumentSection[], candidateId: string, ancestorId: string) {
  let cursor = candidateId
  const visited = new Set<string>()
  while (cursor && !visited.has(cursor)) {
    if (cursor === ancestorId) return true
    visited.add(cursor)
    cursor = sections.find((section) => section.id === cursor)?.parent_id ?? ''
  }
  return false
}

function depthOf(sections: CustomDocumentSection[], id: string) {
  let depth = 0
  let cursor = id
  const visited = new Set<string>()
  while (cursor && !visited.has(cursor)) {
    visited.add(cursor)
    cursor = sections.find((section) => section.id === cursor)?.parent_id ?? ''
    if (cursor) depth += 1
  }
  return depth
}

function normalizeSectionOrders(manifest: DocumentSectionManifest) {
  const byParent = new Map<string, CustomDocumentSection[]>()
  manifest.sections.forEach((section) => byParent.set(section.parent_id, [...(byParent.get(section.parent_id) ?? []), section]))
  const orders = new Map<string, number>()
  byParent.forEach((sections) => sections.sort(sectionOrder).forEach((section, index) => orders.set(section.id, (index + 1) * 10)))
  return { ...manifest, sections: manifest.sections.map((section) => ({ ...section, order: orders.get(section.id) ?? section.order })) }
}

function emptyKnowledgeMetadata(): DocumentKnowledgeMetadata {
  return { doc_type: '', audience: [], owner: '', version: '', related: [], supersedes: [], order: 999_999, pinned: false }
}

function setDocumentOrders(
  manifest: DocumentSectionManifest,
  orderedPaths: string[],
  action: string,
  target: string,
  summary: string,
) {
  const metadata = { ...manifest.document_metadata }
  orderedPaths.forEach((path, index) => {
    const current = metadata[path] ?? emptyKnowledgeMetadata()
    metadata[path] = { ...current, order: (index + 1) * 10 }
  })
  return recordManifestChange({ ...manifest, document_metadata: metadata }, action, target, summary)
}

function uniquePaths(paths: string[]) {
  return [...new Set(paths.map(normalizePath).filter(Boolean))]
}

function sectionOrder(left: CustomDocumentSection, right: CustomDocumentSection) {
  return left.order - right.order || left.label.localeCompare(right.label, 'zh-CN')
}

function customId(key: string) {
  return key.replace(/^custom:/, '').trim()
}

function normalizePath(path: string) {
  return path.trim().replace(/\\/g, '/')
}

function viewPreferencesKey(projectId: string) {
  return `elon:project-docs:view-preferences:${projectId}`
}
