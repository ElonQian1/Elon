export interface LocalAiCapabilityVerificationFallback<T> {
  state: 'ready' | 'upgrade_required'
  providers: T[]
  message: string
}

export function localAiCapabilityVerificationFallback<T>(
  presets: T[],
  upgradeRequired: boolean,
  detail: string,
): LocalAiCapabilityVerificationFallback<T> {
  if (upgradeRequired) {
    return {
      state: 'upgrade_required',
      providers: [],
      message: detail,
    }
  }
  return {
    state: 'ready',
    providers: presets,
    message: detail
      ? `已继续使用 Win 私有能力预设；后台运行时核对暂未完成。${detail}`
      : '已继续使用 Win 私有能力预设；后台运行时核对暂未完成。',
  }
}
