export const LIVE_DEBUG_SUFFIX = '.uituner'

const SCOPED_DEBUG_SUFFIX = /^\.uituner_[a-f0-9]{8}$/
const COMPAT_DEBUG_SUFFIX = /\.(?:uituner|uitest_anim|uitest)(?:_[A-Za-z0-9_]+)?$/

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
  return packageName.trim().replace(COMPAT_DEBUG_SUFFIX, '')
}

export function liveDebugCandidate(projectId: string | null | undefined, deviceIdentity: string) {
  const owner = `pc-ui:${projectId || 'local'}`
  return { ready: true as const, sourceSessionId: `${owner}:${deviceIdentity}`, previewOwner: owner }
}
