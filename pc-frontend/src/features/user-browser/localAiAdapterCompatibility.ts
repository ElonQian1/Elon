export const LOCAL_AI_REQUIRED_ADAPTER_VERSIONS = {
  chatgpt: 180,
  'google-ai-mode': 40,
} as const

export function requiredLocalAiAdapterVersion(providerId: string): number {
  return LOCAL_AI_REQUIRED_ADAPTER_VERSIONS[
    providerId as keyof typeof LOCAL_AI_REQUIRED_ADAPTER_VERSIONS
  ] ?? Number.MAX_SAFE_INTEGER
}
