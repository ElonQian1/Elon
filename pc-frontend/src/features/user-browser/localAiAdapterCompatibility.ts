export const LOCAL_AI_REQUIRED_ADAPTER_VERSIONS = {
  // The PC UI is served independently from the native Tauri shell. Keep this
  // pinned to the current native adapter so a hot-loaded UI never drives an
  // older private-stream implementation that cannot recover finance cards,
  // reconcile new-conversation boundaries, or isolate late replies.
  chatgpt: 190,
  'google-ai-mode': 40,
} as const

// The native Tauri host and the PC UI are released independently. Adapter
// versions only describe page injection; this guards native commands and
// bundled shared assets as one desktop runtime generation.
export const LOCAL_AI_REQUIRED_DESKTOP_RUNTIME_VERSION = 5

export function requiredLocalAiAdapterVersion(providerId: string): number {
  return LOCAL_AI_REQUIRED_ADAPTER_VERSIONS[
    providerId as keyof typeof LOCAL_AI_REQUIRED_ADAPTER_VERSIONS
  ] ?? Number.MAX_SAFE_INTEGER
}
