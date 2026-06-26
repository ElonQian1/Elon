import { clean } from '../../lib/utils'
import type { ProjectNode } from './types'

export function formatBytes(value: number | undefined | null): string {
  const bytes = Number(value || 0)
  if (!bytes) return ''
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${bytes} B`
}

export function nodeId(node: ProjectNode): string {
  return clean(node.node_id ?? node.agent_id ?? node.id)
}

export function nodeCanAccept(node: ProjectNode): boolean {
  const explicit = node.can_accept_project
  if (explicit === true || explicit === 1 || explicit === '1') return true
  if (explicit === false || explicit === 0 || explicit === '0') return false
  const remaining = node.project_slots_remaining
  if (typeof remaining === 'number' && remaining <= 0) return false
  const tone = clean(node.capacity_tone).toLowerCase()
  return tone !== 'bad'
}

export function nodeLabel(node: ProjectNode): string {
  const id = nodeId(node)
  const shortId = clean(node.short_id) || (id.length > 16 ? '...' + id.slice(-14) : id)
  const name = clean(node.display_name ?? node.label ?? node.device_name) || shortId
  return `${name} · ${shortId} · ${nodeCapacitySummary(node)}`
}

function nodeProjectSlotsText(node: ProjectNode): string {
  const count = node.project_count
  const limit = node.project_limit
  const remaining = node.project_slots_remaining
  if (count != null && limit != null && limit > 0) {
    const suffix = remaining != null ? ` · 剩余 ${Math.max(remaining, 0)}` : ''
    return `项目 ${count}/${limit}${suffix}`
  }
  if (remaining != null) return `剩余 ${Math.max(remaining, 0)} 个项目位`
  return ''
}

function nodeCapacitySummary(node: ProjectNode): string {
  const hwSummary = clean(node.hardware_summary)
  const slots = nodeProjectSlotsText(node)
  const disk = formatBytes(node.disk_free_bytes ?? 0)
  const diskText = disk ? `磁盘 ${disk}` : '磁盘未知'
  const runtime = node.workspace_provision_ready ? '运行时就绪' : '运行时未就绪'
  const clis = Array.isArray(node.allowed_clis) ? node.allowed_clis.filter(Boolean).join('/') : ''
  const aiText = clis ? `AI ${clis}` : ''
  return [clean(node.capacity_label) || '容量未知', slots, hwSummary, diskText, runtime, aiText]
    .filter(Boolean)
    .join(' · ')
}
