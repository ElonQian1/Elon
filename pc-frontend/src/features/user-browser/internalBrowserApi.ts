import { getDesktopInvoke } from '../shell/desktopShell'
import {
  controlLocalAiWebSession,
  getLocalAiWebSessionState,
  openLocalAiWebSession,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'

export const OPEN_OFFICIAL_AI_TAB_EVENT = 'elon:open-official-ai-tab'
export const OPEN_INTERNAL_BROWSER_LINK_EVENT = 'elon:open-internal-browser-link'

export interface EmbeddedWebviewBounds {
  x: number
  y: number
  width: number
  height: number
}

export interface OfficialAiTabRequest {
  providerId: string
  providerName: string
  ownerKey: string
}

export interface InternalBrowserLinkRequest {
  url: string
  title: string
}

export interface InternalBrowserTabState {
  tabId: 'source'
  title: string
  currentUrl: string
  currentHost: string
  loading: boolean
  visible: boolean
}

export type InternalBrowserControlAction = 'back' | 'forward' | 'reload' | 'show' | 'hide' | 'external' | 'close'

export function requestOfficialAiTab(request: OfficialAiTabRequest) {
  window.dispatchEvent(new CustomEvent<OfficialAiTabRequest>(OPEN_OFFICIAL_AI_TAB_EVENT, { detail: request }))
}

export function openInternalBrowserLink(request: InternalBrowserLinkRequest) {
  const url = safeHttpsUrl(request.url)
  window.dispatchEvent(new CustomEvent<InternalBrowserLinkRequest>(OPEN_INTERNAL_BROWSER_LINK_EVENT, {
    detail: { url, title: cleanTitle(request.title, new URL(url).hostname) },
  }))
}

export async function presentLocalAiWebSessionEmbedded(
  request: OfficialAiTabRequest,
  bounds: EmbeddedWebviewBounds,
): Promise<LocalAiWebSessionState> {
  await openLocalAiWebSession(request.providerId, request.ownerKey, { showWindow: false })
  return invoke<LocalAiWebSessionState>('present_local_ai_web_session_embedded', {
    providerId: request.providerId,
    ownerKey: request.ownerKey,
    bounds: safeBounds(bounds),
  })
}

export async function hideLocalAiWebSessionEmbedded(
  request: OfficialAiTabRequest,
): Promise<LocalAiWebSessionState> {
  return invoke<LocalAiWebSessionState>('hide_local_ai_web_session_embedded', {
    providerId: request.providerId,
    ownerKey: request.ownerKey,
  })
}

export async function openInternalBrowserTab(
  request: InternalBrowserLinkRequest,
  bounds: EmbeddedWebviewBounds,
): Promise<InternalBrowserTabState> {
  return invoke<InternalBrowserTabState>('open_internal_browser_tab', {
    url: safeHttpsUrl(request.url),
    title: cleanTitle(request.title, new URL(request.url).hostname),
    bounds: safeBounds(bounds),
  })
}

export async function resizeInternalBrowserTab(bounds: EmbeddedWebviewBounds): Promise<void> {
  await invoke<void>('resize_internal_browser_tab', { bounds: safeBounds(bounds) })
}

export async function controlInternalBrowserTab(
  action: InternalBrowserControlAction,
): Promise<InternalBrowserTabState | null> {
  return invoke<InternalBrowserTabState | null>('control_internal_browser_tab', { action })
}

export async function getInternalBrowserTabState(): Promise<InternalBrowserTabState> {
  return invoke<InternalBrowserTabState>('get_internal_browser_tab_state')
}

export async function refreshOfficialAiState(request: OfficialAiTabRequest) {
  return getLocalAiWebSessionState(request.providerId, request.ownerKey)
}

export async function controlOfficialAiTab(
  request: OfficialAiTabRequest,
  action: 'back' | 'reload' | 'home' | 'external',
) {
  return controlLocalAiWebSession(request.providerId, request.ownerKey, action)
}

export function boundsFor(element: HTMLElement): EmbeddedWebviewBounds {
  const rect = element.getBoundingClientRect()
  return safeBounds({ x: rect.left, y: rect.top, width: rect.width, height: rect.height })
}

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const desktopInvoke = getDesktopInvoke()
  if (!desktopInvoke) throw new Error('内部网页标签仅在一龙 Windows 客户端中可用。')
  const timeoutMs = 12_000
  let timeoutHandle: ReturnType<typeof setTimeout> | undefined
  try {
    return await Promise.race([
      desktopInvoke<T>(command, args),
      new Promise<never>((_, reject) => {
        timeoutHandle = setTimeout(() => reject(new Error('内部网页标签响应超时，请使用系统浏览器。')), timeoutMs)
      }),
    ])
  } finally {
    if (timeoutHandle) clearTimeout(timeoutHandle)
  }
}

function safeHttpsUrl(value: string) {
  const url = new URL(value)
  if (url.protocol !== 'https:' || url.username || url.password || !url.hostname) {
    throw new Error('只允许在内部标签打开安全 HTTPS 链接。')
  }
  return url.toString()
}

function safeBounds(bounds: EmbeddedWebviewBounds): EmbeddedWebviewBounds {
  const values = [bounds.x, bounds.y, bounds.width, bounds.height]
  if (values.some((value) => !Number.isFinite(value))
    || bounds.x < 0 || bounds.y < 0 || bounds.width < 320 || bounds.height < 220) {
    throw new Error('内部网页标签区域尚未准备好。')
  }
  return bounds
}

function cleanTitle(value: string, fallback: string) {
  const title = value.replace(/[\u0000-\u001f\u007f]/g, '').trim().slice(0, 120)
  return title || fallback
}
