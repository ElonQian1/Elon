import { createLocalAiRuntimeToken } from './localAiCommandReceipt'
import {
  isLocalAiBrowserAvailable,
  resolveNativeLocalAiGuestOwnerIdentity,
} from './localAiBrowserApi'

const GUEST_OWNER_STORAGE_KEY = 'elon_auth_client_instance_id'
const DEVICE_OWNER_PREFIX = 'anonymous-device:'

export function readBrowserLocalAiGuestOwnerKey(): string {
  try {
    const existing = window.localStorage.getItem(GUEST_OWNER_STORAGE_KEY)?.trim()
    if (existing) return `${DEVICE_OWNER_PREFIX}${existing}`
    const created = `pc:${createGuestOwnerToken()}`
    window.localStorage.setItem(GUEST_OWNER_STORAGE_KEY, created)
    return `${DEVICE_OWNER_PREFIX}${created}`
  } catch {
    return `anonymous-session:${createGuestOwnerToken()}`
  }
}

export async function resolveLocalAiGuestIdentity(browserOwnerKey: string): Promise<string> {
  if (!isLocalAiBrowserAvailable()) return browserOwnerKey
  const legacyOwnerKey = browserOwnerKey.startsWith(DEVICE_OWNER_PREFIX)
    ? browserOwnerKey
    : undefined
  try {
    const identity = await resolveNativeLocalAiGuestOwnerIdentity(legacyOwnerKey)
    rememberBrowserOwnerKey(identity.ownerKey)
    return identity.ownerKey
  } catch {
    // Older desktop builds keep the browser identity until the native command is installed.
    return browserOwnerKey
  }
}

function rememberBrowserOwnerKey(ownerKey: string): void {
  if (!ownerKey.startsWith(DEVICE_OWNER_PREFIX)) return
  try {
    window.localStorage.setItem(
      GUEST_OWNER_STORAGE_KEY,
      ownerKey.slice(DEVICE_OWNER_PREFIX.length),
    )
  } catch {
    // The native file remains authoritative when browser storage is unavailable.
  }
}

function createGuestOwnerToken(): string {
  try {
    const nativeId = globalThis.crypto?.randomUUID?.()
    if (nativeId) return nativeId
  } catch {
    // Public HTTP and older WebView2 runtimes may not expose a usable randomUUID.
  }
  return createLocalAiRuntimeToken()
}
