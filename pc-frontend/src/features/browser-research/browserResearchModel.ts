import { RESEARCH_KINDS, RESULT_SCHEMA } from './types'
import type { ResearchAction, ResearchCommand, ResearchKind, ResearchResult } from './types'
import { researchFailureLabels, type ResearchFailureCode } from './browserResearchErrors'

export class ResearchError extends Error {
  constructor(public readonly code: 'invalid_response' | 'timeout' | 'cancelled' | ResearchFailureCode) {
    super(code)
  }
}
export function researchErrorMessage(error: unknown): string {
  if (!(error instanceof ResearchError)) return '操作未完成，请检查本机服务后重试。'
  return {
    ...researchFailureLabels,
    invalid_response: '返回资料格式不兼容，未显示为成功。请更新 Windows 客户端。',
    timeout: '等待回执超时，操作可能仍在执行。请刷新状态，避免重复打开。',
    cancelled: '已停止等待。已经开始的宿主操作可能仍在执行。',
  }[error.code]
}
export function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}
const string = (value: unknown, max = 4096): value is string => typeof value === 'string' && value.length <= max
const identifier = (value: unknown): value is string => string(value, 256) && value.length > 0 && !/[\u0000-\u001f]/.test(value)
const integer = (value: unknown): value is number => Number.isSafeInteger(value) && (value as number) >= 0
const offset = (value: unknown) => value === null || integer(value)
const strings = (value: unknown, max = 128) => Array.isArray(value) && value.length <= max && value.every((item) => string(item))
export function boundedJson(value: unknown, max = 65536): boolean {
  try { return new TextEncoder().encode(JSON.stringify(value)).byteLength <= max } catch { return false }
}
function site(value: unknown): boolean {
  return record(value) && value.schema === 'yilong.browser-research.site.v1'
    && identifier(value.id) && string(value.name, 256) && string(value.entry_url)
    && ['navigation_origins', 'resource_origins', 'api_origins', 'identity_origins'].every((key) => strings(value[key], 64))
}
function session(value: unknown): boolean {
  return record(value) && identifier(value.id) && identifier(value.site_id) && typeof value.active === 'boolean'
    && ['generation', 'expires_at_ms', 'resource_count', 'request_count'].every((key) => integer(value[key]))
    && string(value.phase, 64) && strings(value.gaps) && value.trading_enabled === false
}
function resource(value: unknown): boolean {
  return record(value) && identifier(value.id) && string(value.url) && string(value.resource_type, 128)
    && string(value.mime, 256) && integer(value.size_bytes) && string(value.sha256, 64)
    && /^[a-f0-9]{64}$/.test(value.sha256) && integer(value.generation)
    && typeof value.truncated === 'boolean' && typeof value.redacted === 'boolean'
}
function request(value: unknown): boolean {
  return record(value) && identifier(value.id) && string(value.url) && string(value.method, 32)
    && (value.status === null || (integer(value.status) && value.status <= 599)) && integer(value.generation)
    && ['request_resource_id', 'response_resource_id'].every((key) => value[key] == null || identifier(value[key]))
}
function slice(value: unknown): boolean {
  return record(value) && string(value.content, 65536) && integer(value.offset)
    && offset(value.next_offset) && typeof value.complete === 'boolean'
    && (value.next_offset === null || (value.next_offset as number) > value.offset)
}
function search(value: unknown): boolean {
  return record(value) && identifier(value.resource_id) && string(value.url)
    && integer(value.offset) && string(value.excerpt, 8192)
}
export function parseResearchResult(value: unknown, expected: ResearchCommand): ResearchResult {
  if (!boundedJson(value) || !record(value) || value.schema !== RESULT_SCHEMA || value.kind !== expected.kind) {
    throw new ResearchError('invalid_response')
  }
  let valid = false
  const lists: Partial<Record<ResearchKind, (item: unknown) => boolean>> = { sites: site, register_site: site, sessions: session, resources: resource, requests: request, search }
  const itemCheck = lists[expected.kind]
  if (itemCheck) {
    valid = Array.isArray(value.items) && value.items.length <= 100 && value.items.every(itemCheck)
      && integer(value.total) && integer(value.offset) && offset(value.next_offset)
      && (value.next_offset === null || (value.next_offset as number) > value.offset)
      && (value.partial === undefined || typeof value.partial === 'boolean')
  } else if (['open', 'status', 'pause', 'resume'].includes(expected.kind)) {
    valid = session(value.session) && record(value.session)
      && (!expected.session_id || value.session.id === expected.session_id)
      && (!expected.site_id || value.session.site_id === expected.site_id)
  } else if (expected.kind === 'read_resource') {
    valid = resource(value.item) && record(value.item) && value.item.id === expected.resource_id && slice(value)
  } else if (expected.kind === 'read_request') {
    valid = request(value.request) && record(value.request) && value.request.id === expected.request_id
      && (value.request_body === null || slice(value.request_body)) && (value.response_body === null || slice(value.response_body))
  }
  if (!valid) throw new ResearchError('invalid_response')
  return value as unknown as ResearchResult
}
export function parseResearchAction(value: unknown): ResearchAction {
  if (!record(value) || !identifier(value.action_id) || !string(value.project_key, 64)
    || !/^[a-f0-9]{64}$/.test(value.project_key) || !record(value.command)
    || !RESEARCH_KINDS.includes(value.command.kind as ResearchKind) || !boundedJson(value.command, 16384)
    || !integer(value.requested_at_ms) || !integer(value.expires_at_ms) || !string(value.status, 64)) {
    throw new ResearchError('invalid_response')
  }
  const fields = ['kind', 'site_id', 'session_id', 'resource_id', 'request_id', 'query', 'offset', 'limit', 'manifest']
  if (Object.keys(value.command).some((key) => !fields.includes(key))) throw new ResearchError('invalid_response')
  for (const key of ['site_id', 'session_id', 'resource_id', 'request_id']) {
    if (value.command[key] !== undefined && !identifier(value.command[key])) throw new ResearchError('invalid_response')
  }
  for (const key of ['offset', 'limit']) {
    if (value.command[key] !== undefined && !integer(value.command[key])) throw new ResearchError('invalid_response')
  }
  if (value.command.query !== undefined && !string(value.command.query, 512)) throw new ResearchError('invalid_response')
  if (value.command.manifest !== undefined && !site(value.command.manifest)) throw new ResearchError('invalid_response')
  return value as unknown as ResearchAction
}

/** Invalidating a view cannot cancel native work; it prevents its late result entering another scope. */
export function createResearchEpoch() {
  let generation = 0
  return { next: () => ++generation, current: (ticket: number) => ticket === generation }
}
