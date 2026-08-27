import type { LocalAiAdapterAction, LocalAiWebProvider } from './localAiBrowserApi'

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
  label: '缓存先接与官网发送确认单飞行',
  requestMode: 'cache_first_single_flight_official_page_transport',
  fallback: 'official_page_confirmation_and_draft_recovery',
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
): { copy: string; detail: string } | null {
  const capabilities = localAiPrivateTransportCapabilities(provider)
  if (!capabilities.length) return null
  const enabled = capabilities.filter((capability) => capability.runtimeEnabled)
  return {
    copy: `私有加速预设 ${enabled.length}/${capabilities.length} 项已载入；无需等待官网扫描，运行健康在后台异步核验。`,
    detail: enabled.map((capability) => capability.label).join('、'),
  }
}
