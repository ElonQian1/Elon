import type {
  LocalAiAdapterAction,
  LocalAiPrivateTransportHealth,
  LocalAiWebProvider,
} from './localAiBrowserApi'

export interface LocalAiPrivateTransportCapability {
  id: string
  label: string
  requestMode: string
  fallback: string
  runtimeEnabled: boolean
  activation: 'preset_then_background_verify'
}

interface CapabilityDefinition {
  id: string
  label: string
  requestMode: string
  fallback: string
  requiredActions?: LocalAiAdapterAction[]
}

const SHARED_CONTINUITY: CapabilityDefinition = {
  id: 'win_web_ai_background_navigation_continuity_v1',
  label: '后台导航连续性',
  requestMode: 'preserve_inflight_official_navigation',
  fallback: 'official_webview_bounded_recovery',
}

const SHARED_REFRESH_SINGLE_FLIGHT: CapabilityDefinition = {
  id: 'win_web_ai_snapshot_refresh_single_flight_v1',
  label: '官网快照单飞行刷新',
  requestMode: 'coalesced_official_snapshot_poll',
  fallback: 'official_webview_bounded_recovery',
  requiredActions: ['snapshot'],
}

const SHARED_SEND_COORDINATOR: CapabilityDefinition = {
  id: 'win_web_ai_unified_send_coordinator_v1',
  label: '稳定回执与不确定发送对账',
  requestMode: 'stable_request_id_single_flight_official_page_transport',
  fallback: 'official_page_reconciliation_without_automatic_write_replay',
  requiredActions: ['send_prompt'],
}

const SHARED_CONVERSATION_BODY_CACHE: CapabilityDefinition = {
  id: 'win_web_ai_per_conversation_rich_snapshot_cache_v1',
  label: '独立会话正文与富内容缓存',
  requestMode: 'dpapi_per_conversation_cache_first',
  fallback: 'official_webview_navigation',
  requiredActions: ['open_conversation'],
}

const SHARED_NATIVE_STREAM_REFRESH: CapabilityDefinition = {
  id: 'win_web_ai_native_stream_refresh_v1',
  label: '私有流原生事件即时刷新',
  requestMode: 'coalesced_native_session_update_event',
  fallback: 'bounded_session_and_dom_watchdog',
}

const CATALOG: Record<string, CapabilityDefinition[]> = {
  chatgpt: [
    {
      id: 'win_chatgpt_private_conversation_project_directory_v1',
      label: '私有会话与项目目录观察',
      requestMode: 'passive_official_response_observer',
      fallback: 'official_dom_directory',
      requiredActions: ['list_conversations', 'open_conversation'],
    },
    {
      id: 'win_chatgpt_private_conversation_prefetch_v1',
      label: '当前会话同源预取',
      requestMode: 'authenticated_same_origin_get',
      fallback: 'official_webview_navigation',
      requiredActions: ['open_conversation'],
    },
    {
      id: 'win_chatgpt_guest_private_conversation_refresh_v1',
      label: '游客会话富内容补齐',
      requestMode: 'observed_same_origin_guest_get_with_bounded_endpoint_fallback',
      fallback: 'official_stream_and_dom_snapshot',
      requiredActions: ['open_conversation', 'snapshot'],
    },
    {
      id: 'win_chatgpt_private_send_dispatch_observer_v1',
      label: '发送请求被动确认',
      requestMode: 'passive_official_request_observer',
      fallback: 'official_dom_send_confirmation',
      requiredActions: ['send_prompt'],
    },
    {
      id: 'win_chatgpt_private_stream_observer_v1',
      label: '私有流与完成态结算',
      requestMode: 'passive_official_response_clone',
      fallback: 'official_dom_stream_snapshot',
      requiredActions: ['snapshot', 'stop_generation'],
    },
    {
      id: 'win_chatgpt_realtime_voice_private_transcript_refresh_v1',
      label: '语音结束后转写单飞行刷新',
      requestMode: 'serial_authenticated_same_origin_get_then_snapshot',
      fallback: 'official_dom_snapshot',
      requiredActions: ['invoke_ui_control'],
    },
    SHARED_CONTINUITY,
    SHARED_REFRESH_SINGLE_FLIGHT,
    SHARED_SEND_COORDINATOR,
    SHARED_CONVERSATION_BODY_CACHE,
    SHARED_NATIVE_STREAM_REFRESH,
  ],
  'google-ai-mode': [
    {
      id: 'win_google_private_conversation_directory_v1',
      label: '私有会话目录观察',
      requestMode: 'passive_official_response_observer',
      fallback: 'local_directory_cache_and_official_page',
      requiredActions: ['list_conversations', 'open_conversation'],
    },
    {
      id: 'win_google_conversation_snapshot_cache_v1',
      label: '会话快照缓存先显',
      requestMode: 'passive_official_snapshot_cache',
      fallback: 'official_webview_navigation',
      requiredActions: ['open_conversation'],
    },
    {
      id: 'win_google_private_reply_observer_v1',
      label: '回复流完成态观察',
      requestMode: 'passive_completion_signal',
      fallback: 'official_dom_reply_snapshot',
      requiredActions: ['snapshot', 'send_prompt'],
    },
    SHARED_CONTINUITY,
    SHARED_REFRESH_SINGLE_FLIGHT,
    SHARED_SEND_COORDINATOR,
    SHARED_CONVERSATION_BODY_CACHE,
    SHARED_NATIVE_STREAM_REFRESH,
  ],
}

export function localAiPrivateTransportCapabilities(
  provider: LocalAiWebProvider | undefined,
): LocalAiPrivateTransportCapability[] {
  if (!provider) return []
  const actions = new Set(provider.adapterActions)
  return (CATALOG[provider.id] ?? []).map((definition) => ({
    id: definition.id,
    label: definition.label,
    requestMode: definition.requestMode,
    fallback: definition.fallback,
    runtimeEnabled: (definition.requiredActions ?? []).every((action) => actions.has(action)),
    activation: 'preset_then_background_verify',
  }))
}

export function localAiPrivateTransportStatusCopy(
  provider: LocalAiWebProvider | undefined,
  health?: LocalAiPrivateTransportHealth,
  nowMs = Date.now(),
): { copy: string; detail: string } | null {
  const capabilities = localAiPrivateTransportCapabilities(provider)
  if (!capabilities.length) return null
  const enabled = capabilities.filter((capability) => capability.runtimeEnabled)
  const detail = enabled.map((capability) => capability.label).join('、')
  if (provider?.id === 'chatgpt' && health?.version === 1
      && health.sampledAtMs > 0
      && health.sampledAtMs <= nowMs + 5_000
      && nowMs - health.sampledAtMs <= 2 * 60 * 1000) {
    if (health.cooldownRemainingMs > 0) {
      return {
        copy: `私有会话预取正在冷却（约 ${Math.ceil(health.cooldownRemainingMs / 1000)} 秒，${privateOutcomeCopy(health.lastOutcome)}）；当前立即回退官网，不阻塞聊天。`,
        detail,
      }
    }
    if (health.prefetchReady) {
      const latency = health.privateLatencyMs > 0 ? `，最近约 ${health.privateLatencyMs}ms` : ''
      return {
        copy: `私有会话预取实时可用${latency}；已成功 ${health.successes} 次，失败自动回退官网。`,
        detail,
      }
    }
    if (health.prefetchEnabled) {
      return {
        copy: '私有会话预取已启用，正在后台等待官网会话上下文；当前使用缓存或官网语义，不阻塞输入。',
        detail,
      }
    }
  }
  return {
    copy: `私有加速预设 ${enabled.length}/${capabilities.length} 项已载入；无需等待官网扫描，运行健康在后台异步核验。`,
    detail,
  }
}

function privateOutcomeCopy(outcome: LocalAiPrivateTransportHealth['lastOutcome']): string {
  if (outcome === 'timeout') return '上次请求超时'
  if (outcome === 'auth') return '官网登录状态待恢复'
  if (outcome === 'context') return '官网请求上下文待恢复'
  if (outcome === 'parse' || outcome === 'empty') return '官网响应结构待适配'
  if (outcome === 'http' || outcome === 'network' || outcome === 'official_error') return '官网网络暂不可用'
  return '自动保护'
}
