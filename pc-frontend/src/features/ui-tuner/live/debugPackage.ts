export const LIVE_DEBUG_SUFFIX = '.uituner'

const SCOPED_DEBUG_SUFFIX = /^\.uituner_[a-f0-9]{8}$/

export function liveDebugSuffix(packageName: string | null | undefined) {
  const value = packageName?.trim() ?? ''
  const index = value.lastIndexOf(LIVE_DEBUG_SUFFIX)
  if (index < 0) return ''
  const suffix = value.slice(index)
  return suffix === LIVE_DEBUG_SUFFIX || SCOPED_DEBUG_SUFFIX.test(suffix) ? suffix : ''
}

export function isLiveDebugPackage(packageName: string | null | undefined) {
  return Boolean(liveDebugSuffix(packageName))
}

export function liveDebugBasePackage(packageName: string) {
  const suffix = liveDebugSuffix(packageName)
  return suffix ? packageName.slice(0, -suffix.length) : packageName
}
