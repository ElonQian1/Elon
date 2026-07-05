export const APP_UPDATE_BEFORE_RELOAD_EVENT = 'elon:app-update:before-reload'

const CURRENT_VERSION_KEY = 'elon.pc.appUpdate.currentVersion.v1'
const RELOAD_NOTICE_KEY = 'elon.pc.appUpdate.reloadNotice.v1'
const MAX_NOTICE_AGE_MS = 10 * 60 * 1000

export interface ServerVersionInfo {
  service?: string
  status?: string
  versionName?: string
  gitSha?: string
}

export interface AppUpdateReloadNotice {
  from: ServerVersionInfo | null
  to: ServerVersionInfo
  path: string
  createdAt: number
}

export function versionIdentity(version: ServerVersionInfo | null | undefined): string {
  if (!version) return ''
  return [version.versionName?.trim(), version.gitSha?.trim()].filter(Boolean).join('@')
}

export function versionLabel(version: ServerVersionInfo | null | undefined): string {
  if (!version) return '新版'
  const name = version.versionName?.trim()
  const sha = version.gitSha?.trim()
  if (name && sha) return `${name} · ${sha.slice(0, 8)}`
  return name || (sha ? sha.slice(0, 8) : '新版')
}

export function readStoredVersion(): ServerVersionInfo | null {
  const raw = safeLocalStorage()?.getItem(CURRENT_VERSION_KEY)
  return parseVersion(raw)
}

export function writeStoredVersion(version: ServerVersionInfo) {
  try {
    safeLocalStorage()?.setItem(CURRENT_VERSION_KEY, JSON.stringify(version))
  } catch {
    // Best effort only.
  }
}

export function saveReloadNotice(notice: AppUpdateReloadNotice) {
  try {
    safeSessionStorage()?.setItem(RELOAD_NOTICE_KEY, JSON.stringify(notice))
  } catch {
    // Best effort only.
  }
}

export function consumeReloadNotice(): AppUpdateReloadNotice | null {
  const storage = safeSessionStorage()
  const raw = storage?.getItem(RELOAD_NOTICE_KEY)
  if (!storage || !raw) return null
  storage.removeItem(RELOAD_NOTICE_KEY)
  try {
    const notice = JSON.parse(raw) as AppUpdateReloadNotice
    if (!notice?.to || Date.now() - Number(notice.createdAt || 0) > MAX_NOTICE_AGE_MS) return null
    return notice
  } catch {
    return null
  }
}

export function dispatchBeforeAppUpdateReload(notice: AppUpdateReloadNotice) {
  window.dispatchEvent(new CustomEvent(APP_UPDATE_BEFORE_RELOAD_EVENT, { detail: notice }))
}

function parseVersion(raw: string | null | undefined): ServerVersionInfo | null {
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as ServerVersionInfo
    return versionIdentity(parsed) ? parsed : null
  } catch {
    return null
  }
}

function safeLocalStorage(): Storage | null {
  if (typeof window === 'undefined') return null
  try {
    return window.localStorage
  } catch {
    return null
  }
}

function safeSessionStorage(): Storage | null {
  if (typeof window === 'undefined') return null
  try {
    return window.sessionStorage
  } catch {
    return null
  }
}
