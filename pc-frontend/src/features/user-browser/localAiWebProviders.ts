import type { LocalAiWebProvider } from './localAiBrowserApi'
import { requiredLocalAiAdapterVersion } from './localAiAdapterCompatibility'

export const LOCAL_AI_PROVIDER_FALLBACKS: Record<string, LocalAiWebProvider> = {
  'google-ai-mode': {
    id: 'google-ai-mode',
    displayName: 'Google AI 模式',
    startHost: 'google.com/aimode',
    loginMode: 'guest_web_system_login',
    profileScope: 'local_owner_provider',
    rendererProtocol: 'yilong.ai.ui.v1',
    rendererStatus: 'active',
    researchCaptureStatus: 'local_raw_prelaunch',
    researchCaptureRetentionDays: 30,
    adapterVersion: requiredLocalAiAdapterVersion('google-ai-mode'),
    adapterActions: [
      'snapshot',
      'send_prompt',
      'stop_generation',
      'new_conversation',
      'list_conversations',
      'open_conversation',
    ],
  },
  chatgpt: {
    id: 'chatgpt',
    displayName: 'ChatGPT',
    startHost: 'chatgpt.com',
    loginMode: 'manual_web',
    profileScope: 'local_owner_provider',
    rendererProtocol: 'yilong.ai.ui.v1',
    rendererStatus: 'active',
    researchCaptureStatus: 'local_raw_prelaunch',
    researchCaptureRetentionDays: 30,
    adapterVersion: requiredLocalAiAdapterVersion('chatgpt'),
    adapterActions: [
      'snapshot',
      'send_prompt',
      'stop_generation',
      'regenerate_response',
      'new_conversation',
      'list_conversations',
      'open_conversation',
      'open_project',
      'start_google_login',
      'list_model_options',
      'list_composer_tools',
      'collect_model_options',
      'collect_composer_tools',
      'select_model_option',
      'select_composer_tool',
      'request_attachment_upload',
      'open_model_selector',
      'open_composer_tools',
      'start_dictation',
      'cancel_dictation',
      'submit_dictation',
      'remove_attachment',
      'dismiss_composer_menu',
      'list_navigation',
      'collect_navigation',
      'select_navigation',
      'dismiss_navigation',
      'snapshot_ui_manifest',
      'invoke_ui_control',
    ],
  },
}

export const DEFAULT_LOCAL_AI_PROVIDER_ID = 'chatgpt'

export function localAiWebProviderPresets(): LocalAiWebProvider[] {
  return Object.values(LOCAL_AI_PROVIDER_FALLBACKS).map((provider) => ({
    ...provider,
    adapterActions: [...provider.adapterActions],
  }))
}
