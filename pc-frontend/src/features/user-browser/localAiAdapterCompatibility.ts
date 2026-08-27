export const LOCAL_AI_REQUIRED_ADAPTER_VERSIONS = {
  // The PC UI is served independently from the native Tauri shell. Keep this
  // pinned to the current native adapter so a hot-loaded UI never drives an
  // older private-stream implementation that cannot recover finance cards,
  // reconcile new-conversation boundaries, or isolate late replies.
  chatgpt: 189,
  'google-ai-mode': 40,
} as const

export function requiredLocalAiAdapterVersion(providerId: string): number {
  return LOCAL_AI_REQUIRED_ADAPTER_VERSIONS[
    providerId as keyof typeof LOCAL_AI_REQUIRED_ADAPTER_VERSIONS
  ] ?? Number.MAX_SAFE_INTEGER
}
