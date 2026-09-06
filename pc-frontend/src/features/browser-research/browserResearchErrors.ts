/** These are the node receipt contract's public codes, never arbitrary native messages. */
export const RESEARCH_FAILURE_CODES = [
  'operation_failed', 'host_unavailable', 'invalid_command', 'invalid_scope',
  'session_not_found', 'session_expired', 'resource_not_found', 'request_not_found',
  'resource_unavailable', 'credentials_forbidden', 'limit_exceeded', 'unsupported',
  'site_not_found', 'navigation_blocked', 'result_too_large',
] as const
export type ResearchFailureCode = typeof RESEARCH_FAILURE_CODES[number]

const nativeCodes: Readonly<Record<string, ResearchFailureCode>> = {
  browser_research_host_dispatch_failed: 'host_unavailable',
  host_cdp_dispatch_failed: 'host_unavailable',
  browser_research_profile_unavailable: 'host_unavailable',
  browser_research_window_unavailable: 'host_unavailable',
  host_core_unavailable: 'host_unavailable',
  host_session_unavailable: 'host_unavailable',
  host_subscription_unavailable: 'host_unavailable',
  research_host_unavailable: 'host_unavailable',
  research_worker_unavailable: 'host_unavailable',
  research_unavailable: 'host_unavailable',
  browser_research_host_unsupported: 'unsupported',
  unsupported_research_action: 'unsupported',
  research_main_window_required: 'invalid_scope',
  invalid_research_identity: 'invalid_scope',
  session_scope_mismatch: 'invalid_scope',
  site_scope_changed: 'invalid_scope',
  research_session_expired: 'session_expired',
  browser_research_host_config_invalid: 'invalid_command',
  session_required: 'invalid_command',
  invalid_site_manifest: 'invalid_command',
  invalid_site_entry: 'invalid_command',
  invalid_site_origin: 'invalid_command',
  invalid_content_offset: 'invalid_command',
  invalid_search_query: 'invalid_command',
  stored_item_unavailable: 'resource_unavailable',
  resource_integrity_changed: 'resource_unavailable',
  body_too_large: 'limit_exceeded',
  metadata_limit: 'limit_exceeded',
  site_limit: 'limit_exceeded',
  research_window_limit: 'limit_exceeded',
  research_result_too_large: 'result_too_large',
}

export function receiptErrorCode(value: unknown): ResearchFailureCode {
  return typeof value === 'string' && RESEARCH_FAILURE_CODES.includes(value as ResearchFailureCode)
    ? value as ResearchFailureCode : 'operation_failed'
}
export function nativeResearchErrorCode(error: unknown): ResearchFailureCode {
  const code = error instanceof Error ? error.message : error
  if (typeof code !== 'string') return 'operation_failed'
  // Exact names only: a sentence containing a code may also contain private data.
  return Object.prototype.hasOwnProperty.call(nativeCodes, code) ? nativeCodes[code] : receiptErrorCode(code)
}

export const researchFailureLabels: Record<ResearchFailureCode, string> = {
  operation_failed: '操作未完成，请检查研究会话状态后重试。',
  host_unavailable: 'Windows 研究宿主未就绪或未能响应。请确认新版客户端正在运行后重试。',
  invalid_command: '研究参数或读取范围无效，请重新选择后重试。',
  invalid_scope: '当前账号、项目或站点范围不匹配，请重新选择研究会话。',
  session_not_found: '研究会话不存在或已被移除，请刷新会话列表。',
  session_expired: '研究会话已过期，请重新打开官网建立新会话。',
  resource_not_found: '所选资源已不存在，请刷新资源列表。',
  request_not_found: '所选请求已不存在，请刷新请求列表。',
  resource_unavailable: '资源无法读取或完整性校验未通过，请重新采集。',
  credentials_forbidden: '资料包含不能返回的登录凭据，此次读取已停止。',
  limit_exceeded: '已达到研究容量或读取上限，请缩小范围。',
  unsupported: '当前客户端不支持此研究操作，请检查 Windows 客户端版本。',
  site_not_found: '站点配置不存在，请刷新站点列表。',
  navigation_blocked: '目标页面不在该站点允许的访问范围内。',
  result_too_large: '返回资料超过读取上限，请缩小范围或分页读取。',
}
