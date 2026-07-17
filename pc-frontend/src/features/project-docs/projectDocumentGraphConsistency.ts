import type { ProjectCapabilityGraph } from './projectDocumentCapabilityGraph'
import type { DocumentCatalog } from './projectDocumentModel'

export type ProjectDocumentGraphConsistencyStatus =
  | 'current'
  | 'stale'
  | 'workspace_mismatch'
  | 'unexpected_template'
  | 'template'
  | 'unavailable'

export interface ProjectDocumentGraphConsistency {
  status: ProjectDocumentGraphConsistencyStatus
  message: string
  blocking: boolean
}

export function diagnoseProjectDocumentGraphConsistency(input: {
  catalog: DocumentCatalog | null
  graph: ProjectCapabilityGraph
  expectedWorkspace: string
  expectedManifestRevision?: string
  configuredNodes: number
}): ProjectDocumentGraphConsistency {
  const { catalog, graph, expectedWorkspace, expectedManifestRevision, configuredNodes } = input
  if (!catalog || graph.source === 'unavailable') {
    return result('unavailable', '图谱数据尚未返回，请刷新目录或升级 Windows 节点。', true)
  }

  if (expectedWorkspace && catalog.workspace
    && normalizeWorkspace(expectedWorkspace) !== normalizeWorkspace(catalog.workspace)) {
    return result('workspace_mismatch', '脑图来自另一个项目目录，已阻止把它当作当前项目事实。', true)
  }

  const catalogManifestRevision = catalog.analysis?.identity?.manifest_revision
  if (expectedManifestRevision && catalogManifestRevision
    && expectedManifestRevision !== catalogManifestRevision) {
    return result('stale', '项目清单已经更新，当前脑图仍是旧快照；正在请求重新扫描。', true)
  }

  if (graph.source === 'profile_template' && configuredNodes > 0) {
    return result('unexpected_template', '项目已有正式图谱节点，却收到了模板图；这是数据一致性异常。', true)
  }
  if (graph.source === 'profile_template') {
    return result('template', '当前是通用模板，不代表项目已经确认的真实功能架构。', false)
  }
  return result('current', '项目清单、目录快照与当前脑图一致。', false)
}

export function shortGraphRevision(value?: string) {
  return value?.trim() ? value.trim().slice(0, 8) : '—'
}

function normalizeWorkspace(value: string) {
  return value.trim().replace(/^\\\\\?\\/, '').replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()
}

function result(
  status: ProjectDocumentGraphConsistencyStatus,
  message: string,
  blocking: boolean,
): ProjectDocumentGraphConsistency {
  return { status, message, blocking }
}
